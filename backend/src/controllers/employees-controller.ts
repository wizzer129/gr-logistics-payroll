import { sql } from '../db/client';
import { employeeSchema } from '../models/schemas';
import { recordAudit } from '../services/audit-service';
import { AuthUser, requireRole } from '../middleware/auth';
import { HttpError } from '../middleware/auth';

export async function listEmployees(): Promise<Response> {
	const employees = await sql`
		SELECT id, first_name, last_name, dob, email, address, active
		FROM employees
		WHERE active = true
		ORDER BY last_name, first_name
	`;

	return Response.json(employees);
}

export async function createEmployee(req: Request, user: AuthUser): Promise<Response> {
	requireRole(user, 'admin');
	const body = employeeSchema.parse(await req.json());

	const [employee] = await sql`
		INSERT INTO employees (first_name, last_name, dob, email, address, created_by, last_edited_by)
		VALUES (${body.firstName}, ${body.lastName}, ${body.dob ?? null}, ${body.email ?? null}, ${body.address ?? null}, ${user.id}, ${user.id})
		RETURNING id, first_name, last_name, dob, email, address, active
	`;

	await recordAudit({
		entity: 'employee',
		entityId: employee.id,
		action: 'create',
		changedBy: user.id,
		diff: body,
	});

	return Response.json(employee, { status: 201 });
}

export async function updateEmployee(req: Request, user: AuthUser, id: string): Promise<Response> {
	requireRole(user, 'admin');
	const body = employeeSchema.partial().parse(await req.json());

	const [employee] = await sql`
		UPDATE employees
		SET
			first_name = COALESCE(${body.firstName ?? null}, first_name),
			last_name = COALESCE(${body.lastName ?? null}, last_name),
			dob = COALESCE(${body.dob ?? null}, dob),
			email = COALESCE(${body.email ?? null}, email),
			address = COALESCE(${body.address ?? null}, address),
			last_edited_by = ${user.id},
			updated_at = now()
		WHERE id = ${id}
		RETURNING id, first_name, last_name, dob, email, address, active
	`;

	if (!employee) throw new HttpError(404, 'Employee not found');

	await recordAudit({
		entity: 'employee',
		entityId: id,
		action: 'update',
		changedBy: user.id,
		diff: body,
	});

	return Response.json(employee);
}

export async function deleteEmployee(user: AuthUser, id: string): Promise<Response> {
	requireRole(user, 'admin');

	const [employee] = await sql`
		UPDATE employees
		SET active = false, last_edited_by = ${user.id}, updated_at = now()
		WHERE id = ${id}
		RETURNING id
	`;

	if (!employee) throw new HttpError(404, 'Employee not found');

	await recordAudit({ entity: 'employee', entityId: id, action: 'delete', changedBy: user.id });

	return new Response(null, { status: 204 });
}
