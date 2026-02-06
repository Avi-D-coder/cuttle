# Docker

- [Local Development](#local-development)
- [Architecture](#architecture)
  - [App Container](#app-container)
  - [Cutthroat Runtime](#cutthroat-runtime)
  - [Postgres Container](#postgres-container)
  - [Diagram](#diagram)


## Local Development

1. Install [Docker Desktop](https://www.docker.com/products/docker-desktop/)
2. Run `npm run docker:start`
3. Go to http://localhost:8080 in your browser (client)
4. JS API is also available on http://localhost:1337

You can connect to the db via `postgresql://cuttlesworth:p4ssw0rd!@database:5432/cuttle` locally, and query or manage migrations with a database administration tool like [DBeaver](https://dbeaver.io/).

## Architecture

### App Container

- Single Docker image runs:
  - Sails.js server (`localhost:1337`)
  - Vite client/proxy (`localhost:8080`)
  - Optional Cutthroat Rust service

### Cutthroat Runtime

- Controlled by `CUTTLE_ENABLE_CUTTHROAT` (default `true`)
- Rust bind address: `RUST_BIND_ADDR` (default `127.0.0.1:4000`)
- Rust auth callback base: `JS_INTERNAL_BASE_URL` (default `http://127.0.0.1:1337`)
- Vite proxy target for Rust: `CUTTLE_RUST_URL` (default `http://127.0.0.1:4000`)
- If Rust is not running, JS services continue and Cutthroat UI is disabled in-app.

### Postgres Container

- Runs Postgres
  - Exposed via http://localhost:5432

### Diagram

![Architectural Diagram](./architecture-diagram.png)

[Diagram](./architecture-diagram.drawio) can be edited on [draw.io](https://app.diagrams.net/).
