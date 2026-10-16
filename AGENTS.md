# AGENTS.md

Context for any agent (or human) working in this repo. Full rationale lives in
[PROJECT_PLAN.md](./PROJECT_PLAN.md) (its backend sections predate the Rust rewrite — this
file wins where they disagree); this file is the condensed "don't relitigate this"
reference. Outstanding/planned work is tracked in [TODO.md](./TODO.md).

## Stack & non-negotiables

- **Monorepo:** `frontend/` (Vite + Vue 3 + Pinia + Vuetify + Vue Router + vue-i18n, a Bun
  workspace) and `backend/` (Rust: axum + sqlx + tokio, one crate, one binary
  `gr-logistics` with `serve` / `migrate` / `create-user` / `healthcheck` subcommands).
  Bun pinned to `1.4.3` (`.github/workflows/*.yml`); Rust pinned to `1.99`
  (`backend/rust-toolchain.toml`, the CI toolchain step, `backend/Dockerfile`).
- **Frontend tooling: prefer Bun/platform natives over adding a package.**
    - **Named exception:** the root `dev` script uses the `concurrently` package
      (not Bun's `--filter --parallel`) specifically because `concurrently` supports custom
      per-process labels and colors (`-n`, `-c`) that Bun's native runner doesn't. That's an
      intentional, considered exception — not a regression to "fix" back to Bun natives.
- **Backend: one crate per concern, already chosen.** HTTP = axum/tower/tower-http,
  Postgres = sqlx, validation = garde, money = rust_decimal, errors = thiserror, logging =
  tracing, HTTP client = reqwest (only the `healthcheck` subcommand uses it). Don't add a
  second crate that duplicates one of these.
- **No ORM.** Schema lives as plain SQL in `backend/migrations/`, applied by
  `gr-logistics migrate` (files embedded in the binary, tracked in `schema_migrations`,
  under a Postgres advisory lock). **Never edit an applied migration** — add the next
  numbered file. Always bind parameters (`$1`); sqlx 0.9 only accepts `'static` SQL
  strings, which makes `format!`-built SQL a compile error — keep it that way. SQL lives in
  `backend/src/services/`, never in handlers. The data is financial; this discipline is
  what an ORM would otherwise enforce.
- **Money is `rust_decimal::Decimal` end to end — never `f64`.** Every money/percent field
  on a DTO uses `#[serde(with = "money::number")]` (`backend/src/models/money.rs`): it
  serializes as a JSON **number** (vue-i18n's `n()` throws on strings) and parses from the
  number's exact text — a JSON string is rejected, more than 2 decimal places is a 400.
- **API contract** (the frontend depends on all of it): camelCase keys
  (`#[serde(rename_all = "camelCase")]` on every DTO), dates as `"YYYY-MM-DD"`, money as
  JSON numbers, every error as `{ "error": string, "code": string }` via `AppError`
  (`backend/src/error.rs`). Bad input is **400** — use the `ValidatedJson`, `IdPath` and
  `ApiQuery` extractors (`backend/src/extract/`), never the plain axum ones, whose
  rejections are 422s / plain text. 401 no session, 403 wrong role, 404, 409 duplicate.
- **Errors carry a stable `code`**: `{ "error": "Employee not found", "code":
"EMPLOYEE_NOT_FOUND" }`. `error` is English for developers/logs; the frontend translates
  `code` via `errors.<CODE>` in its locale files and shows it in brackets for
  troubleshooting (`apiError()` in `frontend/src/utils/api.ts`). Every new `AppError` takes
  an `ErrorCode` variant — add one with its `en`/`es` translations
  (`frontend/unit-tests/api-error.test.ts` fails otherwise); never rename an existing one.
- **E2E is Playwright**, not Cypress (the frontend started with Cypress — see
  `PROJECT_PLAN.md`'s history if you're wondering why `cypress.config.ts` isn't there — it
  was replaced). Cypress's config loader choked on this repo's ESM setup, and Playwright's
  plain programmatic API is also what external browser-driving tools (e.g. `chromium-cli`)
  are built on — worth it for an agent-maintained repo. Specs are `frontend/e2e-tests/*.spec.ts`,
  config is `frontend/playwright.config.ts` (its `webServer` auto-starts the dev server, so
  `bun run e2e` needs nothing else running first).

## Business rules (do not "fix" these — they're intentional)

- **Employee cut defaults to 42%** of the pay period's summary total, per employee via
  `employees.pay_rate_percent` (`backend/src/domain/payment_calc.rs`, mirrored in
  `frontend/src/utils/payment-calc.ts` — keep both in sync if this ever changes).
- **Rounding is round-half-up to the nearest whole dollar**: `.50` and above rounds up,
  `.49` and below rounds down (`$120.50 → $121`, `$120.49 → $120`). This was explicitly
  confirmed with the project owner — don't "correct" it to banker's rounding or similar.
  In Rust that's `RoundingStrategy::MidpointAwayFromZero`; `Decimal::round()` is banker's
  rounding and must not be used for money.
- **Pay weeks run Sunday–Saturday** (`frontend/src/utils/week.ts`'s `getWeekRange`,
  `employee_paycheck_logs.week_start`/`week_end`). Confirmed convention, not a guess.
  `payday` is a separate, required column on the same table — it's not derived from the
  week range (the business doesn't pay on a fixed offset after the period ends).
- **A paycheck log is `kind` `loads` or `lump_sum`** (migration 0009). A lump sum is for
  employees whose loads aren't tracked: the body sends `lumpSum` instead of `loads`
  (exactly one of the two), it has no `employee_loads` rows, and it's paid in full —
  `summary_total` = `employee_cut` = the amount (cents kept, no rounding), stored at
  `pay_rate_percent` 100. A log's kind never changes on edit.
- **`extraPay` is optional** everywhere (form, store type, `LoadInput`) — it defaults to
  `0`, is never required client- or server-side.
- **A paycheck log stores its own totals and the rate they were computed at**
  (`summary_total`, `employee_cut`, `pay_rate_percent` — migrations 0007/0008). Creating a
  log uses the employee's current rate; every edit (draft or completed) recomputes at the
  **log's stored rate**; completing only flips `status`. A later change to an employee's
  rate never alters an existing log. The edit dialog previews with the log's
  `payRatePercent` for the same reason.
- **Employee delete is a soft delete** (`employees.active = false`). Never hard-delete —
  `employee_loads`/`employee_paycheck_logs` reference employees and must keep their history.
- **Every write to an employee, a paycheck log or its loads — draft or completed — writes to
  `audit_log`** (who/what/when) **in the same transaction** as the change. Editing a
  completed/printed log is allowed (it's the designated way to fix mistakes after the fact);
  the audit trail is what makes that safe. `audit_log` is for business record-keeping and is
  deliberately separate from the operational logger — don't conflate the two.
- **Stores are a normalized table** (`stores`, `employee_loads.store_id`), not a free-text
  column on `employee_loads`. The API still accepts/returns a plain `storeName` string —
  `backend/src/services/stores.rs`'s `resolve_store_id()` finds a store case-insensitively
  or creates it in one statement (unique index on `lower(name)`, migration 0006), inside the
  caller's transaction, so the frontend and the legacy import scripts
  (`spreadsheets/import/`, local only) never deal with `store_id` directly.
- **"Weekly log" was renamed to "paycheck log"** (`employee_weekly_logs` →
  `employee_paycheck_logs`, `employee_loads.weekly_log_id` → `paycheck_log_id`) — it's one
  paycheck per pay period, not strictly a calendar week. If you find a stray reference to
  the old names (a comment, a doc, a variable), it's drift to fix, not a hint of a second
  table.
- **No public registration.** Accounts are provisioned via
  `bun run create-user --email ... --password ... [--role admin|employee]`
  (`backend/src/cli/create_user.rs`), never through the API. There's no `/api/auth/register`.
- **Roles are already structured for a future `employee` role**, even though only `admin`
  is used today: every data handler calls `user.require_role(&[Role::Admin])?` explicitly —
  reads included — rather than assuming any logged-in user is an admin, and frontend route
  `meta.roles` already exists. Keep declaring the required role explicitly on anything new,
  so adding the `employee` role later is additive, not a retrofit.

## Auth model

- Session-based via an `httpOnly` cookie (`gr_session` → `sessions` table; see
  `backend/src/auth/`), not JWT. The cookie itself is never readable by JS — that's
  deliberate. Protected routes sit behind the `require_session` route layer **and** take the
  `AuthUser` extractor, so a route added outside the protected router still can't skip auth.
- `frontend/src/store/auth-store.ts` caches the **non-sensitive** user object
  (`id`/`email`/`role`, never a token) in `localStorage` so a page refresh doesn't log the
  user out, then calls `GET /api/auth/me` (`restoreSession()`, invoked from `main.ts` before
  mount) to reconcile against the real session cookie. **Never put the session token itself
  in `localStorage`** — that would defeat the point of `httpOnly`.

## Logging

- `tracing` (`backend/src/logging.rs`). Every event is tagged with its module path
  (`gr_logistics::services::stores`) automatically — that's the scope; don't invent one.
  `LOG_LEVEL` (`ERROR`/`WARN`/`INFO`/`DEBUG`) applies to app code and request traces;
  dependencies stay at `WARN`. `LOG_FORMAT=pretty|json`, defaulting to `pretty` in
  development and `json` in production (for log collectors).
- One INFO line per request (`request{method=… path=…}: request handled status=… latency_ms=…`,
  path only — query strings stay out of logs). `AppError` logs 4xx at WARN and 5xx at ERROR,
  so handlers don't log errors themselves.

## i18n & theming (frontend)

- **Every user-facing string goes through vue-i18n** (`src/locales/en.json`/`es.json`) from
  the start — no hardcoded UI strings, including validation messages and toasts.
- Vuetify's own locale is wired to follow the active vue-i18n locale (see `main.ts`'s
  `watch(localeRef(), ...)`) — don't set Vuetify's locale independently.
- Money is always displayed via the i18n currency number format (`n(value, 'currency')`),
  never hand-formatted with a literal `$`.
- Dark/light theme is handled by Vuetify's theme system with `localStorage` persistence
  (`frontend/src/plugins/vuetify.ts`: `currentTheme()`/`setTheme()`/`toggleTheme()`) —
  reuse those helpers rather than touching `vuetify.theme.global` directly elsewhere.
- **Toasts go through `useToastStore()`** (`frontend/src/store/toast-store.ts`:
  `success`/`error`/`info`/`warning`, or `show(text, color, timeout)`). App.vue's single
  `<v-snackbar-queue>` shows them one at a time, 3s each by default, user-dismissible.
  Don't add a page-level `<v-snackbar>`.
- **Confirmations go through `useConfirmStore().confirm({ title, text, confirmText?, color? })`**
  (`frontend/src/store/confirm-store.ts`), which resolves `true`/`false`; App.vue renders the
  one `<ConfirmDialog>`. Never `window.confirm()` — it can't be translated or themed.
- **API calls never fail silently.** Use `apiJson`/`apiRequest` (`frontend/src/utils/api.ts`,
  which throw a translated `ApiError` for non-2xx _and_ network failures) and
  `catch (error) { toast.fromError(error) }`; writes also `toast.success(...)`. A dialog
  shows the error inline instead (it stays open so input isn't lost). Plain `apiFetch` is
  only for calls where a failure status is expected (logout, the boot-time `/auth/me`).
- Global Vuetify component defaults are set once in `plugins/vuetify.ts`
  (`VTextField` → `outlined`, `VSelect`/`VAutocomplete`/`VCombobox` → `solo-filled`).
  Don't set `variant` on individual component instances unless intentionally overriding
  the default for a specific reason.

## Code style & structure

- **Composition API + `<script setup lang="ts">` everywhere** — no Options API.
- Formatting is Prettier's job (tabs, width 4, semicolons, single quotes — see
  `.prettierrc`). `eslint.config.js` deliberately disables the `eslint-plugin-vue`
  stylistic rules that fight Prettier (`vue/html-indent`, `vue/max-attributes-per-line`,
  etc.) — don't re-enable them or hand-format around what Prettier would do. Rust is
  `cargo fmt` + `cargo clippy -D warnings`; `backend/rustfmt.toml` mirrors `.prettierrc`
  (tabs, width 4, 100 columns) — keep the two in step.
- CRUD-ish features are split into a container view + form/list-type child components
  (see `components/payment-processing/`, `components/employees/`) — props down, events up.
  Don't collapse these back into one mega-component.
- **Backend layering:** `handlers/` are thin (extract → check role → call services → DTO);
  `services/` own all SQL, and writes take `&mut PgConnection` so the handler composes the
  change and its audit row into one transaction; `domain/` is pure logic with unit tests;
  `models/` are the request/response DTOs.
- Frontend and backend each keep their own copy of the rounding/cut-calculation logic
  (`frontend/src/utils/payment-calc.ts` — floats, live preview only;
  `backend/src/domain/payment_calc.rs` — `Decimal`, authoritative) with mirrored test cases
  — intentional duplication across two languages, not drift to fix.

## Env files

- `backend/.env.schema` ([varlock](https://varlock.dev)) is the source of truth for which keys
  exist and their dev defaults; it must list exactly `REQUIRED_KEYS` + `OPTIONAL_KEYS` from
  `backend/src/config.rs` (`backend/tests/env_schema.rs` enforces it). Secrets resolve from
  Proton Pass via `protonPass(pass://...)` — never commit a real value. The root bun scripts
  wrap backend commands in `bunx --bun varlock run --`, run from the repo root
  (`package.json` `varlock.loadPath` points at `backend/`; run from `backend/` the plugin
  can't resolve from `node_modules`). `--bun` because varlock needs a newer Node than the
  default one. The binary reads only its process env; release builds get it
  from the container.

## Before calling any change done

This project has no human reviewer gate beyond CI, so treat these as mandatory, not optional:

1. `cd backend && cargo fmt --check && cargo clippy --all-targets -- -D warnings`, then
   `bun run test:backend` from the root (needs Postgres: `docker compose up -d db`; tests
   create throwaway databases and never touch the dev data).
2. `cd frontend && bun run build && bun run test` (build runs `vue-tsc` — it catches
   `.vue` type errors `tsc` alone cannot)
3. `bun run lint` and `bun run format:check` from the repo root — each covers frontend
   (ESLint / Prettier) and backend (Clippy / rustfmt); CI runs the same checks. Run
   `bun run format` first to fix any formatting drift.
4. For a change touching a user-facing flow: `cd frontend && bun run e2e` (Playwright —
   starts the dev server itself, nothing else needs to be running first)

Don't report a task finished without having actually run these.
