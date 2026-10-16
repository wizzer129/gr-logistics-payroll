//! The email outbox and senders (EMAIL-SPEC.md). Runs against real Postgres
//! (`#[sqlx::test]`); `ResendSender` runs against a local stub server, never the real API.

mod common;

use std::{
	sync::{Arc, Mutex},
	time::Duration,
};

use axum::{
	Json, Router,
	http::{HeaderMap, StatusCode},
	routing::post,
};
use gr_logistics::email::{
	self, Locale, Message,
	outbox::{MAX_ATTEMPTS, Outbox},
	render::Renderer,
	sender::{
		Attachment, EmailSender, LogSender, MemorySender, OutgoingEmail, ResendSender, SendError,
	},
};
use serde_json::Value;
use sqlx::PgPool;
use tokio::sync::watch;
use uuid::Uuid;

async fn migrated(pool: &PgPool) {
	let mut conn = pool.acquire().await.unwrap();
	gr_logistics::cli::migrate::apply(&mut conn).await.unwrap();
}

fn test_message() -> Message {
	Message::TestEmail {
		sent_at: "2026-10-16 09:00 UTC".into(),
	}
}

async fn enqueue(pool: &PgPool, to: &str, locale: Locale) -> Uuid {
	let mut tx = pool.begin().await.unwrap();
	let id = email::enqueue(&mut tx, &test_message(), to, locale, None)
		.await
		.unwrap();
	tx.commit().await.unwrap();
	id
}

fn outbox(pool: &PgPool, sender: &Arc<MemorySender>) -> Outbox {
	Outbox::new(
		pool.clone(),
		sender.clone() as Arc<dyn EmailSender>,
		Arc::new(Renderer::new()),
	)
}

#[derive(sqlx::FromRow, Debug)]
struct Row {
	status: String,
	attempts: i32,
	last_error: Option<String>,
	subject: Option<String>,
	payload: Option<Value>,
	due_now: bool,
}

async fn row(pool: &PgPool, id: Uuid) -> Row {
	sqlx::query_as(
		"SELECT status, attempts, last_error, subject, payload,
			next_attempt_at <= now() AS due_now
		 FROM email_outbox WHERE id = $1",
	)
	.bind(id)
	.fetch_one(pool)
	.await
	.unwrap()
}

#[sqlx::test(migrations = false)]
async fn a_queued_email_is_rendered_and_sent(pool: PgPool) {
	migrated(&pool).await;
	let sender = Arc::new(MemorySender::default());
	let id = enqueue(&pool, "ana@example.com", Locale::Es).await;

	assert_eq!(outbox(&pool, &sender).process_due().await.unwrap(), 1);

	let sent = sender.sent();
	assert_eq!(sent.len(), 1);
	assert_eq!(sent[0].id, Some(id), "the row id is the idempotency key");
	assert_eq!(sent[0].to, "ana@example.com");
	assert_eq!(sent[0].subject, "Correo de prueba de GR Logistics");
	assert!(sent[0].html.contains("El correo funciona"));
	assert!(sent[0].text.contains("2026-10-16 09:00 UTC"));

	let row = row(&pool, id).await;
	assert_eq!(row.status, "sent");
	assert_eq!(
		row.subject.as_deref(),
		Some("Correo de prueba de GR Logistics")
	);
	assert_eq!(row.attempts, 1);
	assert!(row.payload.is_some(), "only password resets are scrubbed");

	assert_eq!(
		outbox(&pool, &sender).process_due().await.unwrap(),
		0,
		"a sent email isn't sent again"
	);
}

#[sqlx::test(migrations = false)]
async fn a_rolled_back_enqueue_sends_nothing(pool: PgPool) {
	migrated(&pool).await;
	let sender = Arc::new(MemorySender::default());

	let mut tx = pool.begin().await.unwrap();
	email::enqueue(
		&mut tx,
		&test_message(),
		"ana@example.com",
		Locale::En,
		None,
	)
	.await
	.unwrap();
	tx.rollback().await.unwrap();

	assert_eq!(outbox(&pool, &sender).process_due().await.unwrap(), 0);
	assert!(sender.sent().is_empty());
}

