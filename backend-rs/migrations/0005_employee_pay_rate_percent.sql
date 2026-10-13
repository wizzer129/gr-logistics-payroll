-- Not every employee is cut at the standard 42% — Helbert Cervera is paid at 40%. The rate
-- needs to live per-employee instead of the app-wide EMPLOYEE_CUT_RATE constant.
ALTER TABLE employees ADD COLUMN pay_rate_percent numeric(5, 2) NOT NULL DEFAULT 42.00;
