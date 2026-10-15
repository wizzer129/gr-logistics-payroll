//! All SQL lives here. Functions that write take `&mut PgConnection` (a transaction) so
//! callers compose them into one atomic unit.

pub mod audit;
pub mod dashboard;
pub mod employees;
pub mod paycheck_logs;
pub mod stores;
