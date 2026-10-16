import { i18n } from '../plugins/i18n';

// In production the API lives on its own origin (api.<domain>, the SPA on app.<domain> via
// Cloudflare Pages), so a bare '/api/...' would hit Pages itself. VITE_API_BASE_URL is that
// origin at build time; left empty in development so requests stay relative and go through
// the Vite dev proxy. The session cookie still rides along cross-origin: app. and api. are
// the same site (SameSite=Lax) and the backend's CORS allows credentials from CORS_ORIGIN.
const API_BASE_URL = (import.meta.env.VITE_API_BASE_URL ?? '').replace(/\/+$/, '');

export function apiUrl(path: string): string {
	return `${API_BASE_URL}${path}`;
}

/** The raw response, any status. A network failure (offline, DNS, CORS) throws `ApiError`. */
export async function apiFetch(path: string, init: RequestInit = {}): Promise<Response> {
	try {
		return await fetch(apiUrl(path), { ...init, credentials: 'include' });
	} catch {
		throw translatedError('NETWORK_ERROR');
	}
}

/** Like `apiFetch`, but a non-2xx response throws `ApiError` too. */
export async function apiRequest(path: string, init: RequestInit = {}): Promise<Response> {
	const response = await apiFetch(path, init);
	if (!response.ok) throw await apiError(response);
	return response;
}

/** `apiRequest` plus the parsed JSON body. */
export async function apiJson<T>(path: string, init: RequestInit = {}): Promise<T> {
	return (await apiRequest(path, init)).json();
}

/** A failed API call. `message` is translated and ends with `code` for troubleshooting. */
export class ApiError extends Error {
	constructor(
		readonly code: string,
		message: string,
	) {
		super(message);
		this.name = 'ApiError';
	}
}

/**
 * Builds the error for a non-2xx response from the backend's `{ error, code }` body.
 * `errors.<code>` is the translation; a response without a code (a proxy error page, say)
 * becomes `HTTP_<status>` with a generic message.
 */
export async function apiError(response: Response): Promise<ApiError> {
	const body = await response.json().catch(() => null);
	const code = typeof body?.code === 'string' ? body.code : `HTTP_${response.status}`;
	return translatedError(code);
}

function translatedError(code: string): ApiError {
	const { t, te } = i18n.global;
	const key = `errors.${code}`;
	return new ApiError(code, `${te(key) ? t(key) : t('errors.UNKNOWN')} [${code}]`);
}
