// RecipientSelect's value. Users and employees share one multi-select, so each item's key
// carries its kind: `user:<id>` or `employee:<id>`.
export type RecipientSelection = { userIds: string[]; employeeIds: string[] };

export type RecipientOptions = {
	users: { id: string; email: string }[];
	employees: { id: string; firstName: string; lastName: string; email: string }[];
};

export function toKeys(selection: RecipientSelection): string[] {
	return [
		...selection.userIds.map((id) => `user:${id}`),
		...selection.employeeIds.map((id) => `employee:${id}`),
	];
}

export function fromKeys(keys: string[]): RecipientSelection {
	const selection: RecipientSelection = { userIds: [], employeeIds: [] };
	for (const key of keys) {
		const [kind, id] = key.split(':');
		if (kind === 'user') selection.userIds.push(id);
		else if (kind === 'employee') selection.employeeIds.push(id);
	}
	return selection;
}
