<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';

// A password input with a show/hide eye button. Other attributes (label, rules,
// autocomplete…) pass straight through to the v-text-field.
const value = defineModel<string>({ required: true });
const { t } = useI18n();
const show = ref(false);
</script>

<template>
	<v-text-field v-model="value" :type="show ? 'text' : 'password'">
		<template #append-inner>
			<v-btn
				:icon="show ? 'mdi-eye-off' : 'mdi-eye'"
				:aria-label="t(show ? 'auth.hidePassword' : 'auth.showPassword')"
				variant="text"
				density="compact"
				size="small"
				@click="show = !show"
			/>
		</template>
	</v-text-field>
</template>
