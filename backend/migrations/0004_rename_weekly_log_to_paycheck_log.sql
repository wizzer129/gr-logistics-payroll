-- employee_weekly_logs is really an employee's paycheck record (one per pay period,
-- not strictly one calendar week), and it was missing the payday itself.
ALTER TABLE employee_weekly_logs RENAME TO employee_paycheck_logs;
ALTER TABLE employee_loads RENAME COLUMN weekly_log_id TO paycheck_log_id;

ALTER TABLE employee_paycheck_logs ADD COLUMN payday date NOT NULL;
