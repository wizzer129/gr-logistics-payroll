import { sql } from '../db/client';

export async function recordAudit(params: {
	entity: string;
	entityId: string;
	action: 'create' | 'update' | 'delete';
	changedBy: string;
	diff?: Record<string, unknown>;
}): Promise<void> {
	await sql`
		INSERT INTO audit_log (entity, entity_id, action, changed_by, diff)
		VALUES (${params.entity}, ${params.entityId}, ${params.action}, ${params.changedBy}, ${params.diff ?? {}})
	`;
}
