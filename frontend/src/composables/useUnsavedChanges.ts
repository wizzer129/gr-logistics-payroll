import { onMounted, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { onBeforeRouteLeave } from 'vue-router';
import { useConfirmStore } from '../store/confirm-store';

/**
 * Guards unsaved work behind the app's confirm dialog.
 *
 * - `confirmDiscard(isDirty)`: for closing a dialog; resolves `true` when it's clean or the
 *   user chose to discard.
 * - `guardLeave(isDirty, onDiscard?)`: call once in setup. Asks before routing away (and lets
 *   the browser ask before a refresh or tab close); `onDiscard` runs when the user leaves
 *   anyway, for state that outlives the view (e.g. a Pinia store).
 */
export function useUnsavedChanges() {
	const { t } = useI18n();
	const confirmDialog = useConfirmStore();

	async function confirmDiscard(isDirty: () => boolean): Promise<boolean> {
		if (!isDirty()) return true;
		return confirmDialog.confirm({
			title: t('unsaved.title'),
			text: t('unsaved.text'),
			confirmText: t('unsaved.discard'),
			color: 'warning',
		});
	}

	function guardLeave(isDirty: () => boolean, onDiscard?: () => void): void {
		onBeforeRouteLeave(async () => {
			if (!isDirty()) return true;
			const discard = await confirmDiscard(isDirty);
			if (discard) onDiscard?.();
			return discard;
		});

		// The browser shows its own (untranslatable) prompt; a custom dialog isn't allowed here.
		const onBeforeUnload = (event: BeforeUnloadEvent): void => {
			if (isDirty()) event.preventDefault();
		};
		onMounted(() => window.addEventListener('beforeunload', onBeforeUnload));
		onUnmounted(() => window.removeEventListener('beforeunload', onBeforeUnload));
	}

	return { confirmDiscard, guardLeave };
}
