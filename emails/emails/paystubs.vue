<!-- POST /api/email/paystubs (EMAIL-SPEC.md §8): several pay stubs, one PDF attachment each. -->
<template>
	<EmailShell>
		<Text class="m-0 text-sm font-semibold uppercase tracking-wide text-slate-500">
			<Raw>{{ t.paystubs.heading }}</Raw>
		</Text>
		<Heading class="mt-1 mb-0 text-2xl text-slate-900">
			<Raw>{{ t.paystubs.intro }}</Raw>
		</Heading>
		<Section class="mt-6">
			<Row>
				<Column class="w-2/5 pb-2 text-xs uppercase text-slate-500">
					<Raw>{{ t.paystubs.employee }}</Raw>
				</Column>
				<Column class="w-2/5 pb-2 text-xs uppercase text-slate-500">
					<Raw>{{ t.paystubs.work_week }}</Raw>
				</Column>
				<Column class="w-1/5 pb-2 text-right text-xs uppercase text-slate-500">
					<Raw>{{ t.paystubs.total_pay }}</Raw>
				</Column>
			</Row>
			<Raw>{% for stub in stubs %}</Raw>
			<Row>
				<Column
					class="w-2/5 border-0 border-t border-solid border-slate-200 py-2 text-sm font-semibold text-slate-900"
				>
					<Raw>{{ stub.employee_name }}</Raw>
				</Column>
				<Column
					class="w-2/5 border-0 border-t border-solid border-slate-200 py-2 text-sm text-slate-700"
				>
					<Raw>{{ stub.week }}</Raw>
				</Column>
				<Column
					class="w-1/5 border-0 border-t border-solid border-slate-200 py-2 text-right text-sm font-semibold text-slate-900"
				>
					<Raw>{{ stub.total_pay }}</Raw>
				</Column>
			</Row>
			<Raw>{% endfor %}</Raw>
		</Section>
		<Text class="mt-6 text-sm text-slate-500">
			<Raw>{{ t.paystubs.attached }}</Raw>
		</Text>
	</EmailShell>
</template>
