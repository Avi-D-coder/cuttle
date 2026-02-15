# Docker

- [Local Development](#local-development)
- [Single Image (DigitalOcean)](#single-image-digitalocean)
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

## Single Image (DigitalOcean)

Use this when you want one image containing:
- Nginx (public entrypoint)
- Sails/Node.js
- Rust Cutthroat server
- Postgres

Build:

```bash
npm run docker:build:single
```

Run locally:

```bash
npm run docker:start:single
```

Or explicitly:

```bash
docker run --rm \
  -p 8080:80 \
  -v "$(pwd)/docker/data/pgdata-single:/var/lib/postgresql/data" \
  -e PORT=80 \
  -e NODE_ENV=staging \
  -e POSTGRES_USER=cuttlesworth \
  -e POSTGRES_PASSWORD=p4ssw0rd \
  -e POSTGRES_DB=cuttle \
  -e CUTTLE_ENABLE_CUTTHROAT=true \
  -e CUTTHROAT_AUTO_RUN_MIGRATIONS=true \
  cuttle-all-in-one
```

Push to DigitalOcean Container Registry:

```bash
docker tag cuttle-all-in-one registry.digitalocean.com/<registry-name>/cuttle-all-in-one:latest
docker push registry.digitalocean.com/<registry-name>/cuttle-all-in-one:latest
```

Notes:
- This is intentionally single-container and stateful; Postgres data is inside the container filesystem.
- For local runs, mount `./docker/data/pgdata-single` so Postgres can initialize reliably and persist while the container is running.
- If the container is replaced/redeployed without a mounted volume, database data is lost.
- For this image, expose one port only (`PORT`, default `80`) through Nginx.

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
