//! `gr-logistics send-test-email --to <address>` — one email straight through the configured
//! provider, skipping the outbox, so a deploy can prove the key, the sender domain and the
//! outbound network work before any real email depends on them (EMAIL-SPEC.md §4).

use anyhow::{Context, anyhow};

use crate::{
	config::Config,
	email::{
		Locale, Message,
		render::Renderer,
		sender::{self, OutgoingEmail},
	},
};

pub async fn run(config: &Config, to: &str, locale: &str) -> anyhow::Result<()> {
	let locale = Locale::parse(locale)
		.ok_or_else(|| anyhow!("unknown locale {locale:?}; expected \"en\" or \"es\""))?;
	let message = Message::TestEmail {
		sent_at: chrono::Utc::now().format("%Y-%m-%d %H:%M UTC").to_string(),
	};
	let rendered = Renderer::new()
		.render(&message, locale)
		.context("rendering the test email")?;

	sender::from_config(&config.email)
		.send(&OutgoingEmail {
			id: None,
			to: to.to_owned(),
			subject: rendered.subject,
			html: rendered.html,
			text: rendered.text,
			attachments: Vec::new(),
		})
		.await
		.map_err(|error| anyhow!("send failed: {error}"))?;

	tracing::info!(to, "test email sent");
	Ok(())
}
