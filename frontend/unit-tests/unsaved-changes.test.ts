import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, test, vi } from 'vitest';
import { defineComponent, h, ref } from 'vue';
import { createMemoryHistory, createRouter, RouterView } from 'vue-router';
import { useUnsavedChanges } from '../src/composables/useUnsavedChanges';
import { i18n } from '../src/plugins/i18n';
import { useConfirmStore } from '../src/store/confirm-store';

const dirty = ref(false);
const onDiscard = vi.fn();

const Guarded = defineComponent({
	setup() {
		useUnsavedChanges().guardLeave(() => dirty.value, onDiscard);
		return () => h('div', 'guarded');
	},
});

async function setup() {
	const pinia = createPinia();
	setActivePinia(pinia);
	const router = createRouter({
		history: createMemoryHistory(),
		routes: [
			{ path: '/form', component: Guarded },
			{ path: '/elsewhere', component: { render: () => h('div', 'elsewhere') } },
		],
	});
	await router.push('/form');
	mount(RouterView, { global: { plugins: [router, pinia, i18n] } });
	await flushPromises();
	return { router, confirm: useConfirmStore() };
}

describe('useUnsavedChanges().guardLeave', () => {
	beforeEach(() => {
		dirty.value = false;
		onDiscard.mockClear();
	});

	test('a clean view leaves without asking', async () => {
		const { router, confirm } = await setup();
		await router.push('/elsewhere');
		expect(confirm.pending).toBeNull();
		expect(router.currentRoute.value.path).toBe('/elsewhere');
	});

	test('a dirty view asks; staying keeps the route and the work', async () => {
		const { router, confirm } = await setup();
		dirty.value = true;

		const navigation = router.push('/elsewhere');
		await flushPromises();
		expect(confirm.pending?.title).toBe('Discard unsaved changes?');

		confirm.answer(false);
		await navigation;
		expect(router.currentRoute.value.path).toBe('/form');
		expect(onDiscard).not.toHaveBeenCalled();
	});

	test('a dirty view asks; discarding leaves and runs onDiscard', async () => {
		const { router, confirm } = await setup();
		dirty.value = true;

		const navigation = router.push('/elsewhere');
		await flushPromises();
		confirm.answer(true);
		await navigation;
		expect(router.currentRoute.value.path).toBe('/elsewhere');
		expect(onDiscard).toHaveBeenCalledOnce();
	});
});
