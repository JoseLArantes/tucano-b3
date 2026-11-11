#!/usr/bin/env bash
# Vegeta load test for sustained throughput verification.
#
# Usage: ./scripts/vegeta_test.sh [BASE_URL]

set -euo pipefail

BASE_URL="${1:-http://localhost:8080}"

if command -v vegeta &> /dev/null; then
    VEGETA_CMD="vegeta"
elif command -v docker &> /dev/null; then
    VEGETA_CMD="docker run --rm -i --network host peterevans/vegeta"
else
    echo "ERROR: Neither vegeta nor docker is available"
    exit 1
fi

echo "=== Vegeta: Cached endpoint (30s, 10000 rps) ==="
echo "GET ${BASE_URL}/api/v1/cotahist/tickers" | \
    ${VEGETA_CMD} attack -rate=10000 -duration=30s | \
    ${VEGETA_CMD} report

echo ""
echo "=== Vegeta: Latest quote (30s, 5000 rps) ==="
echo "GET ${BASE_URL}/api/v1/cotahist/tickers/VALE3/latest" | \
    ${VEGETA_CMD} attack -rate=5000 -duration=30s | \
    ${VEGETA_CMD} report

echo ""
echo "=== Vegeta: IBOV history (30s, 1000 rps) ==="
echo "GET ${BASE_URL}/api/v1/ibov/history" | \
    ${VEGETA_CMD} attack -rate=1000 -duration=30s | \
    ${VEGETA_CMD} report

