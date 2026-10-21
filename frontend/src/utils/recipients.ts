// RecipientSelect's value. Users and employees share one multi-select, so each item's key
// carries its kind: `user:<id>` or `employee:<id>`. Addresses typed in by hand are kept as
// they are (EMAIL-SPEC.md §6).
export type RecipientSelection = { userIds: string[]; employeeIds: string[]; emails: string[] };

export type RecipientOptions = {
	users: { id: string; email: string }[];
	employees: { id: string; firstName: string; lastName: string; email: string }[];
};

export function emptySelection(): RecipientSelection {
	return { userIds: [], employeeIds: [], emails: [] };
}

/** Loose on purpose (the backend validates properly): something@something.tld, no spaces. */
export function isEmail(value: string): boolean {
	return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value.trim());
}

export function recipientCount(selection: RecipientSelection): number {
	return selection.userIds.length + selection.employeeIds.length + selection.emails.length;
}

export function toKeys(selection: RecipientSelection): string[] {
	return [
		...selection.userIds.map((id) => `user:${id}`),
		...selection.employeeIds.map((id) => `employee:${id}`),
		...selection.emails,
	];
}

/** Anything that isn't a `user:`/`employee:` key is a typed address. */
export function fromKeys(keys: string[]): RecipientSelection {
	const selection = emptySelection();
	for (const key of keys) {
		const [kind, id] = key.split(':');
		if (kind === 'user' && id) selection.userIds.push(id);
		else if (kind === 'employee' && id) selection.employeeIds.push(id);
		else if (!key.includes(':')) selection.emails.push(key.trim());
	}
	return selection;
}
