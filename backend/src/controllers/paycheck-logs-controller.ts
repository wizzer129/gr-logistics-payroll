import { sql } from '../db/client';
import { paycheckLogSchema, paycheckLogUpdateSchema } from '../models/schemas';
import { calculateEmployeeCut, sumLoads } from '../utils/payment-calc';
import { recordAudit } from '../services/audit-service';
import { resolveStoreId } from '../services/store-service';
import { toNum } from '../utils/sql-numeric';
import { AuthUser, HttpError, requireRole } from '../middleware/auth';

export async function listPaycheckLogs(req: Request): Promise<Response> {
	const url = new URL(req.url);
	const employeeId = url.searchParams.get('employeeId');
	const from = url.searchParams.get('from');
	const to = url.searchParams.get('to');

	const logs = await sql`
		SELECT pl.id, pl.employee_id AS "employeeId", pl.week_start AS "weekStart",
			pl.week_end AS "weekEnd", pl.payday, pl.status, pl.summary_total AS "summaryTotal",
			pl.employee_cut AS "employeeCut", employees.first_name AS "firstName",
			employees.last_name AS "lastName"
		FROM employee_paycheck_logs pl
		JOIN employees ON employees.id = pl.employee_id
		WHERE (${employeeId}::uuid IS NULL OR pl.employee_id = ${employeeId}::uuid)
		AND (${from}::date IS NULL OR pl.week_start >= ${from}::date)
		AND (${to}::date IS NULL OR pl.week_end <= ${to}::date)
		ORDER BY pl.week_start DESC
	`;

	return Response.json(
		logs.map((row: Record<string, unknown>) => ({
			...row,
			summaryTotal: toNum(row.summaryTotal),
			employeeCut: toNum(row.employeeCut),
		})),
	);
}

export async function createPaycheckLog(req: Request, user: AuthUser): Promise<Response> {
	requireRole(user, 'admin');
	const body = paycheckLogSchema.parse(await req.json());
	const total = sumLoads(body.loads.map((l) => ({ pay: l.pay, extraPay: l.extraPay })));

	const [employee] = await sql`
		SELECT pay_rate_percent AS "payRatePercent" FROM employees WHERE id = ${body.employeeId}
	`;
	if (!employee) throw new HttpError(404, 'Employee not found');

	const { roundedPay } = calculateEmployeeCut(total, toNum(employee.payRatePercent));

	const log = await sql.begin(async (tx) => {
		const [paycheckLog] = await tx`
			INSERT INTO employee_paycheck_logs
				(employee_id, week_start, week_end, payday, status, summary_total, employee_cut,
				 created_by, last_edited_by)
			VALUES
				(${body.employeeId}, ${body.weekStart}, ${body.weekEnd}, ${body.payday}, 'draft',
				 ${total}, ${roundedPay}, ${user.id}, ${user.id})
			RETURNING id, employee_id AS "employeeId", week_start AS "weekStart",
				week_end AS "weekEnd", payday, status, summary_total AS "summaryTotal",
				employee_cut AS "employeeCut"
		`;

		for (const load of body.loads) {
			const storeId = await resolveStoreId(load.storeName, user.id);
			await tx`
				INSERT INTO employee_loads
					(employee_id, paycheck_log_id, store_id, load_date, pay, extra_pay, created_by, last_edited_by)
				VALUES
					(${body.employeeId}, ${paycheckLog.id}, ${storeId}, ${load.loadDate}, ${load.pay}, ${load.extraPay}, ${user.id}, ${user.id})
			`;
		}

		return paycheckLog;
	});

	await recordAudit({
		entity: 'employee_paycheck_log',
		entityId: log.id,
		action: 'create',
		changedBy: user.id,
	});

	return Response.json(
		{ ...log, summaryTotal: toNum(log.summaryTotal), employeeCut: toNum(log.employeeCut) },
		{ status: 201 },
	);
}

