import { sql } from '../db/client';
import { env } from '../utils/env';

const SESSION_COOKIE = 'gr_session';

export async function hashPassword(password: string): Promise<string> {
	return Bun.password.hash(password, { algorithm: 'argon2id' });
}

export async function verifyPassword(password: string, hash: string): Promise<boolean> {
	return Bun.password.verify(password, hash);
}

export async function createSession(
	userId: string,
): Promise<{ sessionId: string; expiresAt: Date }> {
	const expiresAt = new Date(Date.now() + env.sessionTtlHours * 60 * 60 * 1000);

	const [row] = await sql`
		INSERT INTO sessions (user_id, expires_at)
		VALUES (${userId}, ${expiresAt})
		RETURNING session_id
	`;

	return { sessionId: row.session_id, expiresAt };
}

export async function getUserBySessionId(sessionId: string) {
	const [row] = await sql`
		SELECT users.id, users.email, users.role
		FROM sessions
		JOIN users ON users.id = sessions.user_id
		WHERE sessions.session_id = ${sessionId}
		AND sessions.expires_at > now()
	`;

	return row ?? null;
}

export async function destroySession(sessionId: string): Promise<void> {
	await sql`DELETE FROM sessions WHERE session_id = ${sessionId}`;
}

export function readSessionId(req: Request): string | null {
	const cookies = new Bun.CookieMap(req.headers.get('cookie') ?? '');
	return cookies.get(SESSION_COOKIE) ?? null;
}

export function setSessionCookie(headers: Headers, sessionId: string, expiresAt: Date): void {
	const cookies = new Bun.CookieMap();
	cookies.set(SESSION_COOKIE, sessionId, {
		httpOnly: true,
		secure: env.nodeEnv === 'production',
		sameSite: 'lax',
		expires: expiresAt,
		path: '/',
	});

	for (const header of cookies.toSetCookieHeaders()) {
		headers.append('Set-Cookie', header);
	}
}

export function clearSessionCookie(headers: Headers): void {
	headers.append('Set-Cookie', `${SESSION_COOKIE}=; Path=/; Max-Age=0`);
}
