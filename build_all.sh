#!/usr/bin/env bash
set -euo pipefail

echo "=== Building all components ==="

export RUSTFLAGS="-D warnings"

echo ""
echo "=== Running clippy ==="
cargo clippy --all-targets --all-features -- -D warnings

echo ""
echo "=== Building server ==="
cargo build -p mini_games_server

echo ""
echo "=== Building desktop client ==="
cargo build -p mini_games_client

echo ""
echo "=== Building benchmarks ==="
cargo build --benches -p common

echo ""
echo "=== Running tests ==="
cargo test

echo ""
echo "=== Building web client ==="
cd web-client
npm ci
npm run build

echo ""
echo "=== All builds completed successfully ==="
