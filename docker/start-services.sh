#!/usr/bin/env bash
set -euo pipefail

RUST_ENABLED="${CUTTLE_ENABLE_CUTTHROAT:-true}"
RUST_BIND_ADDR="${RUST_BIND_ADDR:-127.0.0.1:4000}"
JS_INTERNAL_BASE_URL="${JS_INTERNAL_BASE_URL:-http://127.0.0.1:1337}"
CUTTLE_RUST_URL="${CUTTLE_RUST_URL:-http://127.0.0.1:4000}"

RUST_PID=""
SERVER_PID=""
CLIENT_PID=""

cleanup() {
  for pid in "$CLIENT_PID" "$SERVER_PID" "$RUST_PID"; do
    if [[ -n "${pid}" ]] && kill -0 "${pid}" >/dev/null 2>&1; then
      kill "${pid}" >/dev/null 2>&1 || true
    fi
  done
}

trap cleanup INT TERM EXIT

wait_for_database() {
  local host="${DATABASE_HOST:-database}"
  local port="${DATABASE_PORT:-5432}"
  local max_tries=120
  local tries=1

  echo "Waiting for database at ${host}:${port}"
  while (( tries <= max_tries )); do
    if (echo >"/dev/tcp/${host}/${port}") >/dev/null 2>&1; then
      echo "Database is reachable"
      return 0
    fi
    sleep 1
    tries=$((tries + 1))
  done

  echo "WARN: Database not reachable after ${max_tries}s; continuing startup anyway."
  return 1
}

wait_for_database || true

if [[ "${RUST_ENABLED}" == "true" ]]; then
  if [[ -x /usr/local/bin/cutthroat_server ]]; then
    echo "Starting Cutthroat Rust server on ${RUST_BIND_ADDR}"
    RUST_BIND_ADDR="${RUST_BIND_ADDR}" JS_INTERNAL_BASE_URL="${JS_INTERNAL_BASE_URL}" /usr/local/bin/cutthroat_server &
    RUST_PID=$!
    sleep 1
    if ! kill -0 "${RUST_PID}" >/dev/null 2>&1; then
      echo "WARN: Cutthroat server failed to start; continuing with JS services only."
      RUST_PID=""
    fi
  else
    echo "WARN: /usr/local/bin/cutthroat_server not found; continuing with JS services only."
  fi
fi

echo "Starting Sails server"
npm run start:server &
SERVER_PID=$!

echo "Starting Vite client with proxy"
CUTTLE_RUST_URL="${CUTTLE_RUST_URL}" npm run start:client &
CLIENT_PID=$!

wait -n "${SERVER_PID}" "${CLIENT_PID}"
