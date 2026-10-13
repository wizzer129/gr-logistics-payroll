-- The rate each paycheck log's cut was computed at. Editing a log (even a completed one)
-- recomputes its cut at this rate, not the employee's current one, so a rate change never
-- silently re-rates history.
ALTER TABLE employee_paycheck_logs ADD COLUMN pay_rate_percent numeric(5, 2);

-- The rate wasn't recorded before, so existing logs take the employee's current rate —
-- the same rate an edit would have used until now.
UPDATE employee_paycheck_logs pl
SET pay_rate_percent = e.pay_rate_percent
FROM employees e
WHERE e.id = pl.employee_id;

-- Left nullable while the Bun backend (which doesn't know this column) still writes logs.
-- After cutover: backfill any NULLs the same way and SET NOT NULL.
