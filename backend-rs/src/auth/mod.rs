//! Session-cookie auth. Byte-compatible with the Bun backend (same `gr_session` cookie, same
//! `sessions` table) so existing logins survive the cutover.

pub mod extractor;
pub mod middleware;
pub mod password;
pub mod session;

pub use extractor::{AuthUser, Role};
