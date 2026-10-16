<!-- POST /api/auth/password-reset (EMAIL-SPEC.md §7). `reset_url` carries the one-time token. -->
<template>
	<EmailShell>
		<Heading class="m-0 text-2xl text-slate-900">
			<Raw>{{ t.password_reset.heading }}</Raw>
		</Heading>
		<Text class="text-slate-700">
			<Raw>{{ t.password_reset.body }}</Raw>
		</Text>
		<Button href="{{ reset_url }}" class="mt-2 rounded-md bg-[#1867C0] text-white">
			<Raw>{{ t.password_reset.button }}</Raw>
		</Button>
		<Text class="mt-6 text-sm text-slate-500">
			<Raw>{{ t.password_reset.expires }}</Raw>
		</Text>
		<!-- Plain text already shows the link under the button, so skip the fallback there. -->
		<NotPlaintext>
			<Text class="text-sm text-slate-500">
				<Raw>{{ t.password_reset.fallback }}</Raw>
				<br />
				<a href="{{ reset_url }}" class="break-all text-[#1867C0]">
					<Raw>{{ reset_url }}</Raw>
				</a>
			</Text>
		</NotPlaintext>
		<Text class="text-sm text-slate-500">
			<Raw>{{ t.password_reset.ignore }}</Raw>
		</Text>
	</EmailShell>
</template>
