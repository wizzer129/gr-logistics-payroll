<!-- POST /api/paycheck-logs/{id}/email (EMAIL-SPEC.md §8). The printable stub is the PDF attachment. -->
<template>
	<EmailShell>
		<Text class="m-0 text-sm font-semibold uppercase tracking-wide text-slate-500">
			<Raw>{{ t.paystub.heading }}</Raw>
		</Text>
		<Heading class="mt-1 mb-0 text-2xl text-slate-900">
			<Raw>{{ employee_name }}</Raw>
		</Heading>
		<Section class="mt-6">
			<Row>
				<Column class="w-1/2 align-top">
					<Text class="m-0 text-xs uppercase text-slate-500">
						<Raw>{{ t.paystub.work_week }}</Raw>
					</Text>
					<Text class="m-0 font-semibold text-slate-900">
						<Raw>{{ week }}</Raw>
					</Text>
				</Column>
				<Column class="w-1/2 align-top">
					<Text class="m-0 text-xs uppercase text-slate-500">
						<Raw>{{ t.paystub.payday }}</Raw>
					</Text>
					<Text class="m-0 font-semibold text-slate-900">
						<Raw>{{ payday }}</Raw>
					</Text>
				</Column>
			</Row>
		</Section>
		<Section class="mt-6 rounded-md bg-[#1867C0] px-6 py-4">
			<Text class="m-0 text-xs uppercase text-blue-100">
				<Raw>{{ t.paystub.total_pay }}</Raw>
			</Text>
			<Text class="m-0 text-3xl font-bold text-white">
				<Raw>{{ total_pay }}</Raw>
			</Text>
		</Section>
		<Text class="mt-6 text-sm text-slate-500">
			<Raw>{{ t.paystub.attached }}</Raw>
		</Text>
	</EmailShell>
</template>
