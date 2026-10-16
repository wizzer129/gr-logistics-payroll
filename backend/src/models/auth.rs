use garde::Validate;
use serde::Deserialize;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
	#[garde(email)]
	pub email: String,
	#[garde(length(min = 1))]
	pub password: String,
}

/// Passwords set through a reset (and `create-user`): 12–128 characters. The cap keeps
/// argon2's work bounded. Mirrored in the frontend's reset form.
pub const PASSWORD_MIN_CHARS: usize = 12;
pub const PASSWORD_MAX_CHARS: usize = 128;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct PasswordResetRequest {
	#[garde(email)]
	pub email: String,
	/// The login page's language, for the email. Anything unknown falls back to English.
	#[garde(skip)]
	pub locale: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct PasswordResetConfirm {
	#[garde(length(min = 1, max = 100))]
	pub token: String,
	#[garde(length(chars, min = PASSWORD_MIN_CHARS, max = PASSWORD_MAX_CHARS))]
	pub new_password: String,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLocale {
	#[garde(custom(super::locale))]
	pub locale: String,
}
