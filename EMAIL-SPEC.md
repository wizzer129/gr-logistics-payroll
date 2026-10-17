# Email Service — Spec

Status: **Phases 1–4 built** (foundation, password reset, recipients, paystub email); Phase 5 (monthly report) to do (§13). Covers the three
"Accounts & email" items in [TODO.md](./TODO.md) (email service, password reset, email
recipients) plus emailed paystubs and monthly reports. Conventions in [AGENTS.md](./AGENTS.md) apply throughout
(error codes, i18n, `services/` owns SQL, explicit roles, money as `Decimal`, …).

## 1. Goals

| Email          | Trigger                                                  | To                                              | Attachment  |
| -------------- | -------------------------------------------------------- | ----------------------------------------------- | ----------- |
| Password reset | "Forgot password?" on the login view                     | the account's email                             | —           |
| Paystub        | "Email paystub" on a completed paycheck log              | users and/or employees picked in a multi-select | paystub PDF |
| Monthly report | Automatically on the 1st (previous month), or "Send now" | the users chosen as report recipients           | report PDF  |

Recipients are always **existing records**: users (the `users` table) and employees with an
email. There's no free-text "send to any address" box, which keeps typos out of payroll
email.

Non-goals: marketing/bulk email, receiving email.

## 2. Architecture at a glance

```
  emails/ (Bun workspace, Maizzle)        backend/ (Rust)
  ┌──────────────────────────┐   build   ┌────────────────────────────────────────────┐
  │ emails/*.vue             │ ────────► │ templates/email/*.html, *.txt  (committed, │
  │ Tailwind → inlined CSS   │           │   embedded with include_dir!)              │
  └──────────────────────────┘           │                                            │
                                         │ handlers/email.rs  ── the email controller │
  frontend ── POST /api/... ───────────► │ handlers/auth.rs   ── password reset       │
                                         │        │ enqueue (same tx as the change)   │
                                         │        ▼                                   │
                                         │ email_outbox table ──► outbox worker task  │
                                         │                         │ render (minijinja│
                                         │                         │ + typst for PDFs)│
                                         │                         ▼                  │
                                         │                   EmailSender trait        │
                                         └──────────────────────────┬─────────────────┘
                                                                    ▼
                                                  Resend (REST API, free plan)
```

Three ideas carry the design:

1. **Maizzle is build-time only.** It turns Tailwind-styled templates into email-safe HTML
   (inlined CSS, table layouts) plus a plain-text version. The Rust binary never runs Node;
   it embeds the built output and fills in placeholders at runtime.
2. **Nothing is sent inside a request.** Handlers write a row to an `email_outbox` table in
   the same transaction as the change that caused it. A background task sends it with
   retries. Requests stay fast, a provider outage doesn't fail the user's action, and the
   outbox doubles as the record of what was sent ("did the paystub go out?").
3. **The provider sits behind a trait**, so dev and tests never touch the network and
   switching providers is one new impl.

## 3. Templates: Maizzle → Rust

### Layout

Maizzle **6** (Vite + Vue + Tailwind 4): templates are Vue single-file components built on
Maizzle's own components (`Layout`, `Container`, `Section`, `Text`, `Heading`, `Button`, …).

```
emails/                        # Bun workspace (root package.json "workspaces")
  package.json                 # @maizzle/framework — scripts: dev (preview server), build
  maizzle.config.ts            # output → ../backend/templates/email, plaintext on
  components/EmailShell.vue    # shared frame: header, white card, footer (auto-imported)
  emails/                      # one .vue per email
    test-email.vue             # Phase 1 (`send-test-email`)
    password-reset.vue  paystub.vue  monthly-report.vue   # Phases 2, 4, 5

backend/templates/email/       # Maizzle build output — committed, never hand-edited
  test-email.html  test-email.txt   …one .html + .txt pair per email
```

Root scripts: `emails:dev` (Maizzle preview server) and `emails:build`.

### Placeholders

