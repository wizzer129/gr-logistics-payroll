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
});

export const loadSchema = z.object({
	storeName: z.string().min(1),
	loadDate: z.string().date(),
	pay: z.number().nonnegative(),
	extraPay: z.number().nonnegative().default(0),
});

export const weeklyLogSchema = z.object({
	employeeId: z.string().uuid(),
	weekStart: z.string().date(),
	weekEnd: z.string().date(),
	loads: z.array(loadSchema).min(1),
});
