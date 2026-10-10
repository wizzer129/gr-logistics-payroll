import { sql } from '../db/client';
import { weeklyLogSchema } from '../models/schemas';
import { calculateEmployeeCut, sumLoads } from '../utils/payment-calc';
import { recordAudit } from '../services/audit-service';
import { AuthUser, HttpError, requireRole } from '../middleware/auth';

export async function listWeeklyLogs(req: Request): Promise<Response> {
	const url = new URL(req.url);
	const employeeId = url.searchParams.get('employeeId');
	const from = url.searchParams.get('from');
	const to = url.searchParams.get('to');

	const logs = await sql`
		SELECT wl.id, wl.employee_id, wl.week_start, wl.week_end, wl.status, wl.summary_total, wl.employee_cut,
			employees.first_name, employees.last_name
		FROM employee_weekly_logs wl
		JOIN employees ON employees.id = wl.employee_id
		WHERE (${employeeId}::uuid IS NULL OR wl.employee_id = ${employeeId}::uuid)
		AND (${from}::date IS NULL OR wl.week_start >= ${from}::date)
		AND (${to}::date IS NULL OR wl.week_end <= ${to}::date)
		ORDER BY wl.week_start DESC
	`;

	return Response.json(logs);
}

export async function createWeeklyLog(req: Request, user: AuthUser): Promise<Response> {
	requireRole(user, 'admin');
	const body = weeklyLogSchema.parse(await req.json());
	const total = sumLoads(body.loads.map((l) => ({ pay: l.pay, extraPay: l.extraPay })));
	const { roundedPay } = calculateEmployeeCut(total);

	const log = await sql.begin(async (tx) => {
		const [weeklyLog] = await tx`
			INSERT INTO employee_weekly_logs
				(employee_id, week_start, week_end, status, summary_total, employee_cut, created_by, last_edited_by)
			VALUES
				(${body.employeeId}, ${body.weekStart}, ${body.weekEnd}, 'draft', ${total}, ${roundedPay}, ${user.id}, ${user.id})
			RETURNING id, employee_id, week_start, week_end, status, summary_total, employee_cut
		`;

		for (const load of body.loads) {
			await tx`
				INSERT INTO employee_loads
					(employee_id, weekly_log_id, store_name, load_date, pay, extra_pay, created_by, last_edited_by)
				VALUES
					(${body.employeeId}, ${weeklyLog.id}, ${load.storeName}, ${load.loadDate}, ${load.pay}, ${load.extraPay}, ${user.id}, ${user.id})
			`;
		}

		return weeklyLog;
	});

	await recordAudit({
		entity: 'employee_weekly_log',
		entityId: log.id,
		action: 'create',
		changedBy: user.id,
	});

	return Response.json(log, { status: 201 });
}

export async function completeWeeklyLog(user: AuthUser, id: string): Promise<Response> {
	requireRole(user, 'admin');

	const [log] = await sql`
		UPDATE employee_weekly_logs
		SET status = 'completed', last_edited_by = ${user.id}, updated_at = now()
		WHERE id = ${id}
		RETURNING id, status, summary_total, employee_cut
	`;

	if (!log) throw new HttpError(404, 'Weekly log not found');

	await recordAudit({
		entity: 'employee_weekly_log',
		entityId: id,
		action: 'update',
		changedBy: user.id,
		diff: { status: 'completed' },
	});

	return Response.json(log);
}

export async function getWeeklyLogLoads(id: string): Promise<Response> {
	const loads = await sql`
		SELECT id, store_name, load_date, pay, extra_pay
		FROM employee_loads
		WHERE weekly_log_id = ${id}
		ORDER BY load_date
	`;

	return Response.json(loads);
}
