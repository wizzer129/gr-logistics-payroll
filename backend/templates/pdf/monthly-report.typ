// Monthly report PDF (EMAIL-SPEC.md §8): the month's totals against the month before, and one
// row per employee. Every string arrives already translated and formatted from Rust
// (`src/pdf/monthly_report.rs`) as JSON in `sys.inputs.data`.
#let d = json(bytes(sys.inputs.data))
#let l = d.labels

#let ink = rgb("#1c2430")
#let muted = rgb("#6b7686")
#let rule = rgb("#dfe4ea")
#let band = rgb("#f4f6f9")
#let accent = rgb("#1976d2")
#let accent-ink = rgb("#0d4a8f")
#let warn = rgb("#fff4e5")
#let warn-ink = rgb("#8a4b00")

#set page(
	paper: "us-letter",
	margin: (x: 0.6in, y: 0.6in),
	footer: context align(right, text(size: 7pt, fill: muted, counter(page).display("1 / 1", both: true))),
)
#set text(font: "Noto Sans", size: 9pt, fill: ink)

#let caps(body, size: 7pt, fill: muted) = text(
	size: size, weight: "medium", fill: fill, tracking: 0.08em, upper(body),
)

#let stat(label, s, highlight: false) = block(
	width: 100%,
	fill: if highlight { accent } else { band },
	stroke: if highlight { none } else { 0.6pt + rule },
	radius: 3pt,
	inset: (x: 10pt, y: 8pt),
)[
	#let fg = if highlight { white } else { ink }
	#let sub = if highlight { white.transparentize(20%) } else { muted }
	#caps(label, fill: sub) \
	#text(size: 16pt, weight: "bold", fill: fg, s.value) \
	#text(size: 7.5pt, fill: sub, if s.change == none { "—" } else { s.change + " " + d.vs_previous })
]

#block(width: 100%, stroke: (top: 3pt + accent), inset: (top: 12pt))[
	#caps(l.company, size: 8pt, fill: accent-ink) \
	#text(size: 20pt, weight: "bold", l.title) \
	#text(size: 12pt, fill: muted, d.month)
]
#v(10pt)

#grid(
	columns: (1fr, 1fr, 1fr, 1fr),
	column-gutter: 8pt,
	stat(l.total_paid, d.paid, highlight: true),
	stat(l.gross, d.gross),
	stat(l.paychecks, d.paychecks),
	stat(l.loads, d.loads),
)

#if d.drafts_note != none {
	v(8pt)
	block(width: 100%, fill: warn, radius: 3pt, inset: (x: 10pt, y: 7pt),
		text(fill: warn-ink, weight: "medium", d.drafts_note))
}

#v(14pt)
#caps(l.by_employee, size: 8pt, fill: ink)
#v(4pt)

#if d.employees.len() == 0 {
	align(center, text(fill: muted, l.no_paychecks))
} else {
	table(
		columns: (1fr, auto, auto, auto, auto),
		inset: (x: 6pt, y: 4pt),
		stroke: (x, y) => if y == 0 { (bottom: 1.1pt + ink) } else { (bottom: 0.6pt + rule) },
		fill: (x, y) => if y > 0 and calc.even(y) { band },
		align: (x, y) => if x >= 1 { right } else { left },
		table.header(caps(l.employee), caps(l.paychecks), caps(l.loads), caps(l.gross), caps(l.paid)),
		..d.employees
			.map(r => (text(weight: "medium", r.name), r.paychecks, r.loads, r.gross, r.paid))
			.flatten(),
		table.footer(
			..(d.total.name, d.total.paychecks, d.total.loads, d.total.gross, d.total.paid)
				.map(c => text(weight: "bold", c)),
		),
	)
}