#[sqlx::test(migrations = false)]
async fn a_retryable_failure_backs_off_then_gives_up(pool: PgPool) {
	migrated(&pool).await;
	let sender = Arc::new(MemorySender::default());
	*sender.fail_with.lock().unwrap() = Some(SendError::Retryable("503 from provider".into()));
	let id = enqueue(&pool, "ana@example.com", Locale::En).await;

	outbox(&pool, &sender).process_due().await.unwrap();
	let after_first = row(&pool, id).await;
	assert_eq!(after_first.status, "pending");
	assert_eq!(after_first.attempts, 1);
	assert!(!after_first.due_now, "retry is scheduled in the future");
	assert_eq!(after_first.last_error.as_deref(), Some("503 from provider"));

	// Fast-forward through the remaining attempts.
	for _ in 1..MAX_ATTEMPTS {
		sqlx::query("UPDATE email_outbox SET next_attempt_at = now() WHERE id = $1")
			.bind(id)
			.execute(&pool)
			.await
			.unwrap();
		outbox(&pool, &sender).process_due().await.unwrap();
	}

	let last = row(&pool, id).await;
	assert_eq!(last.status, "failed");
	assert_eq!(last.attempts, MAX_ATTEMPTS);
	assert!(last.last_error.unwrap().contains("gave up"));
}

#[sqlx::test(migrations = false)]
async fn a_permanent_failure_fails_immediately(pool: PgPool) {
	migrated(&pool).await;
	let sender = Arc::new(MemorySender::default());
	*sender.fail_with.lock().unwrap() = Some(SendError::Permanent("422 invalid to".into()));
	let id = enqueue(&pool, "not-an-address", Locale::En).await;

	outbox(&pool, &sender).process_due().await.unwrap();

	let row = row(&pool, id).await;
	assert_eq!(row.status, "failed");
	assert_eq!(row.attempts, 1);
	assert_eq!(row.last_error.as_deref(), Some("422 invalid to"));
}

#[sqlx::test(migrations = false)]
async fn an_unreadable_payload_fails_without_sending(pool: PgPool) {
	migrated(&pool).await;
	let sender = Arc::new(MemorySender::default());
	let id: Uuid = sqlx::query_scalar(
		"INSERT INTO email_outbox (kind, recipient, locale, payload)
		 VALUES ('test_email', 'ana@example.com', 'en', '{\"kind\": \"nope\"}')
		 RETURNING id",
	)
	.fetch_one(&pool)
	.await
	.unwrap();

	outbox(&pool, &sender).process_due().await.unwrap();

	assert!(sender.sent().is_empty());
	let row = row(&pool, id).await;
	assert_eq!(row.status, "failed");
	assert!(row.last_error.unwrap().contains("unreadable payload"));
}

/// The real loop: a commit wakes it through `pg_notify` (well before the 30s poll), and the
/// shutdown signal stops it.
#[sqlx::test(migrations = false)]
async fn the_worker_wakes_on_commit_and_stops_on_shutdown(pool: PgPool) {
	migrated(&pool).await;
	let sender = Arc::new(MemorySender::default());
	let (shutdown_tx, shutdown_rx) = watch::channel(false);
	let worker = tokio::spawn(outbox(&pool, &sender).run(shutdown_rx));

	// Let it start listening before the notification goes out.
	tokio::time::sleep(Duration::from_millis(300)).await;
	enqueue(&pool, "ana@example.com", Locale::En).await;

	tokio::time::timeout(Duration::from_secs(10), async {
		while sender.sent().is_empty() {
			tokio::time::sleep(Duration::from_millis(50)).await;
		}
	})
	.await
	.expect("the commit notification should wake the worker");

	shutdown_tx.send(true).unwrap();
	tokio::time::timeout(Duration::from_secs(5), worker)
		.await
		.expect("the worker should stop on shutdown")
		.unwrap();
}

// --- ResendSender against a stub ---

#[derive(Default)]
struct Captured {
	headers: Option<HeaderMap>,
	body: Option<Value>,
}

