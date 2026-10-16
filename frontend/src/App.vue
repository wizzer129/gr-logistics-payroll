<script setup lang="ts">
import { useAuthStore } from './store/auth-store';
import { useToastStore } from './store/toast-store';
import NavBar from './components/layout/NavBar.vue';
import ConfirmDialog from './components/common/ConfirmDialog.vue';

const auth = useAuthStore();
const toast = useToastStore();
</script>

<template>
	<v-app>
		<!-- Admins only: every page in it needs the admin role (others get /no-access). -->
		<NavBar v-if="auth.isAdmin" />

		<v-main>
			<router-view />
		</v-main>

		<!-- The app's one snackbar: shows useToastStore()'s queue one toast at a time. -->
		<v-snackbar-queue v-model="toast.queue" location="top right" closable />
		<!-- The app's one confirmation dialog, driven by useConfirmStore().confirm(). -->
		<ConfirmDialog />
	</v-app>
</template>
