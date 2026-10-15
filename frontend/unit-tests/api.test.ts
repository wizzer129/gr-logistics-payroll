import { afterEach, describe, expect, test, vi } from 'vitest';

// api.ts reads VITE_API_BASE_URL once at module load, so each case stubs the env first and
// then imports a fresh copy of the module.
async function loadApi(baseUrl: string) {
	vi.stubEnv('VITE_API_BASE_URL', baseUrl);
	vi.resetModules();
	return import('../src/utils/api');
}

afterEach(() => {
	vi.unstubAllEnvs();
	vi.unstubAllGlobals();
});

describe('apiUrl', () => {
	test('an empty base URL keeps paths relative (Vite dev proxy)', async () => {
		const { apiUrl } = await loadApi('');
		expect(apiUrl('/api/employees')).toBe('/api/employees');
	});

	test('a base URL is prefixed to the path', async () => {
		const { apiUrl } = await loadApi('https://api.example.com');
		expect(apiUrl('/api/employees')).toBe('https://api.example.com/api/employees');
	});

	test('a trailing slash on the base URL does not double up', async () => {
		const { apiUrl } = await loadApi('https://api.example.com/');
		expect(apiUrl('/api/auth/me')).toBe('https://api.example.com/api/auth/me');
	});
});

describe('apiFetch', () => {
	test('always sends credentials and passes the rest of init through', async () => {
		const fetchMock = vi.fn().mockResolvedValue(new Response(null, { status: 204 }));
		vi.stubGlobal('fetch', fetchMock);
		const { apiFetch } = await loadApi('https://api.example.com');

		await apiFetch('/api/employees/1', { method: 'DELETE' });

		expect(fetchMock).toHaveBeenCalledWith('https://api.example.com/api/employees/1', {
			method: 'DELETE',
			credentials: 'include',
		});
	});
});
