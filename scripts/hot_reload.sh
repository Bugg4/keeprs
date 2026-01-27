#!/bin/bash
# hot_reload.sh

# Name of the binary
BIN_NAME="keeprs"

# Kill any running instance of the application
# We use -f to match the full command line to ensure we catch 'target/debug/keeprs'
pkill -f "target/debug/$BIN_NAME" || true

# Wait a moment to ensure it's gone
sleep 0.1

# Start the new instance in the background, fully detached.
# We use setsid to detach from the current terminal group so cargo-watch killing the script doesn't kill the app.
setsid cargo run -p keeprs-gui > /tmp/keeprs.log 2>&1 &

echo "Restarted keeprs (PID $!)"
