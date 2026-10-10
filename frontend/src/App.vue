<script setup lang="ts">
import { useAuthStore } from './store/auth-store';
import NavBar from './components/layout/NavBar.vue';

const auth = useAuthStore();
</script>

<template>
	<v-app>
		<NavBar v-if="auth.isAuthenticated" />

		<v-main>
			<router-view />
		</v-main>
	</v-app>
</template>
