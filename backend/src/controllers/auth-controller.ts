import { sql } from '../db/client';
import { loginSchema } from '../models/schemas';
import {
	clearSessionCookie,
	createSession,
	destroySession,
	readSessionId,
	setSessionCookie,
	verifyPassword,
} from '../services/auth-service';
import { HttpError } from '../middleware/auth';

export async function login(req: Request): Promise<Response> {
	const body = loginSchema.parse(await req.json());

	const [user] =
		await sql`SELECT id, email, role, password_hash FROM users WHERE email = ${body.email}`;
	if (!user) throw new HttpError(401, 'Invalid email or password');

	const valid = await verifyPassword(body.password, user.password_hash);
	if (!valid) throw new HttpError(401, 'Invalid email or password');

	const { sessionId, expiresAt } = await createSession(user.id);

	const headers = new Headers({ 'Content-Type': 'application/json' });
	setSessionCookie(headers, sessionId, expiresAt);

	return new Response(JSON.stringify({ id: user.id, email: user.email, role: user.role }), {
		headers,
	});
}

export async function logout(req: Request): Promise<Response> {
	const sessionId = readSessionId(req);
	if (sessionId) await destroySession(sessionId);

	const headers = new Headers();
	clearSessionCookie(headers);

	return new Response(null, { status: 204, headers });
}
