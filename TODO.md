Fix v-date picker to allow week range visual
Maybe add way to save work.
Add table sorting and ability to email PDFs of payment reports to mom for printing
Update the login view design.
Show the logged-in user's name in the top right (users only have an email today — needs a name
column on `users` plus a migration, and `/api/auth/me` returning it).

Backend (left over from the Rust rewrite):

- Auto-restart the server on deploy. CI pushes the image to GHCR, but updating the server is
  still manual: `cd ~/gr-logistics && docker compose pull && docker compose up -d` (see
  deploy/docker-compose.yml). Options: SSH from the deploy job, a self-hosted runner, or a
  webhook/poller on the server.
- Scheduled pg_dump backups of the production database (only the one-off cutover dump exists).
- Colored per-module log lines in pretty mode (TODO in backend/src/logging.rs).
- Optional: auto-reload for `bun run dev` (cargo install bacon, then dev:backend -> "cd backend && bacon run").

UI fixes (mobile spacing):

- [x] Edit-payment dialog: fullscreen on `xs`, wrap its 5 action buttons, drop the nested load
      form card's double padding.
- [x] Wide tables (Edit Payment, Employees, Sent emails, Loads) scroll sideways on phones: show
      stacked list rows below `sm` instead.
- [x] App bar is crowded on mobile (title, language select with a `mt-5` hack, theme, Logout):
      move language/theme/logout into the drawer on mobile.
- [x] Payment Processing week-range label uses `mt-n4 mb-2`: make it the field's persistent hint.
- [x] Edit Payment filters stack 4 tall full-width fields: dense row, From/To side by side.
- [x] Settings: first card's `mb-4` doubles the gap below it when stacked.
- [x] Dashboard: stat cards stack full width (use a 2×2 grid on `xs`); `mt-2` vs `mt-4` between
      sections.
- [x] Payment summary rows have no vertical gap (Payment Processing and the edit dialog); stack
      its buttons full width on `xs`.
- [x] Employees: add form sits on the bare page with no card/heading separating it from the list.
- [x] LoadList wraps its table in `pa-2`, unlike every other table.
- [x] LoadForm submit button: no top margin, not full width on mobile.
- [x] Load entry folder tabs overflow at ~360px in Spanish: tighter tab padding below `sm`.
- [x] Login / No Access: card `pa-4` plus card-text padding is ~32px inside a phone screen.
- [x] Paystub preview: 24px "desk" padding is a lot of a phone's width; 8px on `xs`.
- [x] Payment history only renders at `lgAndUp`: show it inline below the form on smaller screens.
