import { mount } from '@vue/test-utils';
import { describe, expect, test } from 'vitest';
import LoadForm from '../src/components/payment-processing/LoadForm.vue';
import { i18n } from '../src/plugins/i18n';
import { vuetify } from '../src/plugins/vuetify';

describe('LoadForm isDirty', () => {
	test('is dirty while a load is typed in, clean again once it is added', async () => {
		const wrapper = mount(LoadForm, { global: { plugins: [vuetify, i18n] } });
		expect(wrapper.vm.isDirty).toBe(false);

		await wrapper.find('input').setValue('Store A');
		expect(wrapper.vm.isDirty).toBe(true);

		await wrapper.find('form').trigger('submit');
		expect(wrapper.emitted('add-load')?.[0]?.[0]).toMatchObject({ storeName: 'Store A' });
		expect(wrapper.vm.isDirty).toBe(false);
	});
});
