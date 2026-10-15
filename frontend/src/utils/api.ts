// In production the API lives on its own origin (api.<domain>, the SPA on app.<domain> via
// Cloudflare Pages), so a bare '/api/...' would hit Pages itself. VITE_API_BASE_URL is that
// origin at build time; left empty in development so requests stay relative and go through
// the Vite dev proxy. The session cookie still rides along cross-origin: app. and api. are
// the same site (SameSite=Lax) and the backend's CORS allows credentials from CORS_ORIGIN.
const API_BASE_URL = (import.meta.env.VITE_API_BASE_URL ?? '').replace(/\/+$/, '');

export function apiUrl(path: string): string {
	return `${API_BASE_URL}${path}`;
}

export function apiFetch(path: string, init: RequestInit = {}): Promise<Response> {
	return fetch(apiUrl(path), { ...init, credentials: 'include' });
}
