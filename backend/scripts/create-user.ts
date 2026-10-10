import { parseArgs } from 'node:util';
import { sql } from '../src/db/client';
import { hashPassword } from '../src/services/auth-service';
import { createLogger } from '../src/utils/logger';

const logger = createLogger('create-user');

const { values } = parseArgs({
	args: Bun.argv.slice(2),
	options: {
		email: { type: 'string' },
		password: { type: 'string' },
		role: { type: 'string', default: 'admin' },
	},
});

if (!values.email || !values.password) {
	console.error(
		'Usage: bun run create-user --email <email> --password <password> [--role admin]',
	);
	process.exit(1);
}

const passwordHash = await hashPassword(values.password);

const [user] = await sql`
	INSERT INTO users (email, password_hash, role)
	VALUES (${values.email}, ${passwordHash}, ${values.role})
	ON CONFLICT (email) DO UPDATE SET password_hash = EXCLUDED.password_hash, role = EXCLUDED.role
	RETURNING id, email, role
`;

logger.info('user created/updated', user);
process.exit(0);
