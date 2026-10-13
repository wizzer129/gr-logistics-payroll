-- 0007 left pay_rate_percent nullable while the Bun backend could still write logs without
-- it. Bun is gone, so every log must carry the rate its cut was computed at. Fill any rows
-- written in between the same way 0007 did, then require it.
UPDATE employee_paycheck_logs pl
SET pay_rate_percent = e.pay_rate_percent
FROM employees e
WHERE e.id = pl.employee_id
AND pl.pay_rate_percent IS NULL;

ALTER TABLE employee_paycheck_logs ALTER COLUMN pay_rate_percent SET NOT NULL;
