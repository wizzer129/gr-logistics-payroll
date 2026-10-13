//! Thin HTTP layer: extract → call a service → map to a response DTO. No SQL here.

pub mod auth;
pub mod dashboard;
pub mod employees;
pub mod health;
pub mod loads;
pub mod paycheck_logs;
