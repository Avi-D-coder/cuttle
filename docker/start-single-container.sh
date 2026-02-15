#!/usr/bin/env bash
set -euo pipefail

APP_DIR="/cuttle"
POSTGRES_HOST="${POSTGRES_HOST:-127.0.0.1}"
POSTGRES_PORT="${POSTGRES_PORT:-5432}"
POSTGRES_DB="${POSTGRES_DB:-cuttle}"
POSTGRES_USER="${POSTGRES_USER:-cuttlesworth}"
POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-p4ssw0rd}"
APP_PORT="${APP_PORT:-1337}"
PUBLIC_PORT="${PORT:-80}"
RUST_ENABLED="${CUTTLE_ENABLE_CUTTHROAT:-true}"
RUST_BIND_ADDR="${RUST_BIND_ADDR:-127.0.0.1:4000}"
JS_INTERNAL_BASE_URL="${JS_INTERNAL_BASE_URL:-http://127.0.0.1:${APP_PORT}}"
CUTTLE_RUST_URL="${CUTTLE_RUST_URL:-http://127.0.0.1:4000}"
CUTTHROAT_AUTO_RUN_MIGRATIONS="${CUTTHROAT_AUTO_RUN_MIGRATIONS:-true}"
APP_NODE_ENV="${NODE_ENV:-staging}"
POSTGRES_BIN_DIR="${POSTGRES_BIN_DIR:-}"

if [[ -z "${POSTGRES_BIN_DIR}" ]]; then
  POSTGRES_BIN_DIR="$(find /usr/lib/postgresql -mindepth 2 -maxdepth 2 -type d -name bin | sort | tail -n 1)"
fi

if [[ ! -d "${POSTGRES_BIN_DIR}" ]]; then
  echo "Could not locate Postgres bin directory"
  exit 1
fi

export CUTTLE_DOCKERIZED=true
export DATABASE_URL="${DATABASE_URL:-postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@${POSTGRES_HOST}:${POSTGRES_PORT}/${POSTGRES_DB}}"

sql_escape_literal() {
  printf "%s" "$1" | sed "s/'/''/g"
}

POSTGRES_PID=""
RUST_PID=""
NODE_PID=""
NGINX_PID=""

cleanup() {
  for pid in "$NGINX_PID" "$NODE_PID" "$RUST_PID"; do
    if [[ -n "${pid}" ]] && kill -0 "${pid}" >/dev/null 2>&1; then
      kill "${pid}" >/dev/null 2>&1 || true
    fi
  done

  if [[ -n "${POSTGRES_PID}" ]] && kill -0 "${POSTGRES_PID}" >/dev/null 2>&1; then
    kill "${POSTGRES_PID}" >/dev/null 2>&1 || true
  fi
}

trap cleanup INT TERM EXIT

mkdir -p "${PGDATA}"
chown -R postgres:postgres "${PGDATA}"

first_init="false"
if [[ ! -f "${PGDATA}/PG_VERSION" ]]; then
  echo "Initializing Postgres data directory at ${PGDATA}"
  gosu postgres "${POSTGRES_BIN_DIR}/initdb" -D "${PGDATA}"
  first_init="true"
fi

echo "Starting Postgres on ${POSTGRES_HOST}:${POSTGRES_PORT}"
gosu postgres "${POSTGRES_BIN_DIR}/postgres" -D "${PGDATA}" -h "${POSTGRES_HOST}" -p "${POSTGRES_PORT}" &
POSTGRES_PID=$!

echo "Waiting for Postgres to become ready"
for _ in $(seq 1 60); do
  if gosu postgres "${POSTGRES_BIN_DIR}/pg_isready" -h "${POSTGRES_HOST}" -p "${POSTGRES_PORT}" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

if ! gosu postgres "${POSTGRES_BIN_DIR}/pg_isready" -h "${POSTGRES_HOST}" -p "${POSTGRES_PORT}" >/dev/null 2>&1; then
  echo "Postgres failed to become ready"
  exit 1
fi

echo "Ensuring Postgres role/database exist"
gosu postgres "${POSTGRES_BIN_DIR}/psql" \
  -h "${POSTGRES_HOST}" \
  -p "${POSTGRES_PORT}" \
  -v ON_ERROR_STOP=1 \
  --set=app_user="$(sql_escape_literal "${POSTGRES_USER}")" \
  --set=app_password="$(sql_escape_literal "${POSTGRES_PASSWORD}")" \
  --set=app_db="$(sql_escape_literal "${POSTGRES_DB}")" \
  postgres <<'SQL'
SELECT format('CREATE ROLE %I LOGIN PASSWORD %L', :'app_user', :'app_password')
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = :'app_user') \gexec

SELECT format('CREATE DATABASE %I OWNER %I', :'app_db', :'app_user')
WHERE NOT EXISTS (SELECT 1 FROM pg_database WHERE datname = :'app_db') \gexec
SQL

sed "s/__NGINX_PORT__/${PUBLIC_PORT}/g" /etc/nginx/conf.d/default.conf.template >/etc/nginx/conf.d/default.conf

if [[ "${RUST_ENABLED}" == "true" ]]; then
  if [[ -x /usr/local/bin/cutthroat_server ]]; then
    echo "Starting Cutthroat Rust server at ${RUST_BIND_ADDR}"
    RUST_BIND_ADDR="${RUST_BIND_ADDR}" \
      JS_INTERNAL_BASE_URL="${JS_INTERNAL_BASE_URL}" \
      CUTTHROAT_AUTO_RUN_MIGRATIONS="${CUTTHROAT_AUTO_RUN_MIGRATIONS}" \
      /usr/local/bin/cutthroat_server &
    RUST_PID=$!
  else
    echo "WARN: /usr/local/bin/cutthroat_server not found; skipping Cutthroat"
  fi
fi

echo "Starting Sails server on ${APP_PORT}"
cd "${APP_DIR}"
NODE_ENV="${APP_NODE_ENV}" PORT="${APP_PORT}" CUTTLE_RUST_URL="${CUTTLE_RUST_URL}" npm run start:server &
NODE_PID=$!

echo "Starting Nginx on ${PUBLIC_PORT}"
nginx -g 'daemon off;' &
NGINX_PID=$!

if [[ -n "${RUST_PID}" ]]; then
  wait -n "${POSTGRES_PID}" "${RUST_PID}" "${NODE_PID}" "${NGINX_PID}"
else
  wait -n "${POSTGRES_PID}" "${NODE_PID}" "${NGINX_PID}"
fi
