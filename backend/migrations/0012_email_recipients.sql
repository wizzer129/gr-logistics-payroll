-- Recipients & language (EMAIL-SPEC.md §6). Email goes to existing users and employees,
-- each in their own language; users can opt in to the monthly report.
ALTER TABLE users
	ADD COLUMN locale text NOT NULL DEFAULT 'en' CHECK (locale IN ('en', 'es')),
	ADD COLUMN receives_monthly_report boolean NOT NULL DEFAULT false;

ALTER TABLE employees
	ADD COLUMN locale text NOT NULL DEFAULT 'en' CHECK (locale IN ('en', 'es'));
