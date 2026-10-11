import { sql } from '../db/client';
import { HttpError } from '../middleware/auth';
import { toNum } from '../utils/sql-numeric';

function resolveYear(url: URL): number {
	const raw = url.searchParams.get('year');
	if (!raw) return new Date().getFullYear();

	const year = Number(raw);
	if (!Number.isInteger(year) || year < 2000 || year > 2100) {
		throw new HttpError(400, 'Invalid year');
	}
	return year;
}

export async function getDashboardSummary(req: Request): Promise<Response> {
	const year = resolveYear(new URL(req.url));

	const [totals] = await sql`
		SELECT COALESCE(SUM(employee_cut), 0) AS "ytdTotal",
			COALESCE(SUM(summary_total), 0) AS "ytdGross", COUNT(*) AS "paycheckCount"
		FROM employee_paycheck_logs
		WHERE status = 'completed' AND extract(year from week_start) = ${year}
	`;

	const [{ totalLoads }] = await sql`
		SELECT COUNT(*) AS "totalLoads"
		FROM employee_loads el
		JOIN employee_paycheck_logs pl ON pl.id = el.paycheck_log_id
		WHERE pl.status = 'completed' AND extract(year from pl.week_start) = ${year}
	`;

	const byEmployee = await sql`
		SELECT e.id AS "employeeId", e.first_name AS "firstName", e.last_name AS "lastName",
			SUM(pl.employee_cut) AS total
		FROM employees e
		JOIN employee_paycheck_logs pl ON pl.employee_id = e.id
		WHERE pl.status = 'completed' AND extract(year from pl.week_start) = ${year}
		GROUP BY e.id, e.first_name, e.last_name
		ORDER BY total DESC
	`;

	const byMonth = await sql`
		SELECT to_char(date_trunc('month', pl.week_start), 'YYYY-MM') AS period,
			e.id AS "employeeId", e.first_name AS "firstName", e.last_name AS "lastName",
			SUM(pl.employee_cut) AS total
		FROM employee_paycheck_logs pl
		JOIN employees e ON e.id = pl.employee_id
		WHERE pl.status = 'completed' AND extract(year from pl.week_start) = ${year}
		GROUP BY period, e.id, e.first_name, e.last_name
		ORDER BY period
	`;

	return Response.json({
		year,
		ytdTotal: toNum(totals.ytdTotal),
		ytdGross: toNum(totals.ytdGross),
		paycheckCount: Number(totals.paycheckCount),
		totalLoads: Number(totalLoads),
		byEmployee: byEmployee.map((row: Record<string, unknown>) => ({
			...row,
			total: toNum(row.total),
		})),
		byMonth: byMonth.map((row: Record<string, unknown>) => ({
			...row,
			total: toNum(row.total),
		})),
	});
}

export async function getEmployeeDashboardSummary(
	req: Request,
	employeeId: string,
): Promise<Response> {
	const year = resolveYear(new URL(req.url));

	const [employee] = await sql`
		SELECT id, first_name AS "firstName", last_name AS "lastName"
		FROM employees
		WHERE id = ${employeeId}
	`;
	if (!employee) throw new HttpError(404, 'Employee not found');

	const [{ ytdTotal }] = await sql`
		SELECT COALESCE(SUM(employee_cut), 0) AS "ytdTotal"
		FROM employee_paycheck_logs
		WHERE employee_id = ${employeeId} AND status = 'completed'
		AND extract(year from week_start) = ${year}
	`;

	const logs = await sql`
		SELECT id, week_start AS "weekStart", week_end AS "weekEnd", payday, status,
			summary_total AS "summaryTotal", employee_cut AS "employeeCut"
		FROM employee_paycheck_logs
		WHERE employee_id = ${employeeId} AND extract(year from week_start) = ${year}
		ORDER BY week_start
	`;

	return Response.json({
		year,
		employee,
		ytdTotal: toNum(ytdTotal),
		logs: logs.map((row: Record<string, unknown>) => ({
			...row,
			summaryTotal: toNum(row.summaryTotal),
			employeeCut: toNum(row.employeeCut),
		})),
	});
}
