Fix v-date picker to allow week range visual
Maybe add way to save work.
Add table sorting and ability to email PDFs of payment reports to mom for printing
Update the login view design.
Show the logged-in user's name in the top right (users only have an email today — needs a name
column on `users` plus a migration, and `/api/auth/me` returning it).

Accounts & email (design: EMAIL-SPEC.md):

- Monthly report email (EMAIL-SPEC.md Phase 5): report query, `monthly-report.typ` PDF,
  the scheduler (1st of the month, 08:00 America/Los_Angeles) + `monthly_report_runs`,
  manual "Send now", and a "Sent emails" panel over `email_outbox`. Decide the report's
  contents first (spec §8). Phases 1–4 (outbox, password reset, recipients, paystub
  email) are built.

Backend (left over from the Rust rewrite):

- Auto-restart the server on deploy. CI pushes the image to GHCR, but updating the server is
  still manual: `cd ~/gr-logistics && docker compose pull && docker compose up -d` (see
  deploy/docker-compose.yml). Options: SSH from the deploy job, a self-hosted runner, or a
  webhook/poller on the server.
- Scheduled pg_dump backups of the production database (only the one-off cutover dump exists).
- Colored per-module log lines in pretty mode (TODO in backend/src/logging.rs).
- Optional: auto-reload for `bun run dev` (cargo install bacon, then dev:backend -> "cd backend && bacon run").
