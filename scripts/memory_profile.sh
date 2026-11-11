#!/usr/bin/env bash
# Memory profiling script for tucano-b3.
#
# Usage: ./scripts/memory_profile.sh

set -euo pipefail

echo "=== Memory Profile: tucano-b3 ==="

# Build release
cargo build --release

# Start server
./target/release/tucano-b3 --config config.toml &
SERVER_PID=$!
sleep 3

# Cleanup on exit
trap 'kill $SERVER_PID 2>/dev/null || true' EXIT

# Get initial RSS
RSS_START=$(ps -o rss= -p $SERVER_PID)
echo "Initial RSS: $((RSS_START / 1024)) MB"

# Generate load to populate cache
for i in $(seq 1 100); do
    curl -s http://localhost:8080/api/v1/cotahist/tickers > /dev/null
    curl -s http://localhost:8080/api/v1/ibov/history > /dev/null
done
sleep 2

# Get RSS after load
RSS_LOADED=$(ps -o rss= -p $SERVER_PID)
echo "RSS after load: $((RSS_LOADED / 1024)) MB"

# Sustained load
for i in $(seq 1 1000); do
    curl -s http://localhost:8080/api/v1/cotahist/tickers > /dev/null &
    if [ $((i % 50)) -eq 0 ]; then
        wait
    fi
done
wait
sleep 2

# Get RSS after sustained load
RSS_SUSTAINED=$(ps -o rss= -p $SERVER_PID)
echo "RSS after sustained load: $((RSS_SUSTAINED / 1024)) MB"

# Cleanup
kill $SERVER_PID
wait $SERVER_PID 2>/dev/null || true
trap - EXIT

# Verify target
if [ $((RSS_SUSTAINED / 1024)) -gt 512 ]; then
    echo "✗ FAIL: Memory $((RSS_SUSTAINED / 1024)) MB > 512 MB target"
    exit 1
else
    echo "✓ PASS: Memory within target"
fi

