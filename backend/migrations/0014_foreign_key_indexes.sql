-- Postgres doesn't index foreign keys on its own. Without these, every lookup by these
-- columns reads the whole table:
-- - a log's loads (the edit dialog, replacing loads on every edit, ON DELETE CASCADE, and the
--   load counts in the dashboard and the monthly report);
-- - a user's sessions (logging out everywhere after a password reset, ON DELETE CASCADE).
CREATE INDEX employee_loads_paycheck_log_id ON employee_loads (paycheck_log_id);
CREATE INDEX sessions_user_id ON sessions (user_id);