export async function completePaycheckLog(user: AuthUser, id: string): Promise<Response> {
	requireRole(user, 'admin');

	const [log] = await sql`
		UPDATE employee_paycheck_logs
		SET status = 'completed', last_edited_by = ${user.id}, updated_at = now()
		WHERE id = ${id}
		RETURNING id, status, summary_total AS "summaryTotal", employee_cut AS "employeeCut"
	`;

	if (!log) throw new HttpError(404, 'Paycheck log not found');

	await recordAudit({
		entity: 'employee_paycheck_log',
		entityId: id,
		action: 'update',
		changedBy: user.id,
		diff: { status: 'completed' },
	});

	return Response.json({
		...log,
		summaryTotal: toNum(log.summaryTotal),
		employeeCut: toNum(log.employeeCut),
	});
}

export async function updatePaycheckLog(
	req: Request,
	user: AuthUser,
	id: string,
): Promise<Response> {
	requireRole(user, 'admin');
	const body = paycheckLogUpdateSchema.parse(await req.json());
	const total = sumLoads(body.loads.map((l) => ({ pay: l.pay, extraPay: l.extraPay })));

	const [existing] = await sql`
		SELECT pl.employee_id AS "employeeId", employees.pay_rate_percent AS "payRatePercent"
		FROM employee_paycheck_logs pl
		JOIN employees ON employees.id = pl.employee_id
		WHERE pl.id = ${id}
	`;
	if (!existing) throw new HttpError(404, 'Paycheck log not found');

	const { roundedPay } = calculateEmployeeCut(total, toNum(existing.payRatePercent));

	const log = await sql.begin(async (tx) => {
		const [paycheckLog] = await tx`
			UPDATE employee_paycheck_logs
			SET week_start = ${body.weekStart}, week_end = ${body.weekEnd}, payday = ${body.payday},
				summary_total = ${total}, employee_cut = ${roundedPay}, last_edited_by = ${user.id},
				updated_at = now()
			WHERE id = ${id}
			RETURNING id, employee_id AS "employeeId", week_start AS "weekStart",
				week_end AS "weekEnd", payday, status, summary_total AS "summaryTotal",
				employee_cut AS "employeeCut"
		`;

		await tx`DELETE FROM employee_loads WHERE paycheck_log_id = ${id}`;

		for (const load of body.loads) {
			const storeId = await resolveStoreId(load.storeName, user.id);
			await tx`
				INSERT INTO employee_loads
					(employee_id, paycheck_log_id, store_id, load_date, pay, extra_pay, created_by, last_edited_by)
				VALUES
					(${existing.employeeId}, ${id}, ${storeId}, ${load.loadDate}, ${load.pay}, ${load.extraPay}, ${user.id}, ${user.id})
			`;
		}

		return paycheckLog;
	});

	await recordAudit({
		entity: 'employee_paycheck_log',
		entityId: id,
		action: 'update',
		changedBy: user.id,
		diff: body,
	});

	return Response.json({
		...log,
		summaryTotal: toNum(log.summaryTotal),
		employeeCut: toNum(log.employeeCut),
	});
}

export async function deletePaycheckLog(user: AuthUser, id: string): Promise<Response> {
	requireRole(user, 'admin');

	// employee_loads.paycheck_log_id is ON DELETE CASCADE — loads go with the log.
	const [log] = await sql`
		DELETE FROM employee_paycheck_logs WHERE id = ${id} RETURNING id
	`;

	if (!log) throw new HttpError(404, 'Paycheck log not found');

	await recordAudit({
		entity: 'employee_paycheck_log',
		entityId: id,
		action: 'delete',
		changedBy: user.id,
	});

	return new Response(null, { status: 204 });
}

export async function getPaycheckLogLoads(id: string): Promise<Response> {
	const loads = await sql`
		SELECT employee_loads.id, stores.name AS "storeName", load_date AS "loadDate", pay,
			extra_pay AS "extraPay"
		FROM employee_loads
		JOIN stores ON stores.id = employee_loads.store_id
		WHERE paycheck_log_id = ${id}
		ORDER BY load_date
	`;

	return Response.json(
		loads.map((row: Record<string, unknown>) => ({
			...row,
			pay: toNum(row.pay),
			extraPay: toNum(row.extraPay),
		})),
	);
}
