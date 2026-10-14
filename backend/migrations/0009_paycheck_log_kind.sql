-- A paycheck log either itemizes loads ('loads', every log until now) or records one
-- lump-sum payment for an employee whose loads aren't tracked ('lump_sum'). A lump sum
-- has no employee_loads rows; its summary_total and employee_cut are both the amount paid,
-- at a pay_rate_percent of 100.
ALTER TABLE employee_paycheck_logs
	ADD COLUMN kind text NOT NULL DEFAULT 'loads' CHECK (kind IN ('loads', 'lump_sum'));
