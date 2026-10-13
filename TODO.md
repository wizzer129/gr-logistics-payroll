Fix v-date picker to allow week range visual
Add modals that protect users from routing out before they saved their work.
Maybe add way to save work.
Add AI assisted bulk payment processing (screen shot to payment preview)
Add table sorting and ability to email PDFs of payment reports to mom for printing

Backend (left over from the Rust rewrite):

- Cut the server over to the Rust image. Deploy doesn't restart the server yet, so it's manual:
    1. Add CORS_ORIGIN=<frontend origin> (exact origin, no "*") to the server's backend/.env.production, optionally LOG_FORMAT=json; SESSION_SECRET can go.
    2. docker compose pull
    3. docker compose run --rm backend migrate (applies 0006-0008)
    4. docker compose up -d backend, then check docker compose ps shows healthy and /api/health returns {"status":"ok"}
    5. Log in from the real frontend (existing sessions and passwords keep working)
       Rollback note: after 0008 (pay_rate_percent NOT NULL), the old Bun image can no longer create paycheck logs, so roll back to a pre-0008 Rust image instead.
- Try screenshot extraction with a real ANTHROPIC_API_KEY (only tested against a mock Claude API so far).
- Colored per-module log lines in pretty mode (TODO in backend/src/logging.rs).
- Optional: auto-reload for `bun run dev` (cargo install bacon, then dev:backend -> "cd backend && bacon run").
