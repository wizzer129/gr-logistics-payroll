-- One row per month whose report the scheduler has queued (EMAIL-SPEC.md §8). The primary key
-- is what makes the automatic send exactly-once: a restart, a second instance or a missed hour
-- can't queue the same month twice. "Send now" doesn't touch this table.
CREATE TABLE monthly_report_runs (
	month date PRIMARY KEY CHECK (extract(day FROM month) = 1),
	outbox_ids uuid[] NOT NULL,
	created_at timestamptz NOT NULL DEFAULT now()
);
