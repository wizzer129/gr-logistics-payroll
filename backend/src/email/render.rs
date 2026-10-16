//! Fills in the Maizzle-built templates (`templates/email/`, EMAIL-SPEC.md §3) with
//! minijinja. Both the templates and the locale strings are embedded in the binary.

use std::collections::HashMap;

use include_dir::{Dir, include_dir};
use minijinja::{Environment, UndefinedBehavior, value::Serde};
use serde_json::Value;

use super::message::{Locale, Message};

static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates/email");
static LOCALES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/locales/email");

pub struct RenderedEmail {
	pub subject: String,
	pub html: String,
	pub text: String,
}

pub struct Renderer {
	env: Environment<'static>,
	strings: HashMap<Locale, Value>,
}

impl Renderer {
	/// Panics on a malformed embedded template or locale file: those are build artifacts
	/// checked by the tests below, not runtime input.
	pub fn new() -> Self {
		let mut env = Environment::new();
		// A missing placeholder or locale key is an error, never a silently blank email.
		env.set_undefined_behavior(UndefinedBehavior::Strict);
		// Templates keep their file names, so minijinja HTML-escapes `.html` and leaves
		// `.txt` alone (its default per-extension auto-escaping).
		for file in TEMPLATES.files() {
			let name = file.path().to_string_lossy().into_owned();
			let source = file
				.contents_utf8()
				.unwrap_or_else(|| panic!("{name} is not UTF-8"));
			env.add_template_owned(name.clone(), source)
				.unwrap_or_else(|error| panic!("{name}: {error}"));
		}

		let strings = Locale::ALL
			.into_iter()
			.map(|locale| {
				let name = format!("{}.json", locale.as_str());
				let file = LOCALES
					.get_file(&name)
					.unwrap_or_else(|| panic!("missing locales/email/{name}"));
				let json = serde_json::from_slice(file.contents())
					.unwrap_or_else(|error| panic!("locales/email/{name}: {error}"));
				(locale, json)
			})
			.collect();

		Self { env, strings }
	}

	pub fn render(
		&self,
		message: &Message,
		locale: Locale,
	) -> Result<RenderedEmail, minijinja::Error> {
		let template = message.template();
		let t = &self.strings[&locale];
		let mut context = message.context();
		context["t"] = t.clone();
		let context = Serde(&context);

		let subject_key = template.replace('-', "_");
		let subject_source = t[&subject_key]["subject"].as_str().unwrap_or_default();
		let subject = self.env.render_str(subject_source, context)?;

		let html = self
			.env
			.get_template(&format!("{template}.html"))?
			.render(context)?;
		let text = self
			.env
			.get_template(&format!("{template}.txt"))?
			.render(context)?;

		Ok(RenderedEmail {
			subject: subject.trim().to_owned(),
			html,
			text: text.trim().to_owned(),
		})
	}
}

impl Default for Renderer {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use std::collections::BTreeSet;

	use super::*;

	fn keys(value: &Value, prefix: &str, out: &mut BTreeSet<String>) {
		if let Value::Object(map) = value {
			for (key, child) in map {
				keys(child, &format!("{prefix}{key}."), out);
			}
		} else {
			out.insert(prefix.trim_end_matches('.').to_owned());
		}
	}

	#[test]
	fn every_locale_has_the_same_keys() {
		let renderer = Renderer::new();
		let mut en = BTreeSet::new();
		let mut es = BTreeSet::new();
		keys(&renderer.strings[&Locale::En], "", &mut en);
		keys(&renderer.strings[&Locale::Es], "", &mut es);
		assert_eq!(en, es, "locales/email/en.json vs es.json");
	}

	#[test]
	fn every_template_has_both_html_and_text() {
		let names: BTreeSet<_> = TEMPLATES
			.files()
			.map(|file| {
				file.path()
					.with_extension("")
					.to_string_lossy()
					.into_owned()
			})
			.collect();
		for name in names {
			assert!(
				TEMPLATES.get_file(format!("{name}.html")).is_some(),
				"{name}.html"
			);
			assert!(
				TEMPLATES.get_file(format!("{name}.txt")).is_some(),
				"{name}.txt"
			);
		}
	}

	#[test]
	fn renders_in_each_locale_with_no_placeholders_left() {
		let renderer = Renderer::new();
		let message = Message::TestEmail {
			sent_at: "2026-10-16 09:00".into(),
		};
		for locale in Locale::ALL {
			let email = renderer.render(&message, locale).unwrap();
			assert!(!email.subject.is_empty());
			for body in [&email.html, &email.text] {
				assert!(!body.contains("{{"), "{locale:?}: {body}");
				assert!(body.contains("2026-10-16 09:00"), "{locale:?}");
			}
		}

		let es = renderer.render(&message, Locale::Es).unwrap();
		assert_eq!(es.subject, "Correo de prueba de GR Logistics");
		assert!(es.html.contains("El correo funciona"));
	}

	#[test]
	fn values_are_html_escaped_in_html_only() {
		let renderer = Renderer::new();
		let message = Message::TestEmail {
			sent_at: "<b>x</b> & y".into(),
		};
		let email = renderer.render(&message, Locale::En).unwrap();
		// minijinja also escapes `/`, so check the parts rather than one exact string.
		assert!(email.html.contains("&lt;b&gt;x"), "{}", email.html);
		assert!(email.html.contains("&amp; y"), "{}", email.html);
		assert!(!email.html.contains("<b>x"), "{}", email.html);
		assert!(email.text.contains("<b>x</b> & y"), "{}", email.text);
	}
}
