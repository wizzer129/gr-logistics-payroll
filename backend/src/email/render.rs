//! Fills in the Maizzle-built templates (`templates/email/`, EMAIL-SPEC.md §3) with
//! minijinja. Both the templates and the locale strings are embedded in the binary.

use std::collections::HashMap;

use include_dir::{Dir, include_dir};
use minijinja::{Environment, UndefinedBehavior, value::Serde};
use serde_json::Value;

use crate::locale::Locale;

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

	/// The locale file for `locale` (also the source of PDF labels).
	pub fn strings(&self, locale: Locale) -> &Value {
		&self.strings[&locale]
	}

	/// Fills `templates/email/<template>.html` / `.txt` and the subject
	/// (`t.<template>.subject`) with `context` plus the locale strings as `t`.
	pub fn render(
		&self,
		template: &str,
		locale: Locale,
		mut context: Value,
	) -> Result<RenderedEmail, minijinja::Error> {
		let t = &self.strings[&locale];
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

	use serde_json::json;

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
	fn every_template_renders_in_each_locale_with_no_placeholders_left() {
		let renderer = Renderer::new();
		// One sample context per template; strict mode fails on anything missing.
		let samples = [
			("test-email", json!({ "sent_at": "SAMPLE-1" })),
			(
				"password-reset",
				json!({ "reset_url": "https://example.com/reset-password?token=SAMPLE-1" }),
			),
			(
				"paystub",
				json!({
					"employee_name": "SAMPLE-1", "week": "Oct 4 – Oct 10, 2026",
					"payday": "Oct 16, 2026", "total_pay": "$441.00",
				}),
			),
			(
				"paystubs",
				json!({
					"count": "2",
					"stubs": [
						{ "employee_name": "SAMPLE-1", "week": "Oct 4 – Oct 10, 2026",
						  "payday": "Oct 16, 2026", "total_pay": "$441.00" },
						{ "employee_name": "SAMPLE-2", "week": "Oct 4 – Oct 10, 2026",
						  "payday": "Oct 16, 2026", "total_pay": "$300.00" },
					],
				}),
			),
			(
				"monthly-report",
				json!({
					"month": "September 2026", "total_paid": "$2,100.00",
					"paid_change": "+40% vs. August 2026", "gross": "$5,000.50",
					"paychecks": "6", "drafts_note": "SAMPLE-1",
				}),
			),
		];
		let built: BTreeSet<_> = TEMPLATES
			.files()
			.map(|file| {
				file.path()
					.with_extension("")
					.to_string_lossy()
					.into_owned()
			})
			.collect();
		let sampled: BTreeSet<_> = samples.iter().map(|(name, _)| (*name).to_owned()).collect();
		assert_eq!(built, sampled, "add a sample here for every template");

		for (template, context) in samples {
			for locale in Locale::ALL {
				let email = renderer
					.render(template, locale, context.clone())
					.unwrap_or_else(|error| panic!("{template} {locale:?}: {error:#}"));
				assert!(!email.subject.is_empty(), "{template} {locale:?}");
				for body in [&email.html, &email.text] {
					assert!(!body.contains("{{"), "{template} {locale:?}: {body}");
					assert!(body.contains("SAMPLE-1"), "{template} {locale:?}");
				}
			}
		}

		let es = renderer
			.render("test-email", Locale::Es, json!({ "sent_at": "x" }))
			.unwrap();
		assert_eq!(es.subject, "Correo de prueba de GR Logistics");
		assert!(es.html.contains("El correo funciona"));

		let paystub = renderer
			.render("paystub", Locale::En, samples_paystub())
			.unwrap();
		assert_eq!(paystub.subject, "Pay stub: Ana Diaz · Oct 4 – Oct 10, 2026");
	}

	#[test]
	fn the_drafts_note_shows_only_when_there_are_drafts() {
		let renderer = Renderer::new();
		let context = |note: &str| {
			json!({
				"month": "September 2026", "total_paid": "$0.00", "paid_change": "",
				"gross": "$0.00", "paychecks": "0", "drafts_note": note,
			})
		};
		let with = renderer
			.render("monthly-report", Locale::En, context("DRAFTS-NOTE"))
			.unwrap();
		assert!(with.html.contains("DRAFTS-NOTE"));
		let without = renderer
			.render("monthly-report", Locale::En, context(""))
			.unwrap();
		assert!(!without.html.contains("#fffbeb"), "no empty warning box");
		assert!(with.html.contains("#fffbeb"), "the warning box");
		assert_eq!(with.subject, "Monthly report: September 2026");
	}

	fn samples_paystub() -> Value {
		json!({
			"employee_name": "Ana Diaz", "week": "Oct 4 – Oct 10, 2026",
			"payday": "Oct 16, 2026", "total_pay": "$441.00",
		})
	}

	#[test]
	fn values_are_html_escaped_in_html_only() {
		let renderer = Renderer::new();
		let context = json!({ "sent_at": "<b>x</b> & y" });
		let email = renderer.render("test-email", Locale::En, context).unwrap();
		// minijinja also escapes `/`, so check the parts rather than one exact string.
		assert!(email.html.contains("&lt;b&gt;x"), "{}", email.html);
		assert!(email.html.contains("&amp; y"), "{}", email.html);
		assert!(!email.html.contains("<b>x"), "{}", email.html);
		assert!(email.text.contains("<b>x</b> & y"), "{}", email.text);
	}
}