Vue evaluates `{{ }}` itself, so runtime placeholders go inside Maizzle's `<Raw>` component,
which passes its content through verbatim. In a plain attribute (`href="{{ url }}"`, not
`:href`), Vue leaves the braces alone already. The output's literal `{{ }}` are then
[minijinja](https://docs.rs/minijinja) expressions in Rust:

```vue
<!-- emails/emails/password-reset.vue -->
<Text><Raw>{{ t.password_reset.greeting }}</Raw></Text>
<Button href="{{ reset_url }}"><Raw>{{ t.password_reset.button }}</Raw></Button>
```

Gotcha: never write the Raw tag inside an HTML comment in a template. Maizzle's raw-extract
step matches it there too and breaks the file.

minijinja auto-escapes by file extension: `.html` templates are HTML-escaped, so employee
or store names can't inject markup, and `.txt` templates are left as-is. Templates are
registered under their real file names to keep that behavior. Undefined variables are an
error (`UndefinedBehavior::Strict`), so a missing locale key fails the render instead of
sending a blank.

**Plain text** comes from Maizzle's `plaintext` option (`string-strip-html`). Its `cb` hook
in `maizzle.config.ts` turns block tags into line breaks. Without it, every paragraph runs
together on one line.

### Why commit the build output

The backend's Docker build context is `backend/` and its builder image has no Bun. Committing
`backend/templates/email/` keeps the Rust build self-contained (like `migrations/`, embedded
with `include_dir!`). The CI `lint` job runs `bun run emails:build`, then fails if
`git status --porcelain backend/templates/email` shows anything. That's `status`, not
`diff`, so a new untracked template counts too. The build was checked to be byte-for-byte
deterministic.

`backend/templates/email/` is in `.prettierignore`. Otherwise `bun run format` would
reformat the build output, and CI's drift check and `format:check` would fight forever.

### Email i18n

Templates contain **no literal text**, only `t.*` placeholders. Strings live in
`backend/locales/email/{en,es}.json` (embedded), and the renderer passes the right locale's
object as `t` (e.g. `t.common.footer`, `t.test_email.heading`). Each email's subject is
`t.<template>.subject`. Each email is rendered in its **recipient's** locale (§6).

Unit tests in `email/render.rs` check that:

- both locale files have the same keys
- every template has an `.html` and a `.txt`
- every template renders in both locales with no `{{` left

That's the same kind of guard as `frontend/unit-tests/api-error.test.ts`.

Money in emails and PDFs: `Decimal` formatted as `$1,234.56`, matching the frontend's
`formatCurrency` (en-US/USD in every language, deliberately).

## 4. Backend modules

```
backend/src/
  email/
    mod.rs          # `Mailer`: enqueue(kind, recipients, locale, payload) — the only API
                    #   handlers use; takes &mut PgConnection to join the caller's transaction
    message.rs      # enum EmailKind { PasswordReset, Paystub, MonthlyReport } + typed payloads
    render.rs       # minijinja Environment over the embedded templates + locale strings
    sender.rs       # trait EmailSender + ResendSender, LogSender (+ MemorySender in tests)
    outbox.rs       # worker task: claim → render → send → mark sent / schedule retry
    scheduler.rs    # monthly-report tick (§8)
  pdf/
    mod.rs          # typst World impl, embedded fonts
    paystub.rs      # PaystubData → PDF bytes
    monthly_report.rs
  handlers/email.rs # the email controller: recipient options, send paystub, report, outbox
  services/email_outbox.rs, services/email_recipients.rs, services/password_resets.rs  # SQL
```

`handlers/email.rs` is the controller you asked for: every email-specific endpoint lives
there. The password reset endpoints stay in `handlers/auth.rs` with the rest of the public
auth routes, but they call the same `Mailer`.

### The sender trait

```rust
#[async_trait] // or a boxed-future fn — whichever the codebase settles on
pub trait EmailSender: Send + Sync {
    async fn send(&self, email: &OutgoingEmail) -> Result<(), SendError>;
}

pub struct OutgoingEmail {
    pub to: Vec<String>,
    pub subject: String,
    pub html: String,
    pub text: String,
    pub attachments: Vec<Attachment>, // filename, mime type, bytes
}

pub enum SendError {
    Retryable(String),  // network, 5xx, 429
    Permanent(String),  // other 4xx: bad address, unverified domain, …
}
```

| Impl           | Used when               | Behavior                                                                                                                                                                     |
| -------------- | ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ResendSender` | `EMAIL_PROVIDER=resend` | `POST https://api.resend.com/emails` with `Authorization: Bearer <key>` via `reqwest`; attachments as `{ filename, content: base64 }`                                        |
| `LogSender`    | default in development  | writes `email.html`/`email.txt`/`meta.txt` + attachments to `backend/target/emails/<timestamp>-<id>/` and logs the folder at INFO, so you can open what would have been sent |
| `MemorySender` | integration tests       | records sends so tests can assert on them (e.g. pull the reset link out)                                                                                                     |

`AppState` holds an `Arc<dyn EmailSender>`, built from config at startup.

**Checking production setup:** Phase 1 adds a `gr-logistics send-test-email --to <address>`
subcommand next to `create-user`. It sends one plain email straight through the configured
sender, skipping the outbox. After deploying, run
`docker compose run --rm backend send-test-email --to you@…` to prove the key, the domain
and outbound network all work before any real email depends on them.

**Resend limits:** the free plan allows 100 emails/day. Its API also rate-limits requests
per second (a small number; check the docs). The worker sends one at a time, which stays
under that rate. A `429` is `Retryable`. If the daily cap is hit, the backoff retries for
about a quarter hour (1, 2, 4, 8 minutes apart; `MAX_ATTEMPTS` = 5) and then marks the row `failed`, where the "Sent emails" panel shows
it.

## 5. The outbox

Migration `0010_email_outbox.sql` (one migration for the foundation; later phases add their own):

```sql
CREATE TABLE email_outbox (
	id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
	kind text NOT NULL CHECK (kind IN ('password_reset', 'paystub', 'monthly_report')),
	recipient text NOT NULL,       -- one address per row (§5: never a shared To: line)
	locale text NOT NULL CHECK (locale IN ('en', 'es')),
	payload jsonb,                 -- what to render; scrubbed to NULL once sent (see below)
	status text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'sent', 'failed')),
	attempts int NOT NULL DEFAULT 0,
	next_attempt_at timestamptz NOT NULL DEFAULT now(),
	last_error text,
	subject text,                  -- filled at send time, kept as the record
	requested_by uuid REFERENCES users (id),
	created_at timestamptz NOT NULL DEFAULT now(),
	sent_at timestamptz
);
CREATE INDEX email_outbox_pending ON email_outbox (next_attempt_at) WHERE status = 'pending';
```

**One row per recipient.** Every email goes to exactly one address. Putting the employee
and the person who prints paystubs on one `To:` line would show each of them the other's
address. It also gives every recipient their own status and retries, and their own
language.

**Payload, not rendered output.** A row stores _what_ to send (`{"paycheckLogId": …}`), and
the worker renders it at send time. Rows stay small (no PDFs in the database), and a paystub
reflects the log as it is when it goes out, normally seconds after the request.

**Worker loop** (one tokio task spawned by `serve`):

1. Wake on a Postgres notification or every 30s. `enqueue` runs
   `SELECT pg_notify('email_outbox', '')` in the caller's transaction, and Postgres only
   delivers it **on commit**. The worker never wakes for a row it can't see yet, a rolled
   back change sends nothing, and it works across instances. The worker listens with
   sqlx's `PgListener`. (An in-process `tokio::sync::Notify` would fire before the commit.)
2. Claim up to 10 due rows: `SELECT … WHERE status = 'pending' AND next_attempt_at <= now()
FOR UPDATE SKIP LOCKED`. Safe with more than one backend instance.
3. Render, then send each row.
4. On success, set `status = 'sent'`, `sent_at`, `subject`, and `payload = NULL`. The
   password reset payload carries a live token, so it must not sit in the table after
   it's sent.
5. On `Retryable`, add 1 to `attempts` and set `next_attempt_at = now() + 2^attempts min`,
   up to 5 attempts, then `failed`. On `Permanent`, set `failed` immediately with
   `last_error`.

Edge cases:

- A payload that no longer resolves (e.g. the paycheck log was deleted after the request)
  is a `Permanent` failure ("paycheck log not found").
- A render error is `Permanent` too.
- The reset-token payload is scrubbed on `failed` as well as on `sent`.

Failures log at ERROR with the row id (the operational logger), and the UI shows them (§9).

**Shutdown:** the worker takes the same shutdown signal as `serve`. It finishes the send
already in flight, then exits without claiming more rows, so a deploy never cuts off an
email halfway.

**Testability:** the loop body is a plain `process_due(pool, sender) -> usize`. Integration
tests call it directly instead of waiting on timers.

**Idempotency:** each send carries `Idempotency-Key: <outbox id>`. If a send times out
after Resend accepted it, the retry doesn't send a second copy. _(Verify Resend's
idempotency-key support and its retention window at implementation.)_

