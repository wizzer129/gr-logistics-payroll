-- Outgoing email queue (EMAIL-SPEC.md §5). A handler enqueues a row in the same transaction
-- as the change that caused it; the outbox worker renders and sends it, with retries. Rows
-- are kept as the record of what was sent.
CREATE TABLE email_outbox (
	id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
	kind text NOT NULL
		CHECK (kind IN ('test_email', 'password_reset', 'paystub', 'monthly_report')),
	-- One address per row: recipients never see each other on a shared To: line.
	recipient text NOT NULL,
	locale text NOT NULL CHECK (locale IN ('en', 'es')),
	-- What to render, not the rendered email. Scrubbed to NULL once a password reset is
	-- sent or fails, since it carries a live token.
	payload jsonb,
	status text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'sent', 'failed')),
	attempts int NOT NULL DEFAULT 0,
	-- Also the claim lease: the worker pushes this forward while a row is being sent.
	next_attempt_at timestamptz NOT NULL DEFAULT now(),
	last_error text,
	subject text,
	requested_by uuid REFERENCES users (id),
	created_at timestamptz NOT NULL DEFAULT now(),
	sent_at timestamptz
);

CREATE INDEX email_outbox_pending ON email_outbox (next_attempt_at) WHERE status = 'pending';
