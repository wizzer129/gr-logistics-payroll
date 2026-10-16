//! Where a rendered email actually goes (EMAIL-SPEC.md §4): Resend in production, files on
//! disk in development, memory in tests.

use std::{
	future::Future,
	path::PathBuf,
	pin::Pin,
	sync::{Arc, Mutex},
	time::Duration,
};

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use serde_json::json;
use uuid::Uuid;

use crate::config::EmailProvider;

#[derive(Clone, Debug)]
pub struct Attachment {
	pub filename: String,
	pub content_type: String,
	pub bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct OutgoingEmail {
	/// The outbox row id, sent as the idempotency key so a retried send isn't delivered
	/// twice. `None` for one-off sends (`send-test-email`).
	pub id: Option<Uuid>,
	pub to: String,
	pub subject: String,
	pub html: String,
	pub text: String,
	pub attachments: Vec<Attachment>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SendError {
	/// Worth retrying: network trouble, a 5xx, or rate limiting (429).
	Retryable(String),
	/// Retrying won't help: a bad address, an unverified domain, a rejected key…
	Permanent(String),
}

impl std::fmt::Display for SendError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Retryable(message) | Self::Permanent(message) => f.write_str(message),
		}
	}
}

pub type SendFuture<'a> = Pin<Box<dyn Future<Output = Result<(), SendError>> + Send + 'a>>;

/// Boxed future rather than `async fn` so the worker can hold an `Arc<dyn EmailSender>`.
pub trait EmailSender: Send + Sync {
	fn send<'a>(&'a self, email: &'a OutgoingEmail) -> SendFuture<'a>;
}

pub fn from_config(provider: &EmailProvider) -> Arc<dyn EmailSender> {
	match provider {
		EmailProvider::Log => Arc::new(LogSender::new(LogSender::default_dir())),
		EmailProvider::Resend { api_key, from } => {
			Arc::new(ResendSender::new(api_key.clone(), from.clone()))
		}
	}
}

// --- Resend ---

const RESEND_API: &str = "https://api.resend.com";

pub struct ResendSender {
	client: reqwest::Client,
	api_key: String,
	from: String,
	base_url: String,
}

impl ResendSender {
	pub fn new(api_key: String, from: String) -> Self {
		Self::with_base_url(api_key, from, RESEND_API.to_owned())
	}

	/// Tests point this at a local stub server.
	pub fn with_base_url(api_key: String, from: String, base_url: String) -> Self {
		crate::install_tls_provider();
		let client = reqwest::Client::builder()
			.timeout(Duration::from_secs(30))
			.build()
			.expect("reqwest client with a TLS provider installed");
		Self {
			client,
			api_key,
			from,
			base_url,
		}
	}

	async fn send_inner(&self, email: &OutgoingEmail) -> Result<(), SendError> {
		let attachments: Vec<_> = email
			.attachments
			.iter()
			.map(|a| json!({ "filename": a.filename, "content": BASE64.encode(&a.bytes) }))
			.collect();
		let body = json!({
			"from": self.from,
			"to": [email.to],
			"subject": email.subject,
			"html": email.html,
			"text": email.text,
			"attachments": attachments,
		});

		let mut request = self
			.client
			.post(format!("{}/emails", self.base_url))
			.bearer_auth(&self.api_key)
			.json(&body);
		if let Some(id) = email.id {
			request = request.header("Idempotency-Key", id.to_string());
		}

		let response = request
			.send()
			.await
			.map_err(|error| SendError::Retryable(format!("Resend unreachable: {error}")))?;

		let status = response.status();
		if status.is_success() {
			return Ok(());
		}
		let detail = response.text().await.unwrap_or_default();
		let message = format!("Resend returned {status}: {detail}");
		if status.as_u16() == 429 || status.is_server_error() {
			Err(SendError::Retryable(message))
		} else {
			Err(SendError::Permanent(message))
		}
	}
}

impl EmailSender for ResendSender {
	fn send<'a>(&'a self, email: &'a OutgoingEmail) -> SendFuture<'a> {
		Box::pin(self.send_inner(email))
	}
}

// --- Development: files on disk ---

/// Writes each email to `<dir>/<timestamp>-<id>/` (`email.html`, `email.txt`, `meta.txt`,
/// attachments) and logs the folder, so you can open exactly what would have been sent.
pub struct LogSender {
	dir: PathBuf,
}

impl LogSender {
	pub fn new(dir: PathBuf) -> Self {
		Self { dir }
	}

	pub fn default_dir() -> PathBuf {
		PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/emails")
	}

	fn write(&self, email: &OutgoingEmail) -> std::io::Result<PathBuf> {
		let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S%.3f");
		let id = email.id.unwrap_or_else(Uuid::new_v4);
		let folder = self.dir.join(format!("{stamp}-{id}"));
		std::fs::create_dir_all(&folder)?;
		std::fs::write(folder.join("email.html"), &email.html)?;
		std::fs::write(folder.join("email.txt"), &email.text)?;
		std::fs::write(
			folder.join("meta.txt"),
			format!("To: {}\nSubject: {}\n", email.to, email.subject),
		)?;
		for attachment in &email.attachments {
			std::fs::write(folder.join(&attachment.filename), &attachment.bytes)?;
		}
		Ok(folder)
	}
}

impl EmailSender for LogSender {
	fn send<'a>(&'a self, email: &'a OutgoingEmail) -> SendFuture<'a> {
		Box::pin(async move {
			// Dev-only and tiny, so plain blocking writes are fine here.
			let folder = self
				.write(email)
				.map_err(|error| SendError::Permanent(format!("couldn't write email: {error}")))?;
			tracing::info!(to = %email.to, subject = %email.subject, folder = %folder.display(), "email written (EMAIL_PROVIDER=log)");
			Ok(())
		})
	}
}

// --- Tests ---

/// Records what was sent; `fail_with` makes the next sends fail. Public because the
/// integration tests (a separate crate) use it.
#[derive(Default)]
pub struct MemorySender {
	pub sent: Mutex<Vec<OutgoingEmail>>,
	pub fail_with: Mutex<Option<SendError>>,
}

impl MemorySender {
	pub fn sent(&self) -> Vec<OutgoingEmail> {
		self.sent.lock().unwrap().clone()
	}
}

impl EmailSender for MemorySender {
	fn send<'a>(&'a self, email: &'a OutgoingEmail) -> SendFuture<'a> {
		Box::pin(async move {
			if let Some(error) = self.fail_with.lock().unwrap().clone() {
				return Err(error);
			}
			self.sent.lock().unwrap().push(email.clone());
			Ok(())
		})
	}
}
