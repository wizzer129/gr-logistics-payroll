# GR Logistics

Employee payment tracking portal. Conventions live in [AGENTS.md](./AGENTS.md); the original
design is in [PROJECT_PLAN.md](./PROJECT_PLAN.md).

- `frontend/` — Vue 3 PWA (Vite, Bun)
- `backend/` — Rust API (axum + sqlx + Postgres)

## Local development

Needs Bun 1.4.3, Rust 1.99 (rustup reads `backend/rust-toolchain.toml`), and Docker for Postgres.

```bash
bun install

# Env comes from backend/.env.schema via varlock; secrets live in Proton Pass.
pass-cli login          # once — varlock reads secrets through the pass-cli session
bun run env:check       # validates every key resolves
docker compose up -d db
bun run migrate
bun run create-user --email admin@example.com --password changeme

bun run dev             # backend (:3000) + frontend (:5173) concurrently
# or individually:
bun run dev:backend     # http://localhost:3000 — `cargo run`; restart after Rust changes
bun run dev:frontend    # http://localhost:5173 — hot reload
```

To run the backend from its release Docker image instead:
`bunx --bun varlock run -- docker compose up -d migrate backend` (applies
migrations, then serves on :3000; varlock supplies the env compose passes through).

## Testing

```bash
bun run test:backend    # cargo test — unit + integration tests against throwaway Postgres DBs
bun run test:frontend   # vitest
bun run --cwd frontend e2e
```
