<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { useDisplay } from 'vuetify';

export type Employee = {
	id: string;
	firstName: string;
	lastName: string;
	email?: string;
	payRatePercent: number;
	locale: 'en' | 'es';
};

const { t } = useI18n();
// A phone can't fit five columns: each employee becomes a two-line list row instead.
const { xs } = useDisplay();
defineProps<{ employees: Employee[] }>();
const emit = defineEmits<{ edit: [employee: Employee]; delete: [employee: Employee] }>();
</script>

<template>
	<v-list v-if="xs" lines="two">
		<v-list-item
			v-for="employee in employees"
			:key="employee.id"
			:title="`${employee.firstName} ${employee.lastName}`"
			:subtitle="[employee.email, `${employee.payRatePercent}%`].filter(Boolean).join(' · ')"
		>
			<template #append>
				<v-btn
					icon="mdi-pencil"
					size="small"
					variant="text"
					:aria-label="t('common.edit')"
					@click="emit('edit', employee)"
				/>
				<v-btn
					icon="mdi-delete"
					size="small"
					variant="text"
					:aria-label="t('common.delete')"
					@click="emit('delete', employee)"
				/>
			</template>
		</v-list-item>
	</v-list>
	<v-table v-else>
		<thead>
			<tr>
				<th>{{ t('employees.firstName') }}</th>
				<th>{{ t('employees.lastName') }}</th>
				<th>{{ t('auth.email') }}</th>
				<th>{{ t('employees.payRatePercent') }}</th>
				<th />
			</tr>
		</thead>
		<tbody>
			<tr v-for="employee in employees" :key="employee.id">
				<td>{{ employee.firstName }}</td>
				<td>{{ employee.lastName }}</td>
				<td>{{ employee.email }}</td>
				<td>{{ employee.payRatePercent }}%</td>
				<td>
					<v-btn
						icon="mdi-pencil"
						size="small"
						variant="text"
						:aria-label="t('common.edit')"
						@click="emit('edit', employee)"
					/>
					<v-btn
						icon="mdi-delete"
						size="small"
						variant="text"
						:aria-label="t('common.delete')"
						@click="emit('delete', employee)"
					/>
				</td>
			</tr>
		</tbody>
	</v-table>
</template>
