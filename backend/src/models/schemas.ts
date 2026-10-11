import { z } from 'zod';

export const loginSchema = z.object({
	email: z.string().email(),
	password: z.string().min(1),
});

export const employeeSchema = z.object({
	firstName: z.string().min(1),
	lastName: z.string().min(1),
	dob: z.string().date().optional(),
	email: z.string().email().optional(),
	address: z.string().optional(),
	payRatePercent: z.number().positive().max(100).optional(),
});

export const loadSchema = z.object({
	storeName: z.string().min(1),
	loadDate: z.string().date(),
	pay: z.number().nonnegative(),
	// Extra pay is optional on input — omitted/undefined defaults to 0.
	extraPay: z.number().nonnegative().optional().default(0),
});

export const paycheckLogSchema = z.object({
	employeeId: z.string().uuid(),
	weekStart: z.string().date(),
	weekEnd: z.string().date(),
	payday: z.string().date(),
	loads: z.array(loadSchema).min(1),
});

// Editing a log replaces its week range, payday, and full set of loads — the employee it
// belongs to is immutable, so it's omitted here rather than taken from the request body.
export const paycheckLogUpdateSchema = paycheckLogSchema.omit({ employeeId: true });
