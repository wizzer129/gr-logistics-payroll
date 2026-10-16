use axum::http::HeaderValue;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Environment {
	Development,
	Production,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogFormat {
	Pretty,
	Json,
}

/// Where outgoing email goes (EMAIL-SPEC.md §4).
#[derive(Clone)]
pub enum EmailProvider {
	/// Development: write each email to `backend/target/emails/` instead of sending it.
	Log,
	Resend {
		api_key: String,
		from: String,
	},
}

/// No `Debug` derive — this holds secrets and must never end up in a log line.
#[derive(Clone)]
pub struct Config {
	pub environment: Environment,
	pub port: u16,
	pub database_url: String,
	pub session_ttl_hours: i64,
	pub cors_origin: HeaderValue,
	pub log_level: String,
	pub log_format: LogFormat,
	pub email: EmailProvider,
}

/// Must be set (non-empty) or startup fails. Keep `.env.schema` in sync (tests/env_schema.rs).
pub const REQUIRED_KEYS: &[&str] = &[
	"NODE_ENV",
	"PORT",
	"DATABASE_URL",
	"SESSION_TTL_HOURS",
	"CORS_ORIGIN",
	"LOG_LEVEL",
];

/// Read when present, with a default otherwise.
pub const OPTIONAL_KEYS: &[&str] = &[
	"LOG_FORMAT",
	"EMAIL_PROVIDER",
	"EMAIL_FROM",
	"RESEND_API_KEY",
];

impl Config {
	/// Fails fast listing every missing key at once, like the TS `assertEnv()`.
	pub fn from_env() -> anyhow::Result<Self> {
		let missing: Vec<_> = REQUIRED_KEYS
			.iter()
			.filter(|key| std::env::var(key).map_or(true, |v| v.is_empty()))
			.copied()
			.collect();

		if !missing.is_empty() {
			anyhow::bail!(
				"Missing required environment variables: {}",
				missing.join(", ")
			);
		}

		let environment = match var("NODE_ENV").as_str() {
			"production" => Environment::Production,
			_ => Environment::Development,
		};

		let log_format = match std::env::var("LOG_FORMAT").ok().as_deref() {
			Some("json") => LogFormat::Json,
			Some("pretty") => LogFormat::Pretty,
			_ if environment == Environment::Development => LogFormat::Pretty,
			_ => LogFormat::Json,
		};

		Ok(Self {
			environment,
			port: var("PORT").parse()?,
			database_url: var("DATABASE_URL"),
			session_ttl_hours: var("SESSION_TTL_HOURS").parse()?,
			cors_origin: var("CORS_ORIGIN").parse()?,
			log_level: var("LOG_LEVEL"),
			log_format,
			email: email_provider(
				|key| std::env::var(key).ok().filter(|v| !v.is_empty()),
				environment,
			)?,
		})
	}

	pub fn is_production(&self) -> bool {
		self.environment == Environment::Production
	}
}

/// `EMAIL_PROVIDER` plus its keys. Production can't run with the `log` provider (it would
/// silently write emails to disk), and `resend` needs both its key and a sender address.
pub fn email_provider(
	var: impl Fn(&str) -> Option<String>,
	environment: Environment,
) -> anyhow::Result<EmailProvider> {
	match var("EMAIL_PROVIDER").as_deref() {
		None | Some("log") if environment == Environment::Production => anyhow::bail!(
			"EMAIL_PROVIDER must be \"resend\" in production (the \"log\" provider never sends)"
		),
		None | Some("log") => Ok(EmailProvider::Log),
		Some("resend") => {
			let missing: Vec<_> = ["RESEND_API_KEY", "EMAIL_FROM"]
				.into_iter()
				.filter(|key| var(key).is_none())
				.collect();
			if !missing.is_empty() {
				anyhow::bail!("EMAIL_PROVIDER=resend needs: {}", missing.join(", "));
			}
			Ok(EmailProvider::Resend {
				api_key: var("RESEND_API_KEY").unwrap_or_default(),
				from: var("EMAIL_FROM").unwrap_or_default(),
			})
		}
		Some(other) => {
			anyhow::bail!("unknown EMAIL_PROVIDER {other:?}; expected \"log\" or \"resend\"")
		}
	}
}

/// Only called after the REQUIRED_KEYS check, so a missing key is already impossible.
fn var(key: &str) -> String {
	std::env::var(key).unwrap_or_default()
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	use super::*;

	fn provider(pairs: &[(&str, &str)], environment: Environment) -> anyhow::Result<EmailProvider> {
		let env: HashMap<_, _> = pairs.iter().copied().collect();
		email_provider(|key| env.get(key).map(|v| (*v).to_owned()), environment)
	}

	#[test]
	fn development_defaults_to_the_log_provider() {
		assert!(matches!(
			provider(&[], Environment::Development).unwrap(),
			EmailProvider::Log
		));
	}

	#[test]
	fn production_refuses_the_log_provider() {
		assert!(provider(&[], Environment::Production).is_err());
		assert!(provider(&[("EMAIL_PROVIDER", "log")], Environment::Production).is_err());
	}

	#[test]
	fn resend_needs_its_key_and_a_sender() {
		let error = provider(&[("EMAIL_PROVIDER", "resend")], Environment::Production)
			.err()
			.unwrap()
			.to_string();
		assert!(
			error.contains("RESEND_API_KEY") && error.contains("EMAIL_FROM"),
			"{error}"
		);

		let ok = provider(
			&[
				("EMAIL_PROVIDER", "resend"),
				("RESEND_API_KEY", "re_test"),
				("EMAIL_FROM", "GR Logistics <payroll@example.com>"),
			],
			Environment::Production,
		)
		.unwrap();
		assert!(matches!(ok, EmailProvider::Resend { .. }));
	}

	#[test]
	fn an_unknown_provider_is_an_error() {
		assert!(provider(&[("EMAIL_PROVIDER", "smtp")], Environment::Development).is_err());
	}
}
