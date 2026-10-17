//! The integration tests, as one test binary: one link instead of one per file, which is
//! most of the test build time (Typst makes every binary large). Run one file's tests with
//! `cargo test --test it <module>`, e.g. `cargo test --test it monthly_report`.

mod common;

mod auth;
mod dashboard;
mod email;
mod email_recipients;
mod employees;
mod env_schema;
mod migrate;
mod monthly_report;
mod password_reset;
mod paycheck_logs;
mod paystub_email;
mod routing;
mod stores;
