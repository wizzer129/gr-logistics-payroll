// Pay stub PDF (EMAIL-SPEC.md §8). Mirrors the frontend's PaystubSheet/Paystub.vue: two
// copies on one letter page with a cut line. Every string arrives already translated and
// formatted from Rust (`src/pdf/paystub.rs`) as JSON in `sys.inputs.data`.
#let d = json(bytes(sys.inputs.data))
#let l = d.labels

#let ink = rgb("#1c2430")
#let muted = rgb("#6b7686")
#let rule = rgb("#dfe4ea")
#let band = rgb("#f4f6f9")
#let accent = rgb("#1976d2")
#let accent-ink = rgb("#0d4a8f")

#set page(paper: "us-letter", margin: (x: 0.5in, y: 0.4in))
#set text(font: "Noto Sans", size: 8pt, fill: ink)

#let caps(body, size: 6.5pt, fill: muted) = text(
	size: size, weight: "medium", fill: fill, tracking: 0.08em, upper(body),
)

#let loads-table(rows) = table(
	columns: (auto, 1fr, auto, auto),
	inset: (x: 4pt, y: 2.4pt),
	stroke: (x, y) => if y == 0 { (bottom: 1.1pt + ink) } else { (bottom: 0.6pt + rule) },
	fill: (x, y) => if y > 0 and calc.even(y) { band },
	align: (x, y) => if x >= 2 { right } else { left },
	table.header(caps(l.date), caps(l.load), caps(l.pay), caps(l.extra)),
	..rows
		.map(r => (
			text(fill: muted, r.date),
			text(weight: "medium", r.store),
			r.pay,
			if r.extra == none { text(fill: rgb("#b3bac4"), "—") } else { r.extra },
		))
		.flatten(),
)

#let stub(copy-label) = block(width: 100%, breakable: false, stroke: (top: 3pt + accent), inset: (top: 10pt))[
	#grid(
		columns: (1fr, auto),
		align: (left + bottom, right + bottom),
		[
			#caps(d.company, size: 7.5pt, fill: accent-ink) \
			#text(size: 14pt, weight: "bold", d.employee_name)
		],
		[
			#caps(l.title, size: 8pt) \
			#box(
				stroke: 0.6pt + accent, radius: 6pt, inset: (x: 6pt, y: 2pt),
				caps(copy-label, size: 6.5pt, fill: accent-ink),
			)
		],
	)
	#v(4pt)
	#block(width: 100%, fill: band, stroke: 0.6pt + rule, radius: 3pt, inset: (x: 8pt, y: 5pt))[
		#grid(
			columns: (1fr, 1fr, 1fr),
			[#caps(l.work_week) \ #text(size: 9.5pt, weight: "bold", d.week)],
			[#caps(l.payday) \ #text(size: 9.5pt, weight: "bold", d.payday)],
			if d.lump_sum {
				[#caps(l.payment_type) \ #text(size: 9.5pt, weight: "bold", l.lump_sum)]
			} else {
				[#caps(l.load_count) \ #text(size: 9.5pt, weight: "bold", str(d.loads.len()))]
			},
		)
	]
	#v(4pt)
	#if not d.lump_sum {
		if d.loads.len() == 0 {
			align(center, text(fill: muted, l.no_loads))
		} else if d.loads.len() <= 8 {
			loads-table(d.loads)
		} else {
			// Long weeks split into two columns, like the screen, to stay half a page tall.
			let half = calc.ceil(d.loads.len() / 2)
			grid(
				columns: (1fr, 1fr),
				column-gutter: 14pt,
				loads-table(d.loads.slice(0, half)),
				loads-table(d.loads.slice(half)),
			)
		}
	}
	#v(4pt)
	#line(length: 100%, stroke: 1.1pt + ink)
	#grid(
		columns: (1fr, 1fr, 1fr, auto),
		column-gutter: 8pt,
		align: horizon,
		if d.lump_sum { [] } else {
			[#caps(l.extra_pay_total) \ #text(size: 10pt, weight: "bold", d.extra_total)]
		},
		[#caps(l.summary_total) \ #text(size: 10pt, weight: "bold", d.summary_total)],
		[#caps(d.employee_cut_label) \ #text(size: 10pt, weight: "bold", d.employee_cut)],
		block(fill: accent, radius: 3pt, inset: (x: 10pt, y: 5pt), width: 1.7in)[
			#set align(right)
			#caps(l.total_pay, fill: white.transparentize(20%)) \
			#text(size: 15pt, weight: "bold", fill: white, d.rounded_pay)
		],
	)
]

#stub(l.contractor)
#v(0.15in)
#line(length: 100%, stroke: (paint: rgb("#9aa4b1"), thickness: 0.6pt, dash: "dashed"))
#v(0.15in)
#stub(l.owner_operator)
