import { defineStore } from 'pinia';
import { shallowRef } from 'vue';

export type ConfirmOptions = {
	title: string;
	text: string;
	/** Label for the confirm button; defaults to "Confirm". Text must already be translated. */
	confirmText?: string;
	/** Confirm button color, e.g. `'error'` for destructive actions. Defaults to primary. */
	color?: string;
};

type PendingConfirm = ConfirmOptions & { resolve: (confirmed: boolean) => void };

/**
 * App-wide confirmation dialog, rendered once by App.vue's `<ConfirmDialog>`:
 * `if (!(await useConfirmStore().confirm({ title, text }))) return;`
 */
export const useConfirmStore = defineStore('confirm', () => {
	const pending = shallowRef<PendingConfirm | null>(null);

	/** Resolves `true` on confirm, `false` on cancel, Escape or clicking outside. */
	function confirm(options: ConfirmOptions): Promise<boolean> {
		// Only one dialog at a time: a new request cancels one still open.
		pending.value?.resolve(false);
		return new Promise((resolve) => {
			pending.value = { ...options, resolve };
		});
	}

	function answer(confirmed: boolean): void {
		const current = pending.value;
		pending.value = null;
		current?.resolve(confirmed);
	}

	return { pending, confirm, answer };
});
