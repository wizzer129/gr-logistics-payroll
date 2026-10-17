//! All SQL lives here. Functions that write take `&mut PgConnection` (a transaction) so
//! callers compose them into one atomic unit.

pub mod audit;
pub mod dashboard;
pub mod email_outbox;
pub mod email_recipients;
pub mod employees;
pub mod password_resets;
pub mod paycheck_logs;
pub mod paystubs;
pub mod stores;
