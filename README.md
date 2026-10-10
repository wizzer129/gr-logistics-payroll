# GR Logistics

Employee payment tracking portal. See [PROJECT_PLAN.md](./PROJECT_PLAN.md) for the full design.

## Local development

```bash
bun install

cp backend/.env.example backend/.env.development   # already present with dev defaults
docker compose up -d db
bun run --cwd backend migrate
bun run --cwd backend create-user --email admin@example.com --password changeme

bun run dev:backend     # http://localhost:3000
bun run dev:frontend    # http://localhost:5173
```

## Testing

```bash
bun run test:backend    # bun test
bun run test:frontend   # vitest
bun run --cwd frontend e2e
```
