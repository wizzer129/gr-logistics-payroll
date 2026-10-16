<!-- `gr-logistics send-test-email`: proves the provider, domain and network work end to end. -->
<template>
	<EmailShell>
		<Heading class="m-0 text-2xl text-slate-900">
			<Raw>{{ t.test_email.heading }}</Raw>
		</Heading>
		<Text class="text-slate-700">
			<Raw>{{ t.test_email.body }}</Raw>
		</Text>
		<Text class="text-sm text-slate-500">
			<Raw>{{ t.test_email.sent_at }} {{ sent_at }}</Raw>
		</Text>
	</EmailShell>
</template>
