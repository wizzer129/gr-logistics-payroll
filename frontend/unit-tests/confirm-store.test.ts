import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, test } from 'vitest';
import { useConfirmStore } from '../src/store/confirm-store';

describe('confirm store', () => {
	beforeEach(() => setActivePinia(createPinia()));

	test('resolves true on confirm and closes', async () => {
		const store = useConfirmStore();
		const result = store.confirm({ title: 'Delete?', text: 'Sure?' });
		expect(store.pending?.title).toBe('Delete?');

		store.answer(true);
		expect(await result).toBe(true);
		expect(store.pending).toBeNull();
	});

	test('resolves false on cancel', async () => {
		const store = useConfirmStore();
		const result = store.confirm({ title: 'Delete?', text: 'Sure?' });
		store.answer(false);
		expect(await result).toBe(false);
	});

	test('a new request cancels the one still open', async () => {
		const store = useConfirmStore();
		const first = store.confirm({ title: 'First', text: '' });
		const second = store.confirm({ title: 'Second', text: '' });

		expect(await first).toBe(false);
		expect(store.pending?.title).toBe('Second');
		store.answer(true);
		expect(await second).toBe(true);
	});
});