## 6. Recipients & locale

### Who can be picked

There's no separate recipients list. A reusable `RecipientSelect.vue` (a
`v-autocomplete multiple chips`, grouped under "Users" and "Employees" subheaders) offers:

- **Users:** every row in `users`.
- **Employees:** active employees (`active = true`) that have an `email`.

It's fed by `GET /api/email/recipient-options` and emits `{ userIds, employeeIds }`. The
backend resolves ids to addresses itself and rejects any id that isn't a user, or an active
employee with an email (`400 RECIPIENT_NOT_FOUND`). The frontend never sends raw addresses.

**People who only receive email** (e.g. whoever prints paystubs) need a `users` row. Create
them with `create-user --role employee`. That role has no data access today, since every
data route requires `admin`, so they can receive email but can't see payroll data even if
they log in. When the `employee` role gains features later, revisit this.

**This needs a frontend fix first.** Today a non-admin who logs in hits an infinite
redirect:

1. The router's role guard sends them to `dashboard`.
2. `dashboard` also requires `admin`, so the guard redirects again.

Phase 3 adds a `/no-access` view ("Your account can receive emails but has no access
here", plus a logout button), and the guard sends non-admins there.

### Locale and monthly-report opt-in

Migration `0012_email_recipients.sql` (0011 went to password reset, which shipped first):

```sql
ALTER TABLE users
	ADD COLUMN locale text NOT NULL DEFAULT 'en' CHECK (locale IN ('en', 'es')),
	ADD COLUMN receives_monthly_report boolean NOT NULL DEFAULT false;
ALTER TABLE employees
	ADD COLUMN locale text NOT NULL DEFAULT 'en' CHECK (locale IN ('en', 'es'));
```

- `users.locale` is saved whenever that user switches language in the nav bar (via
  `PUT /api/auth/me/locale`), so it follows what they actually use.
- `employees.locale` is a "Language" select on the employee add/edit form.
- `users.receives_monthly_report` is set from a "Monthly report recipients" card (the same
  `RecipientSelect`, users only). Employees can't receive the monthly report because it
  shows everyone's pay.

Who gets which email, in which language:

| Email          | Recipients                                                      | Locale                                                   |
| -------------- | --------------------------------------------------------------- | -------------------------------------------------------- |
| Password reset | the account's email                                             | the locale the login page was in (sent with the request) |
| Paystub        | users and/or employees picked in `RecipientSelect`              | each recipient's `locale`                                |
| Monthly report | users with `receives_monthly_report` (or picked for "Send now") | each recipient's `locale`                                |

Every recipient gets their own outbox row (§5), rendered in their own `locale`.

## 7. Password reset

Migration `0011_password_reset_tokens.sql`:

```sql
CREATE TABLE password_reset_tokens (
	id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
	user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
	token_hash bytea NOT NULL UNIQUE,   -- SHA-256 of the token; the token itself is never stored
	expires_at timestamptz NOT NULL,    -- created_at + 1 hour
	used_at timestamptz,
	created_at timestamptz NOT NULL DEFAULT now()
);
```

**Request:** `POST /api/auth/password-reset` `{ email, locale }` (public).

- **Always `202`**, the same response whether or not the account exists, so the endpoint
  never reveals who has an account. Because the email is only enqueued, response time
  doesn't give it away either.
- If the account exists, and it has fewer than 3 tokens in the last hour (a silent rate
  limit), the handler does the following in one transaction:
    1. Mark the user's other unused tokens as used.
    2. Generate 32 random bytes and encode them base64url as the token.
    3. Store its SHA-256 hash with a 1-hour expiry.
    4. Enqueue a `password_reset` email. Its link is `{CORS_ORIGIN}/reset-password?token=…`.
       `CORS_ORIGIN` is already the frontend's exact origin, so no second key has to be
       kept in sync with it.

**Confirm:** `POST /api/auth/password-reset/confirm` `{ token, newPassword }` (public).

- Hash the token, then find an unexpired, unused row. If there isn't one, return
  `400 RESET_TOKEN_INVALID`. Expired, used and unknown tokens all get this one code.
- **Password rules:** 12–128 characters (`PASSWORD_MIN_CHARS`/`PASSWORD_MAX_CHARS` in
  `models/auth.rs`). The cap keeps argon2's work bounded. Rules live in garde on the confirm
  DTO (a violation is `400 VALIDATION_FAILED`) and are mirrored in
  `frontend/src/utils/password.ts`. `create-user` enforces the same rule. Existing
  passwords keep working; the rule applies when one is set.
- The new password is argon2-hashed **before** the transaction, so the token's row lock
  isn't held for that work. Then, in one transaction: claim the token (`FOR UPDATE`, so two
  simultaneous uses of one link can't both win), update `users`, and **delete all of the
  user's `sessions`** (logs out every device). A rejected password doesn't use up the
  link.
- Return `204`. The frontend shows a toast and sends the user to `/login`.

**Frontend:**

- A "Forgot password?" link on [LoginView.vue](./frontend/src/views/LoginView.vue).
- `/forgot-password`: an email field. It always shows "If that email has an account, we
  sent a link."
- `/reset-password`: a new password field plus a confirmation field. On load it reads
  `token` from the query, then removes it from the URL with `history.replaceState`. That
  keeps it out of browser history, screenshots and `Referer` headers.

Both routes are `requiresAuth: false`.

## 8. Paystubs & monthly reports

### PDFs: Typst, in-process

PDFs are generated in Rust with [Typst](https://typst.app) 0.15 through the `typst-as-lib`
wrapper (`src/pdf/`; plus `typst-pdf`, and `typst-layout` for `PagedDocument`, which 0.15
no longer re-exports). It's pure Rust with no Chromium, so the distroless image is
unchanged. Templates (`backend/templates/pdf/*.typ`) and fonts are embedded. Data goes in as
a JSON string through `sys.inputs.data` and is parsed in the template. Every string arrives
translated and formatted from Rust (`pdf/paystub.rs`, `domain/format.rs`), so the `.typ`
files hold layout only. The engine is built once (`LazyLock`), and compiles run on the
blocking pool.

- **Fonts:** `backend/fonts/` holds Noto Sans Regular/Medium/Bold (SIL OFL 1.1, `OFL.txt`).
  The distroless image has no system fonts.
- **Cost:** about 200 more crates, and a cold dev build took about 40s. Cargo-chef caches
  them in the Docker build, so only changes to `Cargo.lock` pay that cost again.
- **Formatting:** `domain/format.rs` reproduces the frontend's `Intl.DateTimeFormat` output
  for both locales (`Oct 4, 2026` / `4 oct 2026`, `Sun, 10/4` / `dom, 4/10`), with tests
  checked against Intl.

Alternatives considered:

| Option                                      | Why not (for now)                                                                    |
| ------------------------------------------- | ------------------------------------------------------------------------------------ |
| Gotenberg sidecar (HTML → PDF via Chromium) | an extra container on the NAS; could reuse HTML, but it's another service to keep up |
| Headless Chrome inside the backend image    | breaks the distroless image, large                                                   |
| Frontend generates the PDF and uploads it   | can't work for the scheduled monthly report (no browser)                             |

**Trade-off:** the paystub layout then exists twice: the Vue `PaystubSheet` (screen/print)
and `paystub.typ` (email). Both read the same numbers from the backend, so only _layout_ can
drift, never amounts. Snapshot-ish tests on both catch accidental changes.

### Paystub email

`POST /api/paycheck-logs/{id}/email` `{ userIds: uuid[], employeeIds: uuid[] }` (admin).

- Only **completed** logs can be sent: `409 PAYCHECK_NOT_COMPLETED`.
- Both lists empty: `400 NO_RECIPIENTS`. An unknown id, an inactive employee or an
  employee without an email: `400 RECIPIENT_NOT_FOUND`.
- Enqueue one row per recipient and return `202 { outboxIds }`.

Sending happens in the outbox's compose step (`email/compose.rs`): it loads the log, builds
the document, renders the PDF and attaches it as `pay-stub-<name>-<week start>.pdf` (the
prefix is translated). A log deleted before sending is a permanent failure.

UI: `EmailPaystubDialog.vue` (`RecipientSelect`, with the paycheck's own employee
pre-selected when they have an email) opens from:

- an envelope icon on each completed row in Edit Payment
- an "Email pay stub" button in the Edit Payment dialog, disabled until changes are saved,
  since the email sends the saved log
- an envelope icon on completed rows in the Payment Processing history panel. That's
  where a just-completed payment shows up, because the processing form resets after it's
  saved.

### Monthly report

**Contents** (previous calendar month, completed logs only, grouped by `week_start`, the
same bucketing as the dashboard):

- the month's total paid, gross and paycheck count
- one row per employee: paychecks, loads, gross, cut
- a page listing each paycheck

The email body carries the headline numbers, and the PDF has the detail.

**Automatic:** a `scheduler.rs` task in `serve` checks hourly. On or after the 1st of the
month at 08:00 **America/Los_Angeles** (a constant, via `chrono-tz`), it enqueues the previous month's report
exactly once:

```sql
CREATE TABLE monthly_report_runs (
	month date PRIMARY KEY,          -- first day of the reported month
	outbox_ids uuid[] NOT NULL,
	created_at timestamptz NOT NULL DEFAULT now()
);
```

The tick inserts with `ON CONFLICT (month) DO NOTHING` and enqueues only if the insert
happened. A restart, a second instance or a missed hour can't double-send.

**Catch-up window: days 1–7 only.** A backend that was down on the 1st still sends when it
comes back that week. But the first deploy of this feature mid-month (or after a long
outage) doesn't fire off last month's report out of the blue. Outside the window, use the
manual send.

**Manual:** `POST /api/email/monthly-report` `{ month: "YYYY-MM", userIds?: uuid[] }`
(admin) sends now, regardless of `monthly_report_runs`, for a resend or an earlier month.
Without `userIds` it goes to the opted-in users.

## 9. API summary

| Method & path                              | Role          | Purpose                                                                       |
| ------------------------------------------ | ------------- | ----------------------------------------------------------------------------- |
| `POST /api/auth/password-reset`            | public        | request a reset link (always 202)                                             |
| `POST /api/auth/password-reset/confirm`    | public        | set a new password with a token                                               |
| `PUT /api/auth/me/locale`                  | any logged-in | save the user's UI language: `{ locale }`                                     |
| `GET /api/email/recipient-options`         | admin         | `{ users: [{ id, email }], employees: [{ id, firstName, lastName, email }] }` |
| `GET /api/email/monthly-report/recipients` | admin         | ids of users with `receives_monthly_report`                                   |
| `PUT /api/email/monthly-report/recipients` | admin         | replace that set: `{ userIds }`                                               |
| `POST /api/paycheck-logs/{id}/email`       | admin         | email a paystub: `{ userIds, employeeIds }`                                   |
| `POST /api/email/monthly-report`           | admin         | send a month's report now                                                     |
| `GET /api/email/outbox?kind=&status=`      | admin         | recent sends with status/error, for a "Sent emails" panel                     |

Every handler declares its role explicitly (AGENTS.md). `PUT /api/auth/me/locale` takes
`&[Role::Admin, Role::Employee]`, since it only touches the caller's own row.

### New error codes

Add each to `ErrorCode` with `en`/`es` translations. The existing sync test fails
otherwise.

| Code                     | Status | When                                                             |
| ------------------------ | ------ | ---------------------------------------------------------------- |
| `RESET_TOKEN_INVALID`    | 400    | unknown, expired or used reset token                             |
| `NO_RECIPIENTS`          | 400    | a send with nobody to send to                                    |
| `RECIPIENT_NOT_FOUND`    | 400    | an id that isn't a user, or an active employee with an email     |
| `PAYCHECK_NOT_COMPLETED` | 409    | emailing a draft                                                 |
| `EMAIL_NOT_CONFIGURED`   | 503    | provider is `log` in production (startup prevents this, see §10) |

## 10. Config & secrets

New keys in `config.rs`, `backend/.env.schema` (`tests/env_schema.rs` enforces they match)
and `deploy/backend.env.example`:

| Key              | Required                  | Notes                                                                                |
| ---------------- | ------------------------- | ------------------------------------------------------------------------------------ |
| `EMAIL_PROVIDER` | optional                  | `log` (default in development) or `resend`                                           |
| `EMAIL_FROM`     | when provider is `resend` | e.g. `GR Logistics <payroll@gr-logistics.us>`; must be on the Resend-verified domain |
| `RESEND_API_KEY` | when provider is `resend` | secret: Proton Pass (`protonPass(pass://…)`) in dev, `backend.env` on the server     |

The timezone isn't a key: America/Los_Angeles is a constant, since the business is in one
place.

Startup fails fast when:

- `NODE_ENV=production` and the provider is `log`, so the server can't run in production
  while silently writing emails to disk.
- the provider is `resend` and `RESEND_API_KEY` or `EMAIL_FROM` is missing.

All three keys go in `OPTIONAL_KEYS`, since a development setup doesn't need them. The
"required with `resend`" rule is a startup check in `Config::from_env`.

**Deploy note:** the compose `internal` network has no outbound route, but the backend is
also on the NPM network, which does. Confirm the container can reach `api.resend.com` once
deployed.

**Domain setup (one-time):**

1. Add `gr-logistics.us` in Resend.
2. Add the DNS records it lists in Cloudflare DNS. Resend puts SPF/MX on a `send.`
   subdomain and DKIM on `resend._domainkey`, so nothing on the root domain changes. Also
   add a DMARC record (`_dmarc`, start with `p=none`), which mailbox providers increasingly
   expect. Set the records to "DNS only" (grey cloud).
3. Create an API key with "sending access" only.

The free plan allows one verified domain, which is all this needs.

## 11. Dependencies

AGENTS.md says one crate per concern. These are new concerns, not duplicates:

| Crate                                                                   | Concern                                                                                                                                                                                                                                           |
| ----------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `minijinja` (3.x, `serde` feature)                                      | runtime template rendering (HTML auto-escape)                                                                                                                                                                                                     |
| `typst`, `typst-pdf` (+ fonts, or `typst-as-lib`)                       | PDF generation                                                                                                                                                                                                                                    |
| `sha2`                                                                  | reset-token hashing (high-entropy token, so a fast hash is correct; argon2 is for passwords)                                                                                                                                                      |
| `rand`                                                                  | token generation                                                                                                                                                                                                                                  |
| `base64`                                                                | token encoding and attachment encoding                                                                                                                                                                                                            |
| `reqwest` features `json` + `rustls-no-provider`, and `rustls` (`ring`) | HTTPS to Resend. sqlx already uses rustls with `ring`; reqwest's default `rustls` feature would compile a second backend (`aws-lc-rs`, a slow C build), so `install_tls_provider()` in `lib.rs` installs `ring` as the process default at startup |
| `chrono-tz`                                                             | America/Los_Angeles for the monthly tick (DST-correct)                                                                                                                                                                                            |

Frontend side: `maizzle` and `tailwindcss` in the new `emails/` workspace only.

## 12. Decisions

| Question            | Decision                                                                                                                                                                                                                                                                                                                                        |
| ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Provider            | **Resend, free plan** (account created 2026-10-16): 3,000 emails/month, 100/day, one verified domain, sending pauses at the cap (no surprise bill). Expected volume is a few paystubs a week plus one monthly report, far under the cap. Cloudflare Email Service was ruled out as paid. Check the limits on resend.com/pricing before Phase 1. |
| Paystub recipients  | Picked per send from a multi-select of **users** and **employees with an email** (§6)                                                                                                                                                                                                                                                           |
| Timezone            | **America/Los_Angeles** for the monthly report tick                                                                                                                                                                                                                                                                                             |
| Reset link lifetime | **1 hour** (confirmed): long enough to switch to the inbox, short enough that a forwarded or leaked email is useless soon after                                                                                                                                                                                                                 |

Still open:

- **Monthly report contents.** Are the numbers in §8 the right ones? Should drafts appear
  as a "not yet completed" note? This doesn't block Phases 1–4.

## 13. Phases

Each phase ships on its own and finishes with AGENTS.md's "Before calling any change done"
checklist.

| #   | Phase             | Delivers                                                                                                                                                                                                 | Rough size |
| --- | ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| 1   | Foundation ✅     | `email/` module, `EmailSender` + `LogSender` + `ResendSender`, outbox + worker, `emails/` Maizzle workspace + CI drift check + `.prettierignore`, config keys, email locale files, `send-test-email` CLI | 1–2 days   |
| 2   | Password reset ✅ | migration, two endpoints, two frontend views, login link, tests (MemorySender pulls the link, confirm kills sessions)                                                                                    | 1 day      |
| 3   | Recipients        | `/no-access` view + role-guard fix, locale + opt-in columns, `recipient-options` endpoint, `RecipientSelect.vue`, language saved from the nav bar, employee language field, monthly-recipients card      | ½–1 day    |
| 4   | Paystub email     | Typst setup + `paystub.typ`, send endpoint, "Email paystub" dialog                                                                                                                                       | 1–2 days   |
| 5   | Monthly report    | report query, `monthly-report.typ`, scheduler + `monthly_report_runs`, manual send, "Sent emails" panel                                                                                                  | 1–2 days   |

### Testing approach

- **Integration tests** (`#[sqlx::test]` + `MemorySender`) run each flow end to end:
  enqueue → worker tick → captured email. They check:
    - the reset link works once
    - the reset revokes sessions
    - a draft can't be emailed
    - a second monthly tick is a no-op
    - a permanent send error marks the row `failed`
- **Render tests:** every template with fixture data, in both locales:
    - no `{{` left in the output
    - all `t.*` keys present
    - names are HTML-escaped
- **PDF smoke tests:** the output starts with `%PDF`, and its text contains the employee
  name and the rounded pay.
- **`ResendSender`** is checked against a tiny local axum stub server: request shape,
  401 → `Permanent`, 429/5xx → `Retryable`.
