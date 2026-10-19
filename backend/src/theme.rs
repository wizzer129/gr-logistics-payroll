//! A user's color theme for the web app. The frontend owns the palettes (each has a light and
//! a dark variant); the backend only stores which one the user picked.

use serde::{Deserialize, Serialize};

/// Mirrors `users.theme`'s CHECK (migrations 0015, 0016) and `ThemeFamily` in the frontend.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
	#[default]
	Default,
	Catppuccin,
	Dracula,
	Autumn,
	Stormy,
}

impl Theme {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Default => "default",
			Self::Catppuccin => "catppuccin",
			Self::Dracula => "dracula",
			Self::Autumn => "autumn",
			Self::Stormy => "stormy",
		}
	}

	pub fn parse(value: &str) -> Option<Self> {
		match value {
			"default" => Some(Self::Default),
			"catppuccin" => Some(Self::Catppuccin),
			"dracula" => Some(Self::Dracula),
			"autumn" => Some(Self::Autumn),
			"stormy" => Some(Self::Stormy),
			_ => None,
		}
	}
}
