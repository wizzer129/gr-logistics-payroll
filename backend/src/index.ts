import { assertEnv, env } from './utils/env';
import { createLogger } from './utils/logger';

const logger = createLogger('server');
import { HttpError, requireUser } from './middleware/auth';
import { login, logout, me } from './controllers/auth-controller';
import {
	createEmployee,
	deleteEmployee,
	listEmployees,
	updateEmployee,
} from './controllers/employees-controller';
import {
	completeWeeklyLog,
	createWeeklyLog,
	getWeeklyLogLoads,
	listWeeklyLogs,
} from './controllers/weekly-logs-controller';
import { extractLoad } from './controllers/load-extraction-controller';

assertEnv();

function withCors(response: Response): Response {
	response.headers.set(
		'Access-Control-Allow-Origin',
		env.nodeEnv === 'production' ? 'https://app.example.com' : '*',
	);
	response.headers.set('Access-Control-Allow-Credentials', 'true');
	return response;
}

async function handle(req: Request, handler: () => Promise<Response>): Promise<Response> {
	const start = Date.now();

	try {
		const response = await handler();
		logger.info('request handled', {
			method: req.method,
			path: new URL(req.url).pathname,
			status: response.status,
			durationMs: Date.now() - start,
		});
		return withCors(response);
	} catch (error) {
		if (error instanceof HttpError) {
			logger.warn('request failed', {
				method: req.method,
				path: new URL(req.url).pathname,
				status: error.status,
				message: error.message,
			});
			return withCors(Response.json({ error: error.message }, { status: error.status }));
		}

		logger.error('unhandled error', {
			method: req.method,
			path: new URL(req.url).pathname,
			error: String(error),
		});
		return withCors(Response.json({ error: 'Internal server error' }, { status: 500 }));
	}
}

const server = Bun.serve({
	port: env.port,
	routes: {
		'/api/auth/login': {
			POST: (req) => handle(req, () => login(req)),
		},
		'/api/auth/logout': {
			POST: (req) => handle(req, () => logout(req)),
		},
		'/api/auth/me': {
			GET: (req) => handle(req, async () => me(await requireUser(req))),
		},

		'/api/employees': {
			GET: (req) => handle(req, () => listEmployees()),
			POST: (req) => handle(req, async () => createEmployee(req, await requireUser(req))),
		},
		'/api/employees/:id': {
			PUT: (req) =>
				handle(req, async () => updateEmployee(req, await requireUser(req), req.params.id)),
			DELETE: (req) =>
				handle(req, async () => deleteEmployee(await requireUser(req), req.params.id)),
		},

		'/api/weekly-logs': {
			GET: (req) => handle(req, () => listWeeklyLogs(req)),
			POST: (req) => handle(req, async () => createWeeklyLog(req, await requireUser(req))),
		},
		'/api/weekly-logs/:id/complete': {
			POST: (req) =>
				handle(req, async () => completeWeeklyLog(await requireUser(req), req.params.id)),
		},
		'/api/weekly-logs/:id/loads': {
			GET: (req) => handle(req, () => getWeeklyLogLoads(req.params.id)),
		},

		'/api/loads/extract': {
			POST: (req) =>
				handle(req, async () => {
					await requireUser(req);
					return extractLoad(req);
				}),
		},

		'/api/health': {
			GET: () => Response.json({ status: 'ok' }),
		},
	},
	fetch() {
		return Response.json({ error: 'Not found' }, { status: 404 });
	},
});

logger.info(`backend listening on port ${server.port}`);
