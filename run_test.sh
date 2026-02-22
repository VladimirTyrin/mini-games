#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WEB_CLIENT_DIR="${SCRIPT_DIR}/web-client"
WEB_CLIENT_DIST="${WEB_CLIENT_DIR}/dist"
WEB_URL="http://localhost:5000/ui/"

echo "Building web client..."
cd "${WEB_CLIENT_DIR}"
if [[ ! -d "node_modules" ]]; then
    echo "Installing npm dependencies..."
    npm install
fi
npm run build
cd "${SCRIPT_DIR}"

echo "Building server..."
cargo build --release -p mini_games_server

SERVER_BIN="${SCRIPT_DIR}/target/release/mini_games_server"
if [[ -f "${SERVER_BIN}.exe" ]]; then
    SERVER_BIN="${SERVER_BIN}.exe"
fi

echo "Starting server..."
"${SERVER_BIN}" --use-log-prefix --static-files-path "${WEB_CLIENT_DIST}" &
SERVER_PID=$!
sleep 2

open_browser() {
    if command -v xdg-open >/dev/null 2>&1; then
        xdg-open "${WEB_URL}" >/dev/null 2>&1 || true
    elif command -v open >/dev/null 2>&1; then
        open "${WEB_URL}" >/dev/null 2>&1 || true
    elif command -v cmd.exe >/dev/null 2>&1; then
        cmd.exe /c start "" "${WEB_URL}" >/dev/null 2>&1 || true
    else
        echo "Could not auto-open browser. Open ${WEB_URL} manually."
    fi
}

open_browser

echo ""
echo "Test setup running:"
echo "  Server PID: $SERVER_PID"
echo ""
echo "Endpoints:"
echo "  gRPC server:  http://localhost:5001"
echo "  Web client:   ${WEB_URL}"
echo ""
echo "Press Ctrl+C to stop server"

trap "kill $SERVER_PID 2>/dev/null; exit" INT EXIT
wait
