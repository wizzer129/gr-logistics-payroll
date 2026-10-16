import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, test } from 'vitest';
import { TOAST_TIMEOUT_MS, useToastStore } from '../src/store/toast-store';
import { ApiError } from '../src/utils/api';

describe('toast store', () => {
	beforeEach(() => setActivePinia(createPinia()));

	test('queues toasts in order with the default 3s timeout', () => {
		const toast = useToastStore();
		toast.success('saved');
		toast.error('failed');

		expect(TOAST_TIMEOUT_MS).toBe(3000);
		expect(toast.queue).toEqual([
			{ text: 'saved', color: 'success', timeout: 3000 },
			{ text: 'failed', color: 'error', timeout: 3000 },
		]);
	});

	test('show() takes a custom color and timeout', () => {
		const toast = useToastStore();
		toast.show('heads up', 'warning', 8000);
		expect(toast.queue).toEqual([{ text: 'heads up', color: 'warning', timeout: 8000 }]);
	});

	test('fromError() toasts an ApiError and rethrows anything else', () => {
		const toast = useToastStore();
		toast.fromError(new ApiError('DUPLICATE_WEEK', 'Already paid. [DUPLICATE_WEEK]'));
		expect(toast.queue).toEqual([
			{ text: 'Already paid. [DUPLICATE_WEEK]', color: 'error', timeout: 3000 },
		]);

		const bug = new TypeError('x is undefined');
		expect(() => toast.fromError(bug)).toThrow(bug);
		expect(toast.queue).toHaveLength(1);
	});
});
