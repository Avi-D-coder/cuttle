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
The Cutthroat server currently runs fully in memory for lobby/game state.
No Postgres setup or SQL migration step is required at this time.
