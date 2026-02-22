#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}"

echo "=== Building all components ==="

export RUSTFLAGS="-D warnings"

echo ""
echo "=== Running clippy ==="
cargo clippy --all-targets --all-features -- -D warnings

echo ""
echo "=== Building server ==="
cargo build -p mini_games_server

echo ""
echo "=== Building benchmarks ==="
cargo build --benches -p mini_games_server

echo ""
echo "=== Running tests ==="
cargo test

echo ""
echo "=== Building web client ==="
cd "${SCRIPT_DIR}/web-client"
npm ci
npm run build
cd "${SCRIPT_DIR}"

echo ""
echo "=== All builds completed successfully ==="

if [[ "${1:-}" == "--run" ]]; then
  echo ""
  echo "=== Starting dev environment ==="
  cargo run -p mini_games_server &
  SERVER_PID=$!
  cd "${SCRIPT_DIR}/web-client"
  npm run dev &
  DEV_PID=$!

  sleep 2

  if command -v xdg-open &>/dev/null; then
    xdg-open "http://localhost:5173"
  elif command -v open &>/dev/null; then
    open "http://localhost:5173"
  elif command -v start &>/dev/null; then
    start "http://localhost:5173"
  fi

  trap "kill $SERVER_PID $DEV_PID 2>/dev/null" EXIT
  wait
fi
