#!/bin/bash

# Build all projects
echo "Building all projects..."
cargo build --release

# Start server in background
echo "Starting server..."
./target/release/snake_game_server.exe --use-log-prefix &
SERVER_PID=$!
sleep 1

# Start first client in background
echo "Starting client 1..."
./target/release/snake_game_client.exe --use-log-prefix &
CLIENT1_PID=$!

# Start second client in background
echo "Starting client 2..."
./target/release/snake_game_client.exe --use-log-prefix &
CLIENT2_PID=$!

echo "Test setup running:"
echo "  Server PID: $SERVER_PID"
echo "  Client 1 PID: $CLIENT1_PID"
echo "  Client 2 PID: $CLIENT2_PID"
echo ""
echo "Press Ctrl+C to stop all processes"

# Wait for Ctrl+C
trap "kill $SERVER_PID $CLIENT1_PID $CLIENT2_PID 2>/dev/null; exit" INT
wait
