import { afterEach, describe, expect, test, vi } from 'vitest';
import en from '../src/locales/en.json';
import es from '../src/locales/es.json';
import { i18n } from '../src/plugins/i18n';
import { ApiError, apiError, apiFetch, apiJson, apiRequest } from '../src/utils/api';
// Vite's `?raw` import, so the test needs no Node types (`tsconfig.json` has only Vite's).
import errorRs from '../../backend/src/error.rs?raw';

// The backend's `ErrorCode` variants, as their serialized SCREAMING_SNAKE_CASE names.
function backendErrorCodes(): string[] {
	const body = errorRs.match(/pub enum ErrorCode \{([^}]*)\}/)?.[1] ?? '';
	return [...body.matchAll(/^\s*([A-Z]\w*),/gm)].map(([, name]) =>
		name.replace(/(?<!^)([A-Z])/g, '_$1').toUpperCase(),
	);
}

function jsonResponse(status: number, body: unknown): Response {
	return new Response(JSON.stringify(body), { status });
}

describe('error codes', () => {
	test('every backend ErrorCode has a translation in every locale', () => {
		const codes = backendErrorCodes();
		expect(codes).toContain('EMPLOYEE_NOT_FOUND');
		for (const messages of [en.errors, es.errors]) {
			expect(Object.keys(messages)).toEqual(expect.arrayContaining(codes));
		}
	});
});

describe('apiError', () => {
	test('translates by code and appends the code', async () => {
		i18n.global.locale.value = 'es';
		const error = await apiError(
			jsonResponse(404, { error: 'Employee not found', code: 'EMPLOYEE_NOT_FOUND' }),
		);
		expect(error).toBeInstanceOf(ApiError);
		expect(error.code).toBe('EMPLOYEE_NOT_FOUND');
		expect(error.message).toBe('Empleado no encontrado. [EMPLOYEE_NOT_FOUND]');
		i18n.global.locale.value = 'en';
	});

	test('an unknown code keeps the code with a generic message', async () => {
		const error = await apiError(jsonResponse(400, { error: 'x', code: 'SOMETHING_NEW' }));
		expect(error.message).toBe('Unexpected server response. [SOMETHING_NEW]');
	});

	test('a body without a code falls back to the HTTP status', async () => {
		const error = await apiError(new Response('<html>Bad Gateway</html>', { status: 502 }));
		expect(error.code).toBe('HTTP_502');
		expect(error.message).toBe('Unexpected server response. [HTTP_502]');
	});
});

describe('apiFetch / apiRequest / apiJson', () => {
	afterEach(() => vi.unstubAllGlobals());

	test('a network failure is an ApiError with NETWORK_ERROR', async () => {
		vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new TypeError('Failed to fetch')));
		await expect(apiFetch('/api/x')).rejects.toMatchObject({
			code: 'NETWORK_ERROR',
			message: "Couldn't reach the server. Check your connection. [NETWORK_ERROR]",
		});
	});

	test('apiFetch resolves a non-2xx response; apiRequest throws it', async () => {
		const notFound = () =>
			jsonResponse(404, { error: 'Employee not found', code: 'EMPLOYEE_NOT_FOUND' });
		vi.stubGlobal(
			'fetch',
			vi.fn().mockImplementation(async () => notFound()),
		);

		expect((await apiFetch('/api/x')).status).toBe(404);
		await expect(apiRequest('/api/x')).rejects.toMatchObject({ code: 'EMPLOYEE_NOT_FOUND' });
	});

	test('apiJson returns the parsed body', async () => {
		vi.stubGlobal('fetch', vi.fn().mockResolvedValue(jsonResponse(200, { id: 'a' })));
		expect(await apiJson('/api/x')).toEqual({ id: 'a' });
	});
});
