#!/bin/bash
set -e

echo "Building all projects..."
cargo build --release

echo "Starting server..."
./target/release/mini_games_server.exe --use-log-prefix &
SERVER_PID=$!
sleep 2

echo "Starting client 1..."
./target/release/mini_games_client.exe --use-log-prefix --server-address http://localhost:5001 &
CLIENT1_PID=$!

echo "Starting client 2..."
./target/release/mini_games_client.exe --use-log-prefix --server-address http://localhost:5001 &
CLIENT2_PID=$!

echo "Starting client 3..."
./target/release/mini_games_client.exe --use-log-prefix --server-address http://localhost:5001 &
CLIENT3_PID=$!

echo "Test setup running:"
echo "  Server PID: $SERVER_PID"
echo "  Client 1 PID: $CLIENT1_PID"
echo "  Client 2 PID: $CLIENT2_PID"
echo "  Client 3 PID: $CLIENT3_PID"
echo ""
echo "Press Ctrl+C to stop all processes"

trap "kill $SERVER_PID $CLIENT1_PID $CLIENT2_PID $CLIENT3_PID 2>/dev/null; exit" INT
wait
