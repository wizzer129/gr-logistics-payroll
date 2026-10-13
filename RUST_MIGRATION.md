# Rust Backend Migration

Plan for porting `backend/` (Bun + TypeScript, ~1,200 lines, 15 routes) to Rust. This is the
working reference while the port is in flight. When the port lands, fold the surviving
rules into [AGENTS.md](./AGENTS.md) and delete this file.

Business rules from [AGENTS.md](./AGENTS.md) (42% default cut, round-half-up, Sunday–Saturday
weeks, snapshotting on completion, soft deletes, audit on every paycheck write, no public
registration) carry over **unchanged**. The port changes how the backend is built, not what
it does, except for the five decisions below.

---

## Decisions (settled — don't relitigate)

| #   | Topic                 | Decision                                                                                                                                        |
| --- | --------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Money type            | `rust_decimal::Decimal` end to end. No `f64` anywhere money flows.                                                                              |
| 2   | Date/number JSON      | Dates serialize as `"YYYY-MM-DD"`; money serializes as a JSON **number**. Frontend already handles both (see [Frontend contract](#frontend-contract-findings)). |
| 3   | Validation errors     | Malformed JSON or failed validation → **400** `{ "error": "..." }`. Today these leak out as 500.                                                |
| 4   | Auth                  | Every route except `login`, `logout`, `health` requires a session. Data routes also check `admin` explicitly.                                   |
| 5   | Migrations            | `migrate.ts` is ported as a `migrate` subcommand of the same binary, keeping the existing `schema_migrations` table.                            |

### 1. `rust_decimal` for money

- Columns are `numeric(10,2)` (`pay`, `extra_pay`, `summary_total`, `employee_cut`) and
  `numeric(5,2)` (`pay_rate_percent`). Map all of them to `Decimal` via sqlx's `rust_decimal`
  feature.
- **Rounding:** `amount.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)`.
  All amounts are non-negative (DB `CHECK`s enforce it), so this is the same as the confirmed
  round-half-up rule (`$120.50 → $121`, `$120.49 → $120`). Do **not** use the default
  `round()` — that's banker's rounding.
- **Cut:** `summary_total * pay_rate_percent / Decimal::ONE_HUNDRED`, then round.
- **JSON out:** serialize as a number (`rust_decimal`'s `serde-with-float` attribute, or a
  small `serialize_with` helper). This replaces the TS `toNum()` workaround: there's no
  "numeric comes back as a string" trap in Rust, the type system handles it.
- **JSON in:** the frontend sends plain JS numbers (`120.1`). Parse them as decimals
  **without going through `f64`**: enable `serde_json/arbitrary_precision` +
  `rust_decimal/serde-with-arbitrary-precision`. Reject inputs with more than 2 decimal
  places (400). Pin this with a unit test: `120.1` must parse to exactly `120.10`.
- **Behavior change to accept:** float summing can drift (`0.1 + 0.2`), decimal doesn't. A
  handful of historical edge cases could round differently. The backend is authoritative.
  The frontend's [payment-calc.ts](./frontend/src/utils/payment-calc.ts) is a live preview
  only, and a $0–$1 disagreement on an exact `.50` boundary is acceptable. Keep the mirrored
  test cases in sync.

### 2. Frontend contract findings

Checked every `fetch` in `frontend/src` on 2026-10-12.

**Dates.** `Bun.sql` returns `date` columns as JS `Date`, so today the API sends
`"2026-10-11T00:00:00.000Z"`. Rust (`chrono::NaiveDate`) will send `"2026-10-11"`.
Every consumer already truncates with `.slice(0, 10)`, so both shapes work unchanged:

| Field                            | Consumer                                                                                                        | Handling                                |
| -------------------------------- | --------------------------------------------------------------------------------------------------------------- | --------------------------------------- |
| `weekStart`, `weekEnd`, `payday` | [DashboardView.vue:175-178](./frontend/src/views/DashboardView.vue#L175-L178), [EditPaymentView.vue:203-205](./frontend/src/views/EditPaymentView.vue#L203-L205) | `formatMonthDay()` → `.slice(0, 10)` ([week.ts:17](./frontend/src/utils/week.ts#L17)) |
| `weekStart`, `weekEnd`, `payday` | [EditPaymentView.vue:82-84](./frontend/src/views/EditPaymentView.vue#L82-L84)                                    | `.slice(0, 10)` into date inputs        |
| `loadDate`                       | [LoadList.vue:26](./frontend/src/components/payment-processing/LoadList.vue#L26)                                 | `.slice(0, 10)`                         |
| `dob`                            | not displayed anywhere                                                                                          | —                                       |
| `period` (dashboard)             | [PaymentsOverTimeChart.vue:33](./frontend/src/components/dashboard/PaymentsOverTimeChart.vue#L33)                | already `"YYYY-MM"` text from SQL       |

After cutover, the comment at [LoadList.vue:24-25](./frontend/src/components/payment-processing/LoadList.vue#L24-L25)
("a full ISO datetime when it comes back from the API") becomes stale. Update it. Keep the
`.slice(0, 10)` calls: they cost nothing.

**Numbers.** The frontend expects real JSON numbers for `pay`, `extraPay`, `summaryTotal`,
`employeeCut`, `payRatePercent`, `ytdTotal`, `ytdGross`, `total` (vue-i18n `n()` throws on
strings). `COUNT(*)` fields (`paycheckCount`, `totalLoads`) come back as `i64` and serialize
as numbers.

**Keys.** All response keys are camelCase. Use `#[serde(rename_all = "camelCase")]` on every
DTO, and don't rely on SQL `AS "camelCase"` aliases.

**Errors.** The frontend **never reads the error body**. Every call site only checks
`response.ok` (auth-store, EmployeesView, DashboardView, EditPaymentView,
PaymentProcessingView, useEmployees). The 500 → 400 change and the new 401s on GETs are
invisible to the UI. Keep the `{ "error": string }` shape anyway, for future use and logs.

### 3. 400 on bad input

- Today: `schema.parse()` throws `ZodError`, which isn't an `HttpError`, so `handle()` in
  [index.ts](./backend/src/index.ts) logs it as "unhandled error" and returns 500.
- Rust: a custom `ValidatedJson<T>` extractor wraps `axum::Json` (whose defaults are 400
  for syntax errors and **422** for type errors) and maps **all** rejections and validation
  failures to `AppError::BadRequest(msg)` → 400.
- Validation: `garde` derives (`#[garde(length(min = 1))]`, `#[garde(email)]`,
  `#[garde(range(min = 0))]`) replace Zod. Rules to carry over from
  [schemas.ts](./backend/src/models/schemas.ts):
    - `extraPay` optional, defaults to `0` (`#[serde(default)]`).
    - `loads` min length 1.
    - `payRatePercent` > 0 and ≤ 100.
    - Employee update is a partial (all fields `Option`).
    - Paycheck-log update has no `employeeId` (separate struct, not a flag).
- Also 400: invalid `:id` UUID path params (use a `Path<Uuid>` extractor, mapped to 400,
  not a Postgres cast error → 500), and an invalid `year` query param (already 400 in TS).

### 4. Auth on every endpoint

Today `GET /api/employees`, `GET /api/paycheck-logs` and `GET /api/paycheck-logs/:id/loads`
are **unauthenticated** ([index.ts:89](./backend/src/index.ts#L89)). That's a data leak
(employee names, pay rates, pay history).

Two layers, so forgetting one isn't enough to leak:

1. **Router layer:** routes split into `public` (`POST /api/auth/login`,
   `POST /api/auth/logout`, `GET /api/health`) and `protected` (everything else). The
   protected router gets `.route_layer(from_fn_with_state(state, require_session))`, which
   resolves the session cookie and inserts `AuthUser` into request extensions, or returns 401.
2. **Handler level:** handlers take `user: AuthUser` (a `FromRequestParts` extractor), and
   call `user.require_role(Role::Admin)?` explicitly, **including reads**. That matches the
   AGENTS.md rule ("declare the required role explicitly") so adding an `employee` role later
   is additive. `GET /api/auth/me` needs only a session, no role.

Session mechanics stay byte-compatible so **existing logins survive the cutover**: same
cookie name (`gr_session`), same attributes (`HttpOnly`, `SameSite=Lax`, `Secure` in prod,
`Path=/`, `Expires`), same `sessions` table lookup. Use `axum-extra`'s `CookieJar`.

Passwords: `Bun.password` writes standard PHC strings (`$argon2id$v=19$m=...`). The `argon2`
crate's `PasswordHash::new` + `Argon2::default().verify_password` reads the params from the
string itself. **Before cutover, verify one real production hash** in a test, or every user
gets locked out. Run hashing in `tokio::task::spawn_blocking`: it's CPU-bound and stalls
the async runtime otherwise.

### 5. Port of `migrate.ts`

`gr-logistics migrate`, same algorithm as [migrate.ts](./backend/scripts/migrate.ts):

1. `CREATE TABLE IF NOT EXISTS schema_migrations (name text PRIMARY KEY, applied_at timestamptz NOT NULL DEFAULT now())`, the **same table**, so the existing prod DB is recognized
   as up to date and nothing re-runs.
2. Read applied names.
3. For each `*.sql` in lexical order not yet applied: in one transaction, run the file with
   `sqlx::raw_sql(contents)` (supports multiple statements, unlike `sqlx::query`), then
   `INSERT INTO schema_migrations (name)`.

Improvements over the TS version:

- **Embedded files:** `include_dir!("migrations")` compiles the `.sql` files into the binary,
  so the distroless image needs no files beside it.
- **Advisory lock:** `SELECT pg_advisory_lock(<const>)` around the run, so two containers
  starting at once can't double-apply.
- **Checksum (optional):** store a SHA-256 in a new nullable `checksum` column and warn if an
  applied file changed on disk.

Why not `sqlx migrate` / `sqlx::migrate!()`: it uses its own `_sqlx_migrations` table
with checksums. Adopting it means hand-seeding that table in prod for the 5 existing files.
Not worth it for a ~40-line runner.

`create-user` is ported the same way: `gr-logistics create-user --email … --password … [--role admin]`, with the same upsert SQL.

---

## Other fixes made during the port

These are bugs in the current backend. Fix them in Rust, and don't replicate them:

- **`resolveStoreId` runs outside the transaction.** It uses the global `sql`, not `tx`
  ([store-service.ts](./backend/src/services/store-service.ts)). A failed paycheck insert
  leaves orphan stores. Rust: the function takes `&mut PgConnection` / `&mut Transaction`.
  `recordAudit` should also move inside the same transaction, so an audit write can't
  silently fail after the data write committed.
- **Store lookup race.** Lookup is case-insensitive (`lower(name)`) but the `UNIQUE` is
  case-sensitive. Add migration `0006`:
  `CREATE UNIQUE INDEX stores_name_lower_idx ON stores (lower(name))`, then use
  `INSERT … ON CONFLICT (lower(name)) DO UPDATE SET name = stores.name RETURNING id`.
- **CORS.** Today it sends `*` together with `Allow-Credentials: true` in dev (browsers
  reject that combo), and the prod origin is hardcoded to `app.example.com`. Rust: a new
  `CORS_ORIGIN` env var, and `tower_http::cors::CorsLayer` with an explicit origin plus
  credentials. Handle `OPTIONS` preflight (TS never did).
- **`SESSION_SECRET` is required but unused.** Drop it from `.env.example` (sessions are
  random UUIDs in the DB, so there's nothing to sign).
- **Upload size.** axum's default body limit is **2 MB**. Phone screenshots exceed that.
  Set `DefaultBodyLimit::max(10 * 1024 * 1024)` on `/api/loads/extract` only.

---

## Stack

| Concern          | Crate                                                                 | Replaces                       |
| ---------------- | --------------------------------------------------------------------- | ------------------------------ |
| HTTP             | `axum` 0.8, `tokio`, `tower`, `tower-http` (cors, trace, limit)       | `Bun.serve({ routes })`        |
| Postgres         | `sqlx` 0.8 (`postgres`, `runtime-tokio`, `tls-rustls`, `uuid`, `chrono`, `rust_decimal`, `json`, `macros`) | `Bun.sql` |
| Money            | `rust_decimal` (+ `serde-with-arbitrary-precision`)                   | `number` + `toNum()`           |
| JSON             | `serde`, `serde_json` (`arbitrary_precision`)                         | `Response.json`                |
| Validation       | `garde`                                                               | Zod                            |
| Cookies          | `axum-extra` (`cookie`)                                               | `Bun.CookieMap`                |
| Passwords        | `argon2`                                                              | `Bun.password`                 |
| Logging          | `tracing`, `tracing-subscriber` (`json`, `env-filter`)                | hand-rolled `logger.ts`        |
| Anthropic API    | `reqwest` (`json`, `rustls-tls`, no default features), `base64`       | `@anthropic-ai/sdk`            |
| CLI              | `clap` (derive)                                                       | `parseArgs`, separate scripts  |
| Config           | `dotenvy` (dev only)                                                  | Bun's auto `.env*` loading     |
| Errors           | `thiserror`                                                           | `HttpError` class              |
| Migrations embed | `include_dir`                                                         | `readdir`                      |

Versions are as of writing. Run `cargo add` to get current releases, and pin via the committed
`Cargo.lock`. The "prefer platform natives" rule in AGENTS.md becomes "prefer the
tokio/tower/axum ecosystem, and don't add a second crate for something one of these already
does."

No official Anthropic Rust SDK exists. Call `POST {ANTHROPIC_BASE_URL}/v1/messages` directly
with `x-api-key` and `anthropic-version` headers. Make the base URL configurable (default
`https://api.anthropic.com`) so tests can point it at a mock server.

---

## Folder structure

Lives in `backend-rs/` beside `backend/` during the port. Rename it to `backend/` at cleanup.
It's a single crate, not a workspace (one binary, about 1–2k lines, no shared lib consumer).
The layout follows the common axum layering: thin handlers, extractors for cross-cutting
concerns, services own SQL, a `lib.rs` that builds the `Router` so tests can drive it without
a socket.

```
backend-rs/
├── Cargo.toml
├── Cargo.lock                    # committed (it's a binary)
├── rust-toolchain.toml           # pinned toolchain, like the Bun 1.4.3 pin
├── .env.example                  # source of truth for env keys (parity script still applies)
├── .env.development
├── Dockerfile
├── migrations/                   # moved from backend/src/db/migrations, same filenames
│   ├── 0001_init.sql
│   ├── …
│   └── 0006_stores_name_lower_unique.sql
├── src/
│   ├── main.rs                   # clap: `serve` (default) | `migrate` | `create-user`
│   ├── lib.rs                    # pub fn build_app(state: AppState) -> Router
│   ├── config.rs                 # Config::from_env(); fails fast listing missing keys (assertEnv)
│   ├── state.rs                  # AppState { pool: PgPool, config: Arc<Config>, anthropic: AnthropicClient }
│   ├── error.rs                  # AppError enum → IntoResponse as { "error": msg } + status
│   ├── logging.rs                # tracing init: LOG_LEVEL, LOG_FORMAT=pretty|json, per-scope colors
│   ├── routes.rs                 # public vs protected routers, layers (cors, trace, auth, body limit)
│   ├── auth/
│   │   ├── mod.rs
│   │   ├── extractor.rs          # AuthUser (FromRequestParts), Role, require_role()
│   │   ├── middleware.rs         # require_session route_layer
│   │   ├── password.rs           # hash/verify via spawn_blocking
│   │   └── session.rs            # create/lookup/destroy session, cookie build/clear
│   ├── extract/
│   │   └── validated_json.rs     # ValidatedJson<T>: serde + garde, every failure → 400
│   ├── handlers/                 # thin: extract → call service → map to DTO
│   │   ├── mod.rs
│   │   ├── auth.rs               # login, logout, me
│   │   ├── employees.rs
│   │   ├── paycheck_logs.rs
│   │   ├── dashboard.rs
│   │   ├── loads.rs              # POST /api/loads/extract (multipart)
│   │   └── health.rs
│   ├── services/                 # all SQL lives here; take &PgPool or &mut Transaction
│   │   ├── mod.rs
│   │   ├── employees.rs
│   │   ├── paycheck_logs.rs
│   │   ├── dashboard.rs
│   │   ├── stores.rs             # resolve_store_id(tx, name, user_id)
│   │   ├── audit.rs              # record_audit(tx, …)
│   │   └── load_extraction.rs    # Anthropic call + parse
│   ├── models/                   # request/response DTOs, #[serde(rename_all = "camelCase")]
│   │   ├── mod.rs
│   │   ├── employee.rs
│   │   ├── paycheck_log.rs
│   │   ├── load.rs
│   │   ├── dashboard.rs
│   │   └── money.rs              # Decimal serde helpers (number in/out, ≤2 dp check)
│   ├── domain/
│   │   └── payment_calc.rs       # round_to_dollar, calculate_employee_cut, sum_loads (+ unit tests)
│   └── cli/
│       ├── migrate.rs            # schema_migrations runner, advisory lock, embedded files
│       └── create_user.rs
└── tests/                        # integration: real Postgres via #[sqlx::test]
    ├── common/mod.rs             # test app builder, login helper, fixtures
    ├── auth.rs
    ├── employees.rs
    ├── paycheck_logs.rs
    ├── dashboard.rs
    └── migrate.rs
```

### Key scaffolds

`routes.rs`, showing the public/protected split:

```rust
pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .route("/api/auth/login", post(handlers::auth::login))
        .route("/api/auth/logout", post(handlers::auth::logout))
        .route("/api/health", get(handlers::health::health));

    let protected = Router::new()
        .route("/api/auth/me", get(handlers::auth::me))
        .route("/api/employees", get(employees::list).post(employees::create))
        .route("/api/employees/{id}", put(employees::update).delete(employees::delete))
        .route("/api/paycheck-logs", get(paycheck_logs::list).post(paycheck_logs::create))
        .route("/api/paycheck-logs/{id}", put(paycheck_logs::update).delete(paycheck_logs::delete))
        .route("/api/paycheck-logs/{id}/complete", post(paycheck_logs::complete))
        .route("/api/paycheck-logs/{id}/loads", get(paycheck_logs::loads))
        .route("/api/dashboard/summary", get(dashboard::summary))
        .route("/api/dashboard/employee/{id}/summary", get(dashboard::employee_summary))
        .route(
            "/api/loads/extract",
            post(loads::extract).layer(DefaultBodyLimit::max(10 * 1024 * 1024)),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::require_session));

    Router::new()
        .merge(public)
        .merge(protected)
        .fallback(|| async { AppError::NotFound("Not found".into()) })
        .layer(TraceLayer::new_for_http())   // replaces handle()'s "request handled" log
        .layer(cors_layer(&state.config))
        .with_state(state)
}
```

`error.rs`, the single place status codes and the response shape are decided:

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")] BadRequest(String),
    #[error("{0}")] Unauthorized(String),
    #[error("{0}")] Forbidden(String),
    #[error("{0}")] NotFound(String),
    #[error(transparent)] Internal(#[from] anyhow::Error),   // also From<sqlx::Error>
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            AppError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m.clone()),
            AppError::Forbidden(m) => (StatusCode::FORBIDDEN, m.clone()),
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            AppError::Internal(e) => {
                tracing::error!(error = ?e, "unhandled error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".into())
            }
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}
```

`domain/payment_calc.rs`:

```rust
pub const DEFAULT_PAY_RATE_PERCENT: Decimal = dec!(42);

/// Round-half-up to the nearest whole dollar: .50+ rounds up, .49 and below rounds down.
pub fn round_to_dollar(amount: Decimal) -> Decimal {
    amount.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)
}

pub fn calculate_employee_cut(summary_total: Decimal, pay_rate_percent: Decimal) -> (Decimal, Decimal) {
    let cut = summary_total * pay_rate_percent / Decimal::ONE_HUNDRED;
    (cut, round_to_dollar(cut))
}

pub fn sum_loads(loads: &[LoadInput]) -> Decimal {
    loads.iter().map(|l| l.pay + l.extra_pay).sum()
}
```

---

## Migration steps

Estimates assume focused work. Total: **about 5–7 days**.

### Phase 0: Lock the contract (1–2 days) ⟵ start here

1. Apply decisions **3** and **4** to the *current* Bun backend first: map `ZodError` → 400
   in `handle()`, and add `requireUser` + `requireRole(user, 'admin')` to the three
   unprotected GETs. These are small changes, and they make the old and new backends
   behave identically, so one contract suite can gate both.
2. Write `contract-tests/` (repo root, `bun test`, plain `fetch` against `API_URL`). Cover:
    - every route: happy path + 401 without cookie + 400 on bad body/UUID
    - money edges: `.49` / `.50` rounding, per-employee rate (40%), `extraPay` omitted → 0
    - paycheck lifecycle: create → update (loads replaced) → complete → delete (loads cascade)
    - audit rows written for each paycheck/employee write
    - soft delete hides the employee from the list but keeps their logs
3. Compare dates via `.slice(0, 10)` in assertions, so the suite passes on both
   backends.
4. Green against Bun → commit. This suite is the cutover gate.

### Phase 1: Skeleton (½ day)

1. `cargo new backend-rs`, add `rust-toolchain.toml`, add the dependencies from [Stack](#stack).
2. `config.rs`, `logging.rs`, `error.rs`, `state.rs`, `GET /api/health`.
3. ✅ Migrations moved to `backend-rs/migrations/` and `migrate` ported
   ([src/cli/migrate.rs](./backend-rs/src/cli/migrate.rs), tests in
   [tests/migrate.rs](./backend-rs/tests/migrate.rs)). `backend/scripts/migrate.ts` now
   reads the same folder, so `bun run migrate` still works locally. Rust owns the schema:
   add new migrations **only** in `backend-rs/migrations/`.
   - The Bun Docker image can no longer run `migrate` (the files are outside its build
     context). In any deployed environment, run migrations with the Rust image
     (`gr-logistics migrate`). It works against the same DB as the Bun backend.
4. ✅ `gr-logistics migrate` against the dev DB: "up to date", nothing applied.
5. ✅ `create-user` ported in Phase 2.

### Phase 2: Auth (½–1 day) ✅

1. ✅ Session service, cookie handling, `require_session` layer, `AuthUser` extractor
   ([src/auth/](./backend-rs/src/auth/)).
2. ✅ Login / logout / me ([src/handlers/auth.rs](./backend-rs/src/handlers/auth.rs)).
   A login for an unknown email still runs one password verification, so response timing
   doesn't reveal which emails have accounts.
3. ✅ A real `Bun.password` hash verifies (unit test + `tests/auth.rs` fixture). New hashes
   use Bun's cost params (`m=65536,t=2,p=1`).
4. ✅ A session row created outside Rust is accepted. Expired, unknown and non-UUID cookies
   are 401 (Bun returned 500 for a non-UUID cookie).
5. ✅ `create-user` ported. It now rejects unknown roles (the Bun script stored any string).

### Phase 3: Employees, stores, audit (½ day) ✅

1. ✅ Employees CRUD with soft delete and `COALESCE` partial update
   ([handlers/employees.rs](./backend-rs/src/handlers/employees.rs),
   [services/employees.rs](./backend-rs/src/services/employees.rs)). Each write and its audit
   row share one transaction.
2. ✅ `resolve_store_id` and `record_audit` take the caller's transaction.
3. ✅ Migration `0006` (case-insensitive unique store name). Tested on throwaway DBs. **Not
   yet applied to the dev DB:** run `cargo run -- migrate` in `backend-rs/`.
4. ✅ Money serde ([models/money.rs](./backend-rs/src/models/money.rs)): only JSON numbers
   are accepted, parsed from their exact text. More than 2 decimal places is a 400.
5. ✅ Fixed a Bun bug: the employee form sends `email: ""` when the field is blank. Zod
   rejected that, so **creating an employee without an email returned 500**. Blank optional
   strings now count as "not provided". (The bug is still live in the Bun backend until cutover.)
6. ✅ Malformed `/{id}` → 400 `{"error":"Invalid id"}` via the `IdPath` extractor.

### Phase 4: Paycheck logs (1 day) ✅

1. ✅ `payment_calc.rs` + unit tests (Phase 1), now used with exact decimal totals.
2. ✅ Create / update / complete / delete / list / loads
   ([handlers/paycheck_logs.rs](./backend-rs/src/handlers/paycheck_logs.rs),
   [services/paycheck_logs.rs](./backend-rs/src/services/paycheck_logs.rs)). Log, loads,
   stores and audit share **one** transaction. A test forces a failure on the second load
   and checks nothing is left behind.
3. ✅ Snapshot rule: `complete` only flips the status. A later pay-rate change leaves the
   completed totals alone (tested).
4. Fixes over Bun (Rust only):
    - Edit round trip works: `GET …/loads` returns `"YYYY-MM-DD"` and the PUT ignores the
      loads' `id` keys. (Bun returned ISO datetimes that its own Zod `.date()` then
      rejected, so editing a log with existing loads failed.)
    - Duplicate employee + week → **409**, not 500.
    - `weekEnd` before `weekStart` → 400; a total over `numeric(10,2)` → 400, not 500.
    - Blank (whitespace-only) store names → 400.
    - Bad `employeeId`/`from`/`to` query params → 400 (`ApiQuery` extractor).
    - Concurrent edits of one log serialize (`SELECT … FOR UPDATE`).
5. ✅ **Decided (option B):** each log stores the rate its cut was computed at (migration
   `0007`, `employee_paycheck_logs.pay_rate_percent`). Edits recompute at that rate, never
   the employee's current one. Existing logs were backfilled with the employee's current
   rate (the rate an edit would have used anyway). The column stays nullable while Bun can
   still write logs. A NULL falls back to the current rate, which is then stored on the
   next edit. **Phase 8:** backfill any NULLs and `SET NOT NULL`. The API now returns
   `payRatePercent` on each log, and
   [EditPaymentView.vue](./frontend/src/views/EditPaymentView.vue) previews with it
   (falling back to the employee's rate for Bun responses).

### Phase 5: Dashboard + load extraction (½ day) ✅

1. ✅ Dashboard ([handlers/dashboard.rs](./backend-rs/src/handlers/dashboard.rs),
   [services/dashboard.rs](./backend-rs/src/services/dashboard.rs)). Same response shape.
   `COUNT(*)` is a number, sums are exact decimals. The year filter is a date range (can use
   an index) instead of `extract(year …)`. Ties in `byEmployee`/`byMonth` are broken by name,
   so the order is stable.
2. ✅ Extraction ([services/load_extraction.rs](./backend-rs/src/services/load_extraction.rs)),
   raw `POST /v1/messages` with `reqwest`:
    - Model `claude-opus-5-5` (Bun used `claude-opus-5`).
    - **Structured outputs** (`output_config.format` with a JSON schema) replace the
      "respond with ONLY JSON" prompt and `JSON.parse`, so the reply always matches the schema.
    - **`fallbacks: "default"`** (beta header `server-side-fallback-2026-07-01`): if a safety
      classifier declines a screenshot, the API retries on Anthropic's recommended fallback
      model inside the same call. Any remaining refusal or truncation → 502.
    - Amounts are rounded half-up to cents. A negative amount → 502. The values only pre-fill
      the form.
    - Unsupported type / missing file / not multipart → 400; over 10 MB → 413; Claude API
      error or unusable reply → 502 `{"error":"Could not read the screenshot"}`. Bun returned
      500 for all of these.
    - The HTTP client times out after 120 s.
3. Tests use a mock Claude API (wiremock), so `cargo test` makes no real API calls. One
   **manual** check with a real screenshot is still needed before cutover.

### Phase 6: Infra (½ day) ✅

1. ✅ [Dockerfile](./backend-rs/Dockerfile): cargo-chef on **bookworm** (the unsuffixed tags
   are trixie, whose glibc is newer than the distroless `cc-debian12` runtime), `--locked`,
   non-root, `HEALTHCHECK` via `gr-logistics healthcheck` (the image has no shell or curl).
   Built locally with `docker compose build backend` on 2026-10-12.
2. ✅ [docker-compose.yml](./docker-compose.yml): one-shot `migrate` service, and `backend`
   waits for it to succeed. Both use the Rust image.
3. ✅ CI ([ci.yml](./.github/workflows/ci.yml)): new `backend-rs` job runs fmt, clippy and
   test against a Postgres service. The Bun `backend-test` job stays until cutover.
4. ✅ Deploy ([deploy.yml](./.github/workflows/deploy.yml)): new `build-backend-rs-image` job
   publishes `ghcr.io/<repo>-backend-rs`, **a separate image name**, so the Bun image stays
   the production one until Phase 7.
5. ✅ Runtime: `/api/health` checks the DB (503 if unreachable); request timeouts 30 s
   (150 s for extraction, longer than its 120 s Claude API timeout); one INFO log line per
   request with method, path (no query string), status and latency.
6. ✅ `.env.example` parity is a Rust test ([tests/env_example.rs](./backend-rs/tests/env_example.rs)):
   it lists exactly the keys `config.rs` reads, and local `.env.*` files must have all of
   them. This replaces `check-env-parity.ts` for this crate.
7. ⚠️ **Contract suite (Phase 0) was skipped.** The Rust crate has its own 65 tests, but
   nothing yet runs the *same* HTTP tests against both backends. Build it before Phase 7.

### Phase 7: Cutover (1 day + 1 week soak) — in progress

1. ✅ **Shadow read check** ([scripts/shadow-compare.ts](./scripts/shadow-compare.ts)): Bun and
   Rust run side by side against the dev DB, and the script sends identical GETs to both and
   diffs the JSON. On 2026-10-12: **173 requests (every log, every load list, both dashboard
   years, every employee summary): no differences.** Normalized on purpose: ISO datetime vs
   `YYYY-MM-DD` dates, `42` vs `42.00`, tie order in `byMonth`/`byEmployee`. Rust-only fields:
   `payRatePercent` (option B) and `employeeId` on dashboard logs.
   Re-run: start Bun on 3001 (`cd backend && PORT=3001 bun src/index.ts`) and Rust on 3002
   (`cd backend-rs && PORT=3002 cargo run`), then
   `COOKIE='gr_session=<uuid>' bun scripts/shadow-compare.ts`.
2. ✅ **Manual write click-through** on 2026-10-12, frontend against the release Docker image
   (`docker compose up -d migrate backend`, container healthy), with no UI or backend errors:
    - [x] Log in, refresh (session restore), log out
    - [x] Add an employee **with the email left blank** (failed on Bun)
    - [x] Change that employee's pay rate
    - [x] Create a paycheck log with 2+ loads
    - [x] Edit that log: change a load, add one, save (failed on Bun for existing loads)
    - [x] Complete it, change the employee's rate, edit it again: the cut stays at the old rate
    - [x] Delete a log; delete (deactivate) an employee
    - [x] Dashboard: year totals and per-employee view
    - [ ] **Deferred:** screenshot extraction with a real `ANTHROPIC_API_KEY`. Only covered by
      mock-API tests so far. Do one real screenshot before relying on it in production.
3. ✅ **Deploy switched** ([deploy.yml](./.github/workflows/deploy.yml)): the main
   `<repo>-backend` image now builds from `backend-rs/`. The next push to `main` publishes
   Rust as `:latest`. Pre-cutover `sha-…` tags are the Bun builds (the rollback target).
4. **Server cutover runbook** (deploy doesn't restart the server yet, so this is manual):
    1. Add to the server's `.env.production`: `CORS_ORIGIN=<frontend origin>` (exact origin,
       no `*`), and optionally `LOG_FORMAT=json`. `SESSION_SECRET` can be removed.
    2. `docker compose pull`
    3. `docker compose run --rm backend migrate`: applies `0006`/`0007` (both work with the
       Bun image too, so this is safe before the swap).
    4. `docker compose up -d backend`, then check `docker compose ps` shows **healthy** and
       `curl <host>/api/health` returns `{"status":"ok"}`.
    5. Log in from the real frontend. Existing sessions keep working (same cookie and table).
    - **Rollback:** set the backend image to the last pre-cutover `sha-…` tag and
      `docker compose up -d backend`. No schema rollback needed.
5. **Soak for a week.** Watch for any `ERROR` lines, any `status=5xx`, and `status=4xx`
   that used to be 500s (expected: 400/404/409/413/502). Then Phase 8.

### Phase 8: Cleanup (½ day)

1. Delete `backend/`, rename `backend-rs/` → `backend/`.
2. Rewrite the backend sections of [AGENTS.md](./AGENTS.md): stack, "natives" rule, `toNum()`
   rule (obsolete), logging, the "before calling done" commands.
3. Update the [LoadList.vue](./frontend/src/components/payment-processing/LoadList.vue) date comment.
4. Delete this file.

---

## Development

- **Root scripts** ([package.json](./package.json)): `bun run dev` runs the Rust backend
  (`cargo run`, no auto-reload: restart after changes) with the frontend. `test:backend`,
  `migrate` and `create-user` also target Rust. The `*-bun` variants run the old backend.
  For watch mode, `cargo install bacon` and switch `dev:backend` to `bacon run`.
- **Env loading:** Bun auto-loaded `.env.development`. Rust doesn't, so call
  `dotenvy::from_filename(".env.development").ok()` in `main.rs` behind
  `#[cfg(debug_assertions)]`. Prod gets its env from the container.
- **Env keys:** same names as Bun (`NODE_ENV`, `PORT`, `DATABASE_URL`, `SESSION_TTL_HOURS`,
  `ANTHROPIC_API_KEY`, `LOG_LEVEL`, `LOG_FORMAT`), plus `CORS_ORIGIN` and optional
  `ANTHROPIC_BASE_URL`. `SESSION_SECRET` is dropped. `REQUIRED_KEYS`/`OPTIONAL_KEYS` in
  `config.rs` and `.env.example` must match (a test enforces it).
- **SQL is checked at runtime, not compile time.** All queries use `sqlx::query`/`query_as`
  with `'static` SQL strings (sqlx 0.9 rejects non-static SQL, which is its injection guard),
  so builds need no database and there's no `.sqlx/` cache. Integration tests cover every
  query against real Postgres.
- **Logging parity:** keep per-module scopes. `tracing` targets are module paths
  automatically (`gr_logistics::services::stores`). Write a small `FormatEvent` that hashes
  the target with FNV-1a into the same 32-color palette as `logger.ts`, so dev output looks
  the same. JSON format in prod uses `tracing_subscriber::fmt().json()`.
- **"Before calling done" (Rust side):**
    1. `cargo fmt --check`
    2. `cargo clippy --all-targets -- -D warnings`
    3. `bun run test:backend` (loads `.env.development` for `DATABASE_URL`)
    4. Contract suite, once it exists: `API_URL=http://localhost:3000 bun test contract-tests`
- **Compile times:** first build takes about 2–4 min, incremental about 5–15 s. Use
  `mold` (or `lld`) as the linker locally to cut link time.

## Production

- **Dockerfile:** see [backend-rs/Dockerfile](./backend-rs/Dockerfile) (Phase 6). Expect
  roughly 25–40 MB vs the Bun image's roughly 100+ MB. `rustls` everywhere (sqlx, reqwest),
  so there's no OpenSSL in the image.
- **Migrations as a one-shot step,** not at server boot: the `migrate` service in
  [docker-compose.yml](./docker-compose.yml) runs `gr-logistics migrate`, and `backend`
  waits for it to succeed. In production, do the same with the published image.
- **CI** ([ci.yml](./.github/workflows/ci.yml), [deploy.yml](./.github/workflows/deploy.yml)):
    - `dtolnay/rust-toolchain` (reads `rust-toolchain.toml`) + `Swatinem/rust-cache`. Cold
      builds are 5–10 min without the cache.
    - A `postgres:16-alpine` service container for `cargo test`.
    - `build-push-action` context → `./backend-rs`, image `<repo>-backend-rs`, GHA cache
      scope `backend-rs` (so it doesn't evict the Bun image's cache).
- **Runtime:**
    - Graceful shutdown: `axum::serve(listener, app).with_graceful_shutdown(shutdown_signal())`
      on SIGTERM, so in-flight paycheck writes finish on deploy.
    - `PgPoolOptions::new().max_connections(10).acquire_timeout(5s)`. Size it to the Postgres
      `max_connections` minus headroom.
    - `/api/health` checks the DB (`SELECT 1`, 503 on failure). The container `HEALTHCHECK`
      calls it via `gr-logistics healthcheck`.
    - Request timeouts (`tower_http::timeout`): 30 s generally, 150 s on `/api/loads/extract`.
- **Rollback:** the schema and session format are shared, so rollback is "deploy the previous
  image tag." Don't ship a schema migration in the same release as the cutover (`0006` is
  backward-compatible with the Bun code, so it can go first).

## Testing

| Layer                | Tool                                                       | Covers                                                                                 |
| -------------------- | ---------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| Unit                 | `#[cfg(test)]` in `domain/`, `models/money.rs`             | rounding (`120.50→121`, `120.49→120`, `0→0`), 42% and 40% cuts, `sum_loads`, decimal JSON parse (`120.1` → exactly `120.10`, `1.005` rejected) |
| Integration          | `#[sqlx::test(migrations = false)]` + our `migrate::run()` | each test gets a fresh DB; drive `build_app()` with `tower::ServiceExt::oneshot`, no socket |
| Contract (black-box) | `contract-tests/` via `bun test`                           | same suite against Bun (Phase 0) and Rust (Phase 6+), the cutover gate                 |
| External API         | `wiremock` + `ANTHROPIC_BASE_URL`                          | extraction happy path, malformed model JSON → 502, unsupported media type → 400       |
| E2E                  | existing Playwright (`frontend/e2e-tests/`)                | unchanged; runs against whichever backend the dev proxy points to                      |

Integration tests to write first, since they guard the riskiest behavior:

1. **Auth matrix:** every protected route returns 401 without a cookie, 401 with an expired
   session, and 403 for a non-admin role (insert a `role = 'employee'` user directly).
2. **Transaction rollback:** a paycheck create with a bad load (for example, negative pay
   slipping past validation) leaves **no** log, loads, stores or audit row behind.
3. **Snapshot:** complete a log, change the employee's `pay_rate_percent`, re-fetch, and check
   the cut is unchanged.
4. **Migration idempotency:** run `migrate` twice, second run applies nothing. Run two
   concurrently, each file applies exactly once.
5. **Bun hash compatibility:** a fixture hash generated by `Bun.password.hash(..., 'argon2id')` verifies.

Keep the mirrored payment-calc cases in
[frontend/src/utils/payment-calc.ts](./frontend/src/utils/payment-calc.ts)'s tests in sync with
`domain/payment_calc.rs`'s, as AGENTS.md already requires for the TS pair.
