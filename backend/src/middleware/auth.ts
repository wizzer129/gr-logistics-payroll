import { getUserBySessionId, readSessionId } from '../services/auth-service';

export type AuthUser = { id: string; email: string; role: string };

export async function requireUser(req: Request): Promise<AuthUser> {
	const sessionId = readSessionId(req);
	if (!sessionId) throw new HttpError(401, 'Not authenticated');

	const user = await getUserBySessionId(sessionId);
	if (!user) throw new HttpError(401, 'Session expired or invalid');

	return user;
}

export function requireRole(user: AuthUser, ...roles: string[]): void {
	if (!roles.includes(user.role)) {
		throw new HttpError(403, 'Insufficient permissions');
	}
}

export class HttpError extends Error {
	constructor(
		public status: number,
		message: string,
	) {
		super(message);
	}
}
