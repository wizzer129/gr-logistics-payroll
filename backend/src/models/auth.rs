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
