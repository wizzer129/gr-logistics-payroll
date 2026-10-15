# GR Logistics — Employee Payment Tracking Portal

## 1. Overview

A web portal for an owner-operator trucking business to track and document payments made to drivers who operate the owner's vehicles. The admin (owner) logs loads driven per employee per week, the system calculates each driver's cut, and produces printable payment records.

**Primary users:** The owner/admin (payment processing, employee management, reporting). Drivers are _data subjects_, not necessarily system users — see [Open Question: Do drivers log in?](#open-questions-needing-your-input).

---

## 2. Tech Stack

> **Update (2026-10):** the backend was rewritten in Rust (axum + sqlx + rust_decimal; same
> API, schema and business rules). The Bun-backend choices in this section and §3.1 are
> historical — current backend conventions live in [AGENTS.md](./AGENTS.md).

| Layer              | Choice                                                                                                   |
| ------------------ | -------------------------------------------------------------------------------------------------------- |
| Frontend           | PWA — Vite, Vue 3 (Composition API), Pinia, Vuetify, Vue Router, Vue I18n                                |
| Backend            | Bun — runtime, `Bun.serve` native routing (no web framework), `Bun.sql`, `Bun.password`, `Bun.CookieMap` |
| Database           | PostgreSQL, accessed via `Bun.sql` (no ORM)                                                              |
| Containers         | Docker (backend + db)                                                                                    |
| Frontend hosting   | Cloudflare (Pages)                                                                                       |
| CI/CD              | GitHub Actions                                                                                           |
| Unit testing       | `bun test` (backend) · Vitest (frontend — needs Vite's `.vue` transform + DOM env)                       |
| E2E testing        | Playwright                                                                                               |
| Request validation | Zod (backend)                                                                                            |
| Logging            | hand-rolled leveled logger (see §9)                                                                      |
| Lint/Format        | ESLint + Prettier (tabs, tab width 4, semicolons, single quotes)                                         |

---

## 3. Architecture Notes

```
┌─────────────────────┐        ┌──────────────────────┐        ┌────────────┐
│ Vue 3 PWA            │  HTTPS │ Bun REST API          │  TCP   │ PostgreSQL │
│ (Cloudflare Pages)   │◄──────►│ (Docker container)    │◄──────►│ (Docker)   │
└─────────────────────┘        └──────────────────────┘        └────────────┘
```

Cloudflare Pages serves static frontend assets only. The Bun API + Postgres run in Docker on the user's own server. Recommend fronting that server with a **Cloudflare Tunnel** (`cloudflared`) rather than opening a port directly, so the origin isn't internet-facing and the Pages frontend can reach it over a stable `*.yourdomain.com` hostname with Cloudflare-managed TLS.

### 3.1 Backend built on Bun natives, not a framework stack

Bun ships enough built-in tooling that the backend doesn't need most of the usual Node packages. Decided:

| Concern            | Choice                                                                    | Why                                                                                                                                                                                  |
| ------------------ | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Web framework      | **None — `Bun.serve({ routes: {...} })`**                                 | Bun's native server supports method+path routing with params (`/api/employees/:id`). ~20 routes doesn't justify Express/Hono.                                                        |
| Env loading        | **None — Bun auto-loads `.env*` files**                                   | No `dotenv` package.                                                                                                                                                                 |
| Password hashing   | **`Bun.password.hash()` / `.verify()`**                                   | Built-in bcrypt/argon2, no separate package.                                                                                                                                         |
| Cookies/sessions   | **`Bun.CookieMap`** for parsing/signing, custom session middleware on top | Native cookie handling; session logic itself is thin enough not to need `express-session`. _(Confirm exact API against the pinned Bun version at scaffold time.)_                    |
| Running TS         | **None — Bun runs `.ts` directly**                                        | No `tsx`/`ts-node`/build step for the backend, dev or prod.                                                                                                                          |
| DB access          | **`Bun.sql` (native Postgres driver), raw SQL**                           | No ORM. Schema is small (5 tables) and stable; hand-written SQL in `db/migrations` keeps every change to financial data auditable at the SQL level rather than hidden behind an ORM. |
| Request validation | **Zod**                                                                   | Kept as a dependency — correctness on money fields (pay, extra_pay) is worth schema-based validation with good error messages, reused across ~10 write endpoints.                    |
| Logging            | **Hand-rolled leveled logger** (no Pino)                                  | See [§9](#9-environment-config--logging) — simple ERROR/WARN/INFO/DEBUG-over-console module is enough for a single-server app; avoids a dependency for something this size.          |
| Unit tests         | **`bun test`**                                                            | Jest-compatible, ships with the runtime — no test-framework dependency for backend unit tests.                                                                                       |

Net result: the backend's only real runtime dependencies are **Zod** (validation) and the Claude SDK/fetch call for AI extraction — everything else is Bun built-ins.

---

## 4. Frontend

### 4.1 Folder structure

```
frontend/
├── unit-tests/
├── e2e-tests/
└── src/
    ├── components/
    ├── locales/        # en.json, es.json
    ├── plugins/         # vuetify, i18n, etc.
    ├── routes/
    ├── store/
    ├── utils/
    └── views/
```

### 4.1.1 Internationalization (i18n)

- **vue-i18n**, registered as a plugin alongside Vuetify/router.
- Supported locales at launch: **English (`en`)** and **Spanish (`es`)**.
- Translation strings live in `src/locales/en.json` / `src/locales/es.json`, organized by view/component namespace (mirrors `src/views/`) so files stay manageable as the app grows.
- Vuetify ships its own locale files (`vuetify/locale`) — wire the active `vue-i18n` locale to Vuetify's locale provider so component-level strings (date pickers, validation messages, etc.) translate too, not just app copy.
- Locale selection: a visible switcher (e.g. in the app bar), persisted to `localStorage` so it's remembered across sessions; default to browser `navigator.language` on first visit, falling back to `en`.
- All user-facing strings (including form validation messages and toasts/snackbars) must go through the i18n layer from the start — retrofitting hardcoded strings later is the expensive path.
- Since money/dates are shown throughout (pay amounts, week ranges, dashboard charts), use `vue-i18n`'s number/date formatters (`$n`, `$d`) rather than hand-rolled formatting, so currency/date display stays consistent per locale.

### 4.2 Views

#### Login

- Admin-only. No public Register page/route in the UI.
- Email + password auth, session-based (per `session_model`) rather than JWT — cookie session, httpOnly, secure, sameSite.
- New admin users are created via a **CLI seed/create-user script** (backend-side, run on the server — e.g. `bun run create-user --email ... --password ...`), not through the app itself. Keeps user provisioning out of the public attack surface while staying quick to run when a new admin needs an account.

#### Dashboard (Admin)

- Payment overview: bar/line chart of payments over time.
- Pie chart: distribution of total payout by employee.
- YTD summary (total paid, total loads, etc.).
- Employee summary section: dropdown to select one employee → shows that employee's individual summary (totals, trend, log history).
- Charting library not yet chosen — recommend **vue-chartjs** (Chart.js wrapper, lightweight, pairs well with Vuetify) or **ApexCharts**.

#### Payment Processing

- Dropdown to select an employee.
- Two load-entry modes:
    1. **AI-assisted:** upload a screenshot; AI extracts Date, Store Name/Number, Payment, Extra Pay into the load entry fields for review before saving.
    2. **Manual entry:** same fields entered by hand.
- As loads are entered, a running summary updates live, broken into:
    - **Summary total** — sum of all loads' pay + extra pay for the week.
    - **Employee cut** — 42% of summary total.
    - **Total pay for employee** — the 42% cut, rounded per the rule below.
- **Rounding rule:** standard round-half-up to the nearest whole dollar — `.50` and above rounds up, `.49` and below rounds down (e.g. `$120.50 → $121`, `$120.49 → $120`).
- Each load is submitted as part of an array to the backend; each load becomes its own row in `employee_loads`.
- The whole submission is tied to one `employee_paycheck_logs` record (one row per employee per pay period), which all its loads reference. Also carries the `payday` date, separate from `week_start`/`week_end` — the business doesn't pay on a fixed offset after the period ends, so it's entered directly rather than derived.
- "Complete" button finalizes the paycheck log (locks it against the live-editing flow, distinct from the later admin [Edit Payment](#edit-payment) view).
- After completion, a "Print" button appears. Print layout fits the same table **twice** on one printed page, stacked.

#### Edit Payment

- Lookup `employee_paycheck_logs` records by employee name and date (range).
- Edit a located paycheck log (and its child loads).
- Any edit to a paycheck log or its loads — draft or completed — writes an entry to `audit_log` (who, what changed, old/new values, when). Editing a completed/printed log is still allowed (this is the designated way to fix mistakes after the fact), but the audit trail makes every change traceable.

#### Edit Employees

- Add / edit / delete employees.
- Delete is a **soft delete**: sets `employees.active = false` rather than removing the row, so existing `employee_loads`/`employee_paycheck_logs` history stays intact. Inactive employees drop out of the "select employee" dropdowns but remain visible/searchable in Edit Payment lookups and dashboard history.

---

## 5. Backend

### 5.1 Controllers

- **Authentication** — Login, Logout (no Register endpoint — accounts are created via the CLI user-creation script, not the API).
- **Employees** — Add, Edit, Delete, (List/Get — implied, not listed but required by UI).
- **Employee Loads** — Add, Edit, Delete, (List by weekly log — implied).
- **Employee Weekly Logs** — Add, Edit, Delete, (List/Search by employee + date — implied, needed for Edit Payment view).

### 5.2 Suggested REST routes

```
POST   /api/auth/login
POST   /api/auth/logout

GET    /api/employees
POST   /api/employees
PUT    /api/employees/:id
DELETE /api/employees/:id

GET    /api/paycheck-logs?employeeId=&from=&to=
POST   /api/paycheck-logs
PUT    /api/paycheck-logs/:id
DELETE /api/paycheck-logs/:id

GET    /api/paycheck-logs/:id/loads
POST   /api/paycheck-logs/:id/loads     (bulk array insert)
PUT    /api/loads/:id
DELETE /api/loads/:id

POST   /api/loads/extract             (AI screenshot → structured fields)

GET    /api/dashboard/summary?year=
GET    /api/dashboard/employee/:id/summary

GET    /api/audit-log?entity=&entityId=
```

### Scripts (not HTTP routes)

```
bun run scripts/create-user.ts --email <email> --password <password> --role admin
```

Run directly on the server (or inside the backend container via `docker exec`) to provision new admin accounts.

### 5.3 Models / Database schema

Corrected/expanded from spec (naming cleaned up: `empoloyee_loads` → `employee_loads`; `created_by`/`last_edited_by` should be **foreign keys to the acting admin user**, not free text):

**`users`** _(spec called this out implicitly via `session_model.user_id`/`user_email`, but no users table was defined — added)_

| column        | type           | notes                                                                                                                |
| ------------- | -------------- | -------------------------------------------------------------------------------------------------------------------- |
| id            | uuid/serial PK |                                                                                                                      |
| email         | text unique    |                                                                                                                      |
| password_hash | text           | via `Bun.password.hash()` — never store plaintext                                                                    |
| role          | text           | `admin` at launch; `employee` planned for a future employee-portal phase (read-only access to their own weekly logs) |
| created_at    | timestamptz    |                                                                                                                      |

**`sessions`**

| column     | type          | notes                                              |
| ---------- | ------------- | -------------------------------------------------- |
| session_id | uuid PK       |                                                    |
| user_id    | FK → users.id |                                                    |
| user_email | text          | _(redundant if we join users — consider dropping)_ |
| expires_at | timestamptz   | needed for session expiry                          |
| created_at | timestamptz   |                                                    |

**`employees`**

| column                  | type                 | notes                            |
| ----------------------- | -------------------- | -------------------------------- |
| id                      | serial/uuid PK       |                                  |
| first_name              | text                 |                                  |
| last_name               | text                 |                                  |
| dob                     | date                 |                                  |
| email                   | text                 |                                  |
| address                 | text                 |                                  |
| active                  | boolean default true | **added — supports soft delete** |
| created_by              | FK → users.id        |                                  |
| last_edited_by          | FK → users.id        |                                  |
| created_at / updated_at | timestamptz          |                                  |

**`employee_paycheck_logs`** _(renamed from `employee_weekly_logs` — it's one paycheck per pay period, not strictly a calendar week)_

| column                  | type              | notes                                                                                                                  |
| ----------------------- | ----------------- | ---------------------------------------------------------------------------------------------------------------------- |
| id                      | serial/uuid PK    |                                                                                                                        |
| employee_id             | FK → employees.id |                                                                                                                        |
| week_start              | date              | Sunday of the pay week — split "date range" into two columns for easy querying/sorting                                 |
| week_end                | date              | Saturday of the pay week                                                                                               |
| payday                  | date              | the actual date paid — not derived from week_end, entered directly (the lag between period-end and payday isn't fixed) |
| status                  | text              | `draft` / `completed` — needed for the Complete→Print flow                                                             |
| summary_total           | numeric(10,2)     | snapshot at completion, computed live from `employee_loads` while draft                                                |
| employee_cut            | numeric(10,2)     |                                                                                                                        |
| created_by              | FK → users.id     |                                                                                                                        |
| last_edited_by          | FK → users.id     |                                                                                                                        |
| created_at / updated_at | timestamptz       |                                                                                                                        |

**`stores`** _(added after the initial scaffold — normalizes what was a free-text `store_name` column)_

| column                  | type           | notes                                     |
| ----------------------- | -------------- | ----------------------------------------- |
| id                      | serial/uuid PK |                                           |
| name                    | text unique    | case-insensitive lookup via `lower(name)` |
| created_by              | FK → users.id  |                                           |
| last_edited_by          | FK → users.id  |                                           |
| created_at / updated_at | timestamptz    |                                           |

**`employee_loads`**

| column                  | type                           | notes                                                                   |
| ----------------------- | ------------------------------ | ----------------------------------------------------------------------- |
| id                      | serial/uuid PK                 | **added — spec gave no PK for this table**                              |
| employee_id             | FK → employees.id              |                                                                         |
| paycheck_log_id         | FK → employee_paycheck_logs.id | renamed from `weekly_log_id`                                            |
| store_id                | FK → stores.id                 | renamed from `store_name` (free text) once the `stores` table was added |
| load_date               | date                           | (spec said `date`, renamed to avoid reserved-word friction)             |
| pay                     | numeric(10,2)                  |                                                                         |
| extra_pay               | numeric(10,2) default 0        |                                                                         |
| created_by              | FK → users.id                  |                                                                         |
| last_edited_by          | FK → users.id                  |                                                                         |
| created_at / updated_at | timestamptz                    |                                                                         |

> Note: `load_name` appeared in the spec's model list but isn't mentioned in the Payment Processing UI fields (Date, Store Name/Number, Payment, Extra Pay). Dropped in favor of `store_name`/`store_id`; flag if `load_name` was meant to be separate from store name.
>
> The API layer still accepts/returns a plain `storeName` string — `resolveStoreId()` in the backend looks it up case-insensitively or creates it, so the frontend form and the legacy import script never deal with `store_id` directly.

**`audit_log`**

| column     | type                                 | notes                                                     |
| ---------- | ------------------------------------ | --------------------------------------------------------- |
| id         | serial/uuid PK                       |                                                           |
| entity     | text                                 | e.g. `employee_paycheck_log`, `employee_load`, `employee` |
| entity_id  | FK (polymorphic, no hard constraint) |                                                           |
| action     | text                                 | `create` / `update` / `delete`                            |
| changed_by | FK → users.id                        |                                                           |
| diff       | jsonb                                | old/new field values                                      |
| created_at | timestamptz                          |                                                           |

---

## 6. AI Load Extraction

> **Removed (2026-10-14).** Built in the Rust backend (`POST /api/loads/extract`) but never
> wired into the frontend, then dropped along with `ANTHROPIC_API_KEY`. Kept below as history.

Flow: admin uploads a screenshot (e.g. a load-board or paystub screenshot) → backend sends image to **Claude** (Messages API, vision input) → model returns structured JSON `{ date, store_name, payment, extra_pay }` → frontend pre-fills the manual-entry form for the admin to confirm/correct before saving.

Extracted data is always a **suggestion the admin must confirm**, not an auto-save, to avoid silent misreads of payment amounts. Needs an `ANTHROPIC_API_KEY` in backend env config (see [Environment Config](#9-environment-config--logging)).

---

## 7. Rounding Logic

Standard round-half-up to the nearest whole dollar: `.50` and above rounds up, `.49` and below rounds down. Example: `$120.50 → $121`, `$120.49 → $120`. Implement as a pure, well-tested utility function (see [Testing Strategy](#8-testing-strategy)).

---

## 8. Testing Strategy

- **Unit — frontend (Vitest):** Pinia stores, utils (esp. the 42%-cut + rounding calculation — pure function, thorough edge-case coverage: $0, exact `.50`, negative/invalid input guards), components.
- **Unit — backend (`bun test`):** controllers/services, Zod schemas, the shared rounding/cut-calculation util (mirrored test cases to the frontend one, since both sides may independently compute/display it).
- **E2E (Playwright):** login flow, full payment-processing flow (select employee → add loads → complete → print), employee CRUD, weekly-log lookup/edit.
- CI (GitHub Actions): lint → `bun test` (backend) + `vitest run` (frontend) → e2e (headless) → build → deploy on merge to main.

---

## 9. Environment Config & Logging

### Env files

- `.env.example` — committed, documents every variable with placeholder/dummy values. Source of truth for what config exists.
- `.env.development` and `.env.production` — both **extend** `.env.example` (i.e. must define a superset of its keys; startup should validate no key from `.env.example` is missing in either). Not committed (gitignored) except `.env.example`.
- Expected keys include: `DATABASE_URL`, `SESSION_SECRET`, `ANTHROPIC_API_KEY`, `LOG_LEVEL`, `PORT`, `NODE_ENV`.
- Bun auto-loads these `.env*` files on startup — no `dotenv` package needed.

### Logging

- Backend uses a small **hand-rolled logging module** (no Pino) with levels **ERROR, WARN, INFO, DEBUG**, controlled by a `LOG_LEVEL` env var (default `INFO` in production, `DEBUG` in development). JSON-line output to stdout — easy to grep/trace in `docker logs`, and consistent across levels without needing a formatting package.
- Each request should log at minimum: method, path, status, duration, and (if authenticated) `user_id`, to make tracing a specific admin action straightforward.
- Pair with the `audit_log` table ([section 5.3](#53-models--database-schema)): logs are for _operational troubleshooting_ (why did a request fail), `audit_log` is for _business record-keeping_ (who changed this employee's pay data and when) — keep them separate, don't conflate.

---

## 10. Roles & Future Employee Portal

- `users.role` supports multiple values from day one: `admin` now, `employee` reserved for a future phase.
- Backend: add role-check middleware now (even though only `admin` exists today), so every route declares its required role explicitly (e.g. `requireRole('admin')`) rather than assuming "any logged-in user is an admin." This avoids an authorization gap when the `employee` role is introduced later.
- Frontend: route guards (`vue-router` `beforeEnter`/meta fields) should already branch on role, even if today every route just requires `admin`. Keeps the future employee-portal views (e.g. "my weekly logs" read-only view) additive rather than a retrofit.
- `employees` table rows are **data about a driver**, not login accounts. When the employee portal ships, an `employees` row will need to link to a `users` row (e.g. `employees.user_id` nullable FK) for the employee who's also a login-capable user — not needed yet, but worth keeping in mind so the eventual migration is additive.
- `employee_paycheck_logs.summary_total`/`employee_cut` are **snapshotted at completion** (confirmed) — computed/stored once when status flips to `completed`, so later rate changes never retroactively alter historical records. While status is `draft`, these are computed live from `employee_loads` on read, not stored.

---

## 11. Other Recommendations

- Add `GET` (list/detail) endpoints — the spec's controllers list Add/Edit/Delete only, but the UI needs reads too (dashboard, dropdowns, lookups).
- With raw SQL via `Bun.sql`, always use its tagged-template parameterization (`sql\`... WHERE id = ${id}\``) — never string-concatenate values into queries. This is the one place hand-rolled SQL needs discipline in place of an ORM's guardrails, given this handles financial data.
- Enforce one `employee_paycheck_logs` row per `(employee_id, week_start)` via a unique constraint, to prevent accidental duplicate weekly submissions.
- Validate `pay`/`extra_pay` as non-negative numerics server-side, not just in the UI.
- `created_at`/`updated_at` on every table (already reflected in the schema above), plus the `audit_log` table for completed-record changes.
