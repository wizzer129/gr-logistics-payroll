<!-- The monthly report (EMAIL-SPEC.md §8): headline numbers here, the per-employee table in the PDF attachment. -->
<template>
	<EmailShell>
		<Text class="m-0 text-sm font-semibold uppercase tracking-wide text-slate-500">
			<Raw>{{ t.monthly_report.heading }}</Raw>
		</Text>
		<Heading class="mt-1 mb-0 text-2xl text-slate-900">
			<Raw>{{ month }}</Raw>
		</Heading>
		<Section class="mt-6 rounded-md bg-[#1867C0] px-6 py-4">
			<Text class="m-0 text-xs uppercase text-blue-100">
				<Raw>{{ t.monthly_report.total_paid }}</Raw>
			</Text>
			<Text class="m-0 text-3xl font-bold text-white">
				<Raw>{{ total_paid }}</Raw>
			</Text>
			<Text class="m-0 text-sm text-blue-100">
				<Raw>{{ paid_change }}</Raw>
			</Text>
		</Section>
		<Section class="mt-6">
			<Row>
				<Column class="w-1/2 align-top">
					<Text class="m-0 text-xs uppercase text-slate-500">
						<Raw>{{ t.monthly_report.gross }}</Raw>
					</Text>
					<Text class="m-0 font-semibold text-slate-900">
						<Raw>{{ gross }}</Raw>
					</Text>
				</Column>
				<Column class="w-1/2 align-top">
					<Text class="m-0 text-xs uppercase text-slate-500">
						<Raw>{{ t.monthly_report.paychecks }}</Raw>
					</Text>
					<Text class="m-0 font-semibold text-slate-900">
						<Raw>{{ paychecks }}</Raw>
					</Text>
				</Column>
			</Row>
		</Section>
		<Raw>{% if drafts_note %}</Raw>
		<Section class="mt-6 rounded-md bg-amber-50 px-6 py-3">
			<Text class="m-0 text-sm font-medium text-amber-800">
				<Raw>{{ drafts_note }}</Raw>
			</Text>
		</Section>
		<Raw>{% endif %}</Raw>
		<Text class="mt-6 text-sm text-slate-500">
			<Raw>{{ t.monthly_report.attached }}</Raw>
		</Text>
	</EmailShell>
</template>
