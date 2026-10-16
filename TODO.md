Fix v-date picker to allow week range visual
Add modals that protect users from routing out before they saved their work.
Maybe add way to save work.
Add table sorting and ability to email PDFs of payment reports to mom for printing
Update the login view design.
Show the logged-in user's name in the top right (users only have an email today — needs a name
column on `users` plus a migration, and `/api/auth/me` returning it).

Accounts & email:

- Email service (backend): sends paystubs and password-reset emails. Templates built with
  Maizzle. Pick a sending provider and add its credentials to backend/.env.schema +
  config.rs (REQUIRED/OPTIONAL_KEYS) and deploy/backend.env.example.
- Password reset: "Forgot password?" on the login view → emailed one-time link → set a new
  password. Needs a reset-token table (hashed token, expiry, single use) and invalidating the
  user's existing sessions on reset. Don't reveal whether an email has an account.
- Email recipients setting: a settings page where a user manages their preferred email targets
  (e.g. where paystubs/reports get sent). Stored per user in a new table (migration), with
  add/remove endpoints behind the usual role check.

Backend (left over from the Rust rewrite):

- Auto-restart the server on deploy. CI pushes the image to GHCR, but updating the server is
  still manual: `cd ~/gr-logistics && docker compose pull && docker compose up -d` (see
  deploy/docker-compose.yml). Options: SSH from the deploy job, a self-hosted runner, or a
  webhook/poller on the server.
- Scheduled pg_dump backups of the production database (only the one-off cutover dump exists).
- Colored per-module log lines in pretty mode (TODO in backend/src/logging.rs).
- Optional: auto-reload for `bun run dev` (cargo install bacon, then dev:backend -> "cd backend && bacon run").
