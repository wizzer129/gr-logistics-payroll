<!--
  Shared frame for every email: header, white card, footer. Text comes from the backend's
  locale files at send time, so every string is a Raw placeholder, never literal copy.
  (Don't write the Raw tag itself in a comment: Maizzle's raw-extract step matches it there too.)
-->
<template>
	<Layout body-class="bg-slate-100">
		<Container class="px-4 py-8">
			<Text class="m-0 text-xl font-semibold text-[#1867C0]">GR Logistics</Text>
			<Section class="mt-4 rounded-lg bg-white p-8">
				<slot />
			</Section>
			<Text class="mt-6 text-center text-xs text-slate-500">
				<Raw>{{ t.common.footer }}</Raw>
			</Text>
		</Container>
	</Layout>
</template>
