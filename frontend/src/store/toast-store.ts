import { defineStore } from 'pinia';
import { ref } from 'vue';
import { ApiError } from '../utils/api';

export type ToastColor = 'success' | 'error' | 'info' | 'warning';

export type Toast = { text: string; color: ToastColor; timeout: number };

export const TOAST_TIMEOUT_MS = 3000;

/**
 * App-wide snackbar queue, rendered once by App.vue's `<v-snackbar-queue>`. Toasts show one
 * at a time in the order they were pushed; each closes after its timeout or when the user
 * dismisses it, and the queue removes it as it's shown. Text must already be translated.
 */
export const useToastStore = defineStore('toast', () => {
	const queue = ref<Toast[]>([]);

	function show(text: string, color: ToastColor = 'info', timeout = TOAST_TIMEOUT_MS): void {
		queue.value.push({ text, color, timeout });
	}

	/**
	 * For a failed API call's catch block: shows an `ApiError` (already translated, with its
	 * code) as an error toast. Anything else is a bug, not an API failure — rethrown so it
	 * still surfaces in the console.
	 */
	function fromError(error: unknown): void {
		if (!(error instanceof ApiError)) throw error;
		show(error.message, 'error');
	}

	return {
		queue,
		show,
		fromError,
		success: (text: string) => show(text, 'success'),
		error: (text: string) => show(text, 'error'),
		info: (text: string) => show(text, 'info'),
		warning: (text: string) => show(text, 'warning'),
	};
});
