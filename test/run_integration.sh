#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

export TEST_DATABASE_URL="${TEST_DATABASE_URL:-postgres://nebula:nebula123@localhost:55432/nebula}"
export TEST_REDIS_URL="${TEST_REDIS_URL:-redis://localhost:36379/0}"
export TEST_JWT_SECRET="${TEST_JWT_SECRET:-integration-test-secret}"
export RUST_TEST_THREADS=1

cd "$ROOT"

compose_cmd=("docker" "compose" "-f" "docker-compose.test.yml")
"${compose_cmd[@]}" up -d --wait --remove-orphans

cleanup() {
  "${compose_cmd[@]}" down -v --remove-orphans
}
trap cleanup EXIT

for suite in auth_flow room_flow; do
  cargo test --test "$suite" "$@"
done
