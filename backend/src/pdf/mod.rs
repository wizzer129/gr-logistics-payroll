//! PDFs (EMAIL-SPEC.md §8), rendered in-process with Typst: no Chromium, so the distroless
//! image stays as it is. Templates (`templates/pdf/*.typ`) and fonts (`fonts/`, Noto Sans,
//! OFL) are embedded in the binary.

pub mod monthly_report;
pub mod paystub;

use std::sync::LazyLock;

use serde::Serialize;
use typst::foundations::{Dict, IntoValue};
use typst_as_lib::{TypstEngine, TypstTemplateMainFile};
use typst_layout::PagedDocument;

const FONTS: [&[u8]; 3] = [
	include_bytes!("../../fonts/NotoSans-Regular.ttf"),
	include_bytes!("../../fonts/NotoSans-Medium.ttf"),
	include_bytes!("../../fonts/NotoSans-Bold.ttf"),
];

/// Built once: parsing the fonts and template is the slow part.
static PAYSTUB: LazyLock<TypstEngine<TypstTemplateMainFile>> = LazyLock::new(|| {
	TypstEngine::builder()
		.main_file(include_str!("../../templates/pdf/paystub.typ"))
		.fonts(FONTS)
		.build()
});

static MONTHLY_REPORT: LazyLock<TypstEngine<TypstTemplateMainFile>> = LazyLock::new(|| {
	TypstEngine::builder()
		.main_file(include_str!("../../templates/pdf/monthly-report.typ"))
		.fonts(FONTS)
		.build()
});

/// Compiles `engine` with `data` as JSON in `sys.inputs.data`. CPU-bound (tens of ms), so
/// async callers run it on the blocking pool.
fn render(
	engine: &TypstEngine<TypstTemplateMainFile>,
	data: &impl Serialize,
) -> Result<Vec<u8>, String> {
	let json = serde_json::to_string(data).map_err(|error| error.to_string())?;
	let mut inputs = Dict::new();
	inputs.insert("data".into(), json.into_value());

	let document: PagedDocument = engine
		.compile_with_input(inputs)
		.output
		.map_err(|error| format!("typst compile failed: {error:?}"))?;
	typst_pdf::pdf(&document, &Default::default())
		.map_err(|errors| format!("pdf export failed: {errors:?}"))
}

pub fn paystub(document: &paystub::PaystubDocument) -> Result<Vec<u8>, String> {
	render(&PAYSTUB, document)
}

pub fn monthly_report(document: &monthly_report::MonthlyReportDocument) -> Result<Vec<u8>, String> {
	render(&MONTHLY_REPORT, document)
}