/// Serves `POST /emails`, answering `status` and recording the request.
async fn stub_resend(status: StatusCode) -> (String, Arc<Mutex<Captured>>) {
	let captured = Arc::new(Mutex::new(Captured::default()));
	let state = captured.clone();
	let app = Router::new().route(
		"/emails",
		post(move |headers: HeaderMap, Json(body): Json<Value>| {
			let state = state.clone();
			async move {
				let mut captured = state.lock().unwrap();
				captured.headers = Some(headers);
				captured.body = Some(body);
				(status, Json(serde_json::json!({ "id": "stub" })))
			}
		}),
	);
	let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
	let url = format!("http://{}", listener.local_addr().unwrap());
	tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
	(url, captured)
}

fn outgoing() -> OutgoingEmail {
	OutgoingEmail {
		id: Some(Uuid::nil()),
		to: "ana@example.com".into(),
		subject: "Hello".into(),
		html: "<p>Hi</p>".into(),
		text: "Hi".into(),
		attachments: vec![Attachment {
			filename: "stub.pdf".into(),
			content_type: "application/pdf".into(),
			bytes: b"%PDF-1.7".to_vec(),
		}],
	}
}

#[tokio::test]
async fn resend_gets_the_expected_request() {
	let (url, captured) = stub_resend(StatusCode::OK).await;
	let sender =
		ResendSender::with_base_url("re_test".into(), "GR <payroll@example.com>".into(), url);

	sender.send(&outgoing()).await.unwrap();

	let captured = captured.lock().unwrap();
	let headers = captured.headers.as_ref().unwrap();
	assert_eq!(headers["authorization"], "Bearer re_test");
	assert_eq!(headers["idempotency-key"], Uuid::nil().to_string());
	let body = captured.body.as_ref().unwrap();
	assert_eq!(body["from"], "GR <payroll@example.com>");
	assert_eq!(body["to"], serde_json::json!(["ana@example.com"]));
	assert_eq!(body["subject"], "Hello");
	assert_eq!(body["html"], "<p>Hi</p>");
	assert_eq!(body["text"], "Hi");
	assert_eq!(body["attachments"][0]["filename"], "stub.pdf");
	assert_eq!(
		body["attachments"][0]["content"], "JVBERi0xLjc=",
		"base64 of %PDF-1.7"
	);
}

#[tokio::test]
async fn resend_errors_are_classified() {
	for (status, retryable) in [
		(StatusCode::TOO_MANY_REQUESTS, true),
		(StatusCode::INTERNAL_SERVER_ERROR, true),
		(StatusCode::UNAUTHORIZED, false),
		(StatusCode::UNPROCESSABLE_ENTITY, false),
	] {
		let (url, _) = stub_resend(status).await;
		let sender = ResendSender::with_base_url("re_test".into(), "x@example.com".into(), url);
		let error = sender.send(&outgoing()).await.unwrap_err();
		assert_eq!(
			matches!(error, SendError::Retryable(_)),
			retryable,
			"{status}: {error:?}"
		);
	}
}

#[tokio::test]
async fn an_unreachable_resend_is_retryable() {
	// Nothing listens on port 1.
	let sender = ResendSender::with_base_url(
		"re_test".into(),
		"x@example.com".into(),
		"http://127.0.0.1:1".into(),
	);
	assert!(matches!(
		sender.send(&outgoing()).await,
		Err(SendError::Retryable(_))
	));
}

#[tokio::test]
async fn the_log_sender_writes_the_email_to_disk() {
	let dir = std::env::temp_dir().join(format!("gr-logistics-email-{}", Uuid::new_v4()));
	LogSender::new(dir.clone()).send(&outgoing()).await.unwrap();

	let folder = std::fs::read_dir(&dir)
		.unwrap()
		.next()
		.unwrap()
		.unwrap()
		.path();
	assert_eq!(
		std::fs::read_to_string(folder.join("email.html")).unwrap(),
		"<p>Hi</p>"
	);
	assert_eq!(
		std::fs::read_to_string(folder.join("email.txt")).unwrap(),
		"Hi"
	);
	assert!(
		std::fs::read_to_string(folder.join("meta.txt"))
			.unwrap()
			.contains("To: ana@example.com")
	);
	assert_eq!(std::fs::read(folder.join("stub.pdf")).unwrap(), b"%PDF-1.7");
	std::fs::remove_dir_all(dir).unwrap();
}
