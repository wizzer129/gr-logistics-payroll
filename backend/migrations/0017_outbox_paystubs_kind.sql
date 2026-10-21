-- A `paystubs` email: several paycheck logs' pay stubs in one email, one PDF each (sent from
-- View Payments' bulk selection). Widens 0010's CHECK on email_outbox.kind.
ALTER TABLE email_outbox DROP CONSTRAINT email_outbox_kind_check;
ALTER TABLE email_outbox ADD CONSTRAINT email_outbox_kind_check
	CHECK (kind IN ('test_email', 'password_reset', 'paystub', 'paystubs', 'monthly_report'));
