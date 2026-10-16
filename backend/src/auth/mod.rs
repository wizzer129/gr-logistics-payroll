//! Session-cookie auth (`gr_session` cookie → `sessions` table). Same format the original
//! Bun backend used, so sessions and password hashes created before the port stay valid.

pub mod extractor;
pub mod middleware;
pub mod password;
pub mod reset_token;
pub mod session;

pub use extractor::{AuthUser, Role};
