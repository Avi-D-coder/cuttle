# Cutthroat Rust Stack

This workspace hosts the Cutthroat (3-player) Rust engine + server.

## Crates
- `crates/cutthroat_engine`: rules + state machine + tokenlog format
- `crates/cutthroat_server`: Axum API + WS server (in-memory game/lobby state)

## URL/Proxy expectation
Production routing should send:
- `/cutthroat/api/*` → Rust server
- `/cutthroat/ws/*` → Rust server (WS upgrade)
Everything else continues to the JS server.

## Env vars (server)
- `JS_INTERNAL_BASE_URL` (e.g. `http://localhost:1337`)
- `RUST_BIND_ADDR` (e.g. `0.0.0.0:4000`)

## Health endpoint
- `GET /cutthroat/api/v1/health` returns service capability metadata for frontend probing.

## Persistence status
The Cutthroat server keeps active lobbies/games in memory, but finished games are persisted to Postgres.

Required env vars:
- `CUTTHROAT_DATABASE_URL` (preferred) or `DATABASE_URL`

Schema setup options:
- Run migrations manually: `sqlx migrate run --source crates/cutthroat_server/migrations`
- Or set `CUTTHROAT_AUTO_RUN_MIGRATIONS=true` to auto-run migrations on startup

Exception:
- When built with the `e2e-seed` feature, the server may run without a DB.
