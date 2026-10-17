//! The monthly report's automatic send (EMAIL-SPEC.md §8): a task in `serve` that checks every
//! hour and, from 08:00 America/Los_Angeles on the 1st, queues the previous month's report for
//! the opted-in users, once.

use std::time::Duration;

use chrono::{DateTime, Datelike, Months, NaiveDate, Timelike, Utc};
use chrono_tz::{America::Los_Angeles, Tz};
use sqlx::PgPool;
use tokio::sync::watch;
use uuid::Uuid;

use super::{Message, enqueue_for_each};
use crate::services::{email_recipients, monthly_report};

/// The business's clock: "the 1st at 8am" means Los Angeles time, DST included.
pub const TIMEZONE: Tz = Los_Angeles;
pub const SEND_HOUR: u32 = 8;
/// A backend that was down on the 1st still sends when it comes back that week, but a first
/// deploy mid-month (or a long outage) doesn't send last month's report out of the blue.
/// Outside the window, "Send now" covers it.
pub const CATCH_UP_DAYS: u32 = 7;
const CHECK_EVERY: Duration = Duration::from_secs(60 * 60);

/// The month (its 1st) whose report is due at `now`, if any.
pub fn due_month(now: DateTime<Utc>) -> Option<NaiveDate> {
	let local = now.with_timezone(&TIMEZONE);
	let day = local.day();
	if day > CATCH_UP_DAYS || (day == 1 && local.hour() < SEND_HOUR) {
		return None;
	}
	let this_month = local.date_naive().with_day(1)?;
	Some(this_month - Months::new(1))
}

/// One check: queues the due month's report unless it already was. Returns the new outbox
/// ids, or `None` if nothing was queued. With nobody opted in it records nothing, so adding
/// a recipient later in the window still gets that month's report out.
pub async fn tick(pool: &PgPool, now: DateTime<Utc>) -> Result<Option<Vec<Uuid>>, sqlx::Error> {
	let Some(month) = due_month(now) else {
		return Ok(None);
	};

	let mut tx = pool.begin().await?;
	let recipients = email_recipients::monthly_report_recipients(&mut tx).await?;
	if recipients.is_empty() {
		tracing::debug!(%month, "monthly report due, but nobody receives it");
		return Ok(None);
	}
	// A concurrent tick blocks here on the primary key until this one commits, then skips.
	if !monthly_report::claim_run(&mut tx, month).await? {
		return Ok(None);
	}
	let message = Message::MonthlyReport { month };
	let ids = enqueue_for_each(&mut tx, &message, &recipients, None).await?;
	monthly_report::record_run_outbox_ids(&mut tx, month, &ids).await?;
	tx.commit().await?;

	tracing::info!(%month, recipients = ids.len(), "monthly report queued");
	Ok(Some(ids))
}

/// Runs until `shutdown` flips to `true`.
pub async fn run(pool: PgPool, mut shutdown: watch::Receiver<bool>) {
	tracing::info!("monthly report scheduler started");
	loop {
		if let Err(error) = tick(&pool, Utc::now()).await {
			tracing::error!(%error, "monthly report check failed");
		}
		tokio::select! {
			_ = shutdown.changed() => break,
			() = tokio::time::sleep(CHECK_EVERY) => {},
		}
		if *shutdown.borrow() {
			break;
		}
	}
	tracing::info!("monthly report scheduler stopped");
}

#[cfg(test)]
mod tests {
	use chrono::TimeZone;

	use super::*;

	/// A Los Angeles wall-clock time, as UTC.
	fn la(y: i32, m: u32, d: u32, h: u32) -> DateTime<Utc> {
		TIMEZONE
			.with_ymd_and_hms(y, m, d, h, 0, 0)
			.unwrap()
			.with_timezone(&Utc)
	}

	fn month(y: i32, m: u32) -> Option<NaiveDate> {
		NaiveDate::from_ymd_opt(y, m, 1)
	}

	#[test]
	fn due_from_8am_on_the_1st_through_the_7th() {
		assert_eq!(due_month(la(2026, 10, 1, 7)), None, "before 8am");
		assert_eq!(due_month(la(2026, 10, 1, 8)), month(2026, 9));
		assert_eq!(due_month(la(2026, 10, 7, 23)), month(2026, 9), "catch-up");
		assert_eq!(due_month(la(2026, 10, 8, 0)), None, "past the window");
		assert_eq!(
			due_month(la(2026, 1, 1, 9)),
			month(2025, 12),
			"across the year"
		);
	}

	#[test]
	fn uses_los_angeles_time_not_utc() {
		// 2026-10-01 03:00 UTC is still Sep 30 in Los Angeles.
		let utc = Utc.with_ymd_and_hms(2026, 10, 1, 3, 0, 0).unwrap();
		assert_eq!(due_month(utc), None);
		// 8am PST (after DST ends) is 16:00 UTC.
		assert_eq!(
			due_month(Utc.with_ymd_and_hms(2026, 12, 1, 15, 59, 0).unwrap()),
			None
		);
		assert_eq!(
			due_month(Utc.with_ymd_and_hms(2026, 12, 1, 16, 0, 0).unwrap()),
			month(2026, 11)
		);
	}
}
