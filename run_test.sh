#!/bin/bash
set -e

WEB_ONLY=false
if [[ "$1" == "--web" ]]; then
    WEB_ONLY=true
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WEB_CLIENT_DIR="${SCRIPT_DIR}/web-client"
WEB_CLIENT_DIST="${WEB_CLIENT_DIR}/dist"

echo "Building web client..."
cd "${WEB_CLIENT_DIR}"
if [[ ! -d "node_modules" ]]; then
    echo "Installing npm dependencies..."
    npm install
fi
npm run build
cd "${SCRIPT_DIR}"

echo "Building Rust projects..."
cargo build --release

echo "Starting server..."
./target/release/mini_games_server.exe --use-log-prefix --static-files-path "${WEB_CLIENT_DIST}" &
SERVER_PID=$!
sleep 2

PIDS_TO_KILL="$SERVER_PID"

if [[ "$WEB_ONLY" == false ]]; then
    echo "Starting client 1..."
    ./target/release/mini_games_client.exe --use-log-prefix --server-address http://localhost:5001 --random-client-id &
    CLIENT1_PID=$!
    PIDS_TO_KILL="$PIDS_TO_KILL $CLIENT1_PID"

    echo "Starting client 2..."
    ./target/release/mini_games_client.exe --use-log-prefix --server-address http://localhost:5001 --random-client-id &
    CLIENT2_PID=$!
    PIDS_TO_KILL="$PIDS_TO_KILL $CLIENT2_PID"
fi

echo ""
echo "Test setup running:"
echo "  Server PID: $SERVER_PID"
if [[ "$WEB_ONLY" == false ]]; then
    echo "  Client 1 PID: $CLIENT1_PID"
    echo "  Client 2 PID: $CLIENT2_PID"
fi
echo ""
echo "Endpoints:"
echo "  gRPC server:  http://localhost:5001"
echo "  Web client:   http://localhost:5000/ui/"
echo ""
echo "Press Ctrl+C to stop all processes"

trap "kill $PIDS_TO_KILL 2>/dev/null; exit" INT
wait
