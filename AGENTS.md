# AGENTS.md

Context for any agent (or human) working in this repo. Full rationale lives in
[PROJECT_PLAN.md](./PROJECT_PLAN.md); this file is the condensed "don't relitigate this"
reference. Outstanding/planned work is tracked in [TODO.md](./TODO.md).

## Stack & non-negotiables

- **Bun workspaces** monorepo: `frontend/` (Vite + Vue 3 + Pinia + Vuetify + Vue Router + vue-i18n)
  and `backend/` (Bun-native REST API, no web framework). Bun pinned to `1.4.3` — see
  `.github/workflows/*.yml` and `backend/Dockerfile`.
- **Prefer Bun/platform natives over adding a package.** Before reaching for an npm
  dependency, check whether Bun already does it: `Bun.serve({ routes })` (routing),
  `Bun.sql` (Postgres — no ORM), `Bun.password` (hashing), `Bun.CookieMap` (cookies),
  running `.ts` directly (no `tsx`/build step), `bun test` (backend unit tests), auto
  `.env*` loading (no `dotenv`). This has been a deliberate, repeated choice throughout
  the project — don't silently reintroduce a framework/library that duplicates one of these.
    - **Named exception:** the root `dev` script uses the `concurrently` package
      (not Bun's `--filter --parallel`) specifically because `concurrently` supports custom
      per-process labels and colors (`-n`, `-c`) that Bun's native runner doesn't. That's an
      intentional, considered exception — not a regression to "fix" back to Bun natives.
- **No ORM.** Schema lives as plain SQL in `backend/src/db/migrations/`. Always use
  `Bun.sql`'s tagged-template parameterization (`` sql`... WHERE id = ${id}` ``) — never
  string-concatenate a query. This is the one place hand-rolled SQL needs the discipline
  an ORM would otherwise enforce, given the data is financial.
- **Zod** is the one validation dependency kept on the backend (money fields need real
  validation + good error messages). **Pino was rejected** in favor of a hand-rolled
  logger (`backend/src/utils/logger.ts`) — see "Logging" below before adding a logging
  package.

## Business rules (do not "fix" these — they're intentional)

- **Employee cut is 42%** of the weekly summary total (`backend/src/utils/payment-calc.ts`,
  mirrored in `frontend/src/utils/payment-calc.ts` — keep both in sync if this ever changes).
- **Rounding is round-half-up to the nearest whole dollar**: `.50` and above rounds up,
  `.49` and below rounds down (`$120.50 → $121`, `$120.49 → $120`). This was explicitly
  confirmed with the project owner — don't "correct" it to banker's rounding or similar.
- **Pay weeks run Sunday–Saturday** (`frontend/src/utils/week.ts`'s `getWeekRange`,
  `employee_weekly_logs.week_start`/`week_end`). Confirmed convention, not a guess.
- **`extraPay` is optional** everywhere (form, store type, Zod schema) — it defaults to
  `0`, is never required client- or server-side.
- **`employee_weekly_logs.summary_total`/`employee_cut` are snapshotted at completion.**
  While `status = 'draft'` they're computed live from `employee_loads`; once `completed`,
  they're frozen so a later rate change never retroactively alters a historical record.
- **Employee delete is a soft delete** (`employees.active = false`). Never hard-delete —
  `employee_loads`/`employee_weekly_logs` reference employees and must keep their history.
- **Every write to a weekly log or its loads — draft or completed — writes to `audit_log`**
  (who/what/when). Editing a completed/printed log is allowed (it's the designated way to
  fix mistakes after the fact); the audit trail is what makes that safe. `audit_log` is for
  business record-keeping and is deliberately separate from the operational logger — don't
  conflate the two.
- **No public registration.** Admin accounts are provisioned via
  `bun run create-user --email ... --password ... [--role admin]`
  (`backend/scripts/create-user.ts`), never through the API. There's no `/api/auth/register`.
- **Roles are already structured for a future `employee` role**, even though only `admin`
  exists today: backend routes call `requireRole(user, 'admin')` explicitly rather than
  assuming any logged-in user is an admin, and frontend route `meta.roles` already exists.
  Keep declaring the required role explicitly on anything new, so adding the `employee`
  role later is additive, not a retrofit.

## Auth model

- Session-based via an `httpOnly` cookie (`Bun.CookieMap`, see `backend/src/services/auth-service.ts`),
  not JWT. The cookie itself is never readable by JS — that's deliberate.
- `frontend/src/store/auth-store.ts` caches the **non-sensitive** user object
  (`id`/`email`/`role`, never a token) in `localStorage` so a page refresh doesn't log the
  user out, then calls `GET /api/auth/me` (`restoreSession()`, invoked from `main.ts` before
  mount) to reconcile against the real session cookie. **Never put the session token itself
  in `localStorage`** — that would defeat the point of `httpOnly`.

## Logging

- `backend/src/utils/logger.ts` exports `createLogger(scope)` — every file that logs
  creates its own scoped logger named after itself (e.g.
  `const logger = createLogger('auth-service');`), not a shared generic `logger` import.
  Each scope gets a stable, deterministic color (hashed via FNV-1a into a 32-entry
  256-color palette) so different files' output is visually distinguishable in a dev
  console. Levels: `ERROR`/`WARN`/`INFO`/`DEBUG`, gated by `LOG_LEVEL`. Format is
  `LOG_FORMAT=pretty|json`, defaulting to `pretty` in development and `json` in production
  (for log collectors). When adding logging to a new file, add a new scope name — don't
  reuse another file's scope or fall back to an unscoped logger.

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
- Global Vuetify component defaults are set once in `plugins/vuetify.ts`
  (`VTextField` → `outlined`, `VSelect`/`VAutocomplete`/`VCombobox` → `solo-filled`).
  Don't set `variant` on individual component instances unless intentionally overriding
  the default for a specific reason.

## Code style & structure

- **Composition API + `<script setup lang="ts">` everywhere** — no Options API.
- Formatting is Prettier's job (tabs, width 4, semicolons, single quotes — see
  `.prettierrc`). `eslint.config.js` deliberately disables the `eslint-plugin-vue`
  stylistic rules that fight Prettier (`vue/html-indent`, `vue/max-attributes-per-line`,
  etc.) — don't re-enable them or hand-format around what Prettier would do.
- CRUD-ish features are split into a container view + form/list-type child components
  (see `components/payment-processing/`, `components/employees/`) — props down, events up.
  Don't collapse these back into one mega-component.
- Frontend/backend each keep their own copy of the shared rounding/cut-calculation logic
  (`utils/payment-calc.ts` on both sides) with mirrored test cases — this is intentional
  duplication (no shared package between the two Bun/Vite worlds exists), not drift to fix.

## Env files

- `backend/.env.example` is the source of truth for which keys exist; `.env.development`
  and `.env.production` must each be a **superset** of it. `backend/scripts/check-env-parity.ts`
  enforces this and runs in CI — if you add a new env var, add it to `.env.example` first.

## Before calling any change done

This project has no human reviewer gate beyond CI, so treat these as mandatory, not optional:

1. `cd backend && bunx tsc --noEmit && bun test`
2. `cd frontend && bun run build && bun run test` (build runs `vue-tsc` — it catches
   `.vue` type errors `tsc` alone cannot)
3. `bun run lint` from the repo root (and `bunx prettier --write <changed files>` first —
   lint will flag Prettier-formatting drift as real findings otherwise)

Don't report a task finished without having actually run these.
