//! The app's two languages. Used for email, PDFs and each user's/employee's saved language.

use serde::{Deserialize, Serialize};

/// `en` or `es`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
	En,
	Es,
}

impl Locale {
	pub const ALL: [Self; 2] = [Self::En, Self::Es];

	pub fn as_str(self) -> &'static str {
		match self {
			Self::En => "en",
			Self::Es => "es",
		}
	}

	pub fn parse(value: &str) -> Option<Self> {
		match value {
			"en" => Some(Self::En),
			"es" => Some(Self::Es),
			_ => None,
		}
	}
}
