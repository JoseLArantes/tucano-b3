#!/usr/bin/env bash
# Load test script for tucano-b3.
#
# Usage: ./scripts/load_test.sh [BASE_URL]
#
# Requires: wrk (or Docker), curl, jq

set -euo pipefail

BASE_URL="${1:-http://localhost:8080}"
DURATION="30s"
THREADS=4
CONNECTIONS=100

echo "============================================"
echo "tucano-b3 Load Test"
echo "============================================"
echo "Target:      ${BASE_URL}"
echo "Duration:    ${DURATION}"
echo "Threads:     ${THREADS}"
echo "Connections: ${CONNECTIONS}"
echo "============================================"

# Resolve wrk command (native or docker)
if command -v wrk &> /dev/null; then
    WRK_CMD="wrk"
elif command -v docker &> /dev/null; then
    WRK_CMD="docker run --rm --network host -v $(pwd)/scripts:/scripts williamyeh/wrk"
else
    echo "ERROR: Neither wrk nor docker is available"
    exit 1
fi

# Verify server is up
echo ""
echo "[1/6] Health check..."
curl -sf "${BASE_URL}/api/v1/health" > /dev/null || {
    echo "ERROR: Server not responding at ${BASE_URL}"
    exit 1
}
echo "  ✓ Server is up"

# Test 1: Cached endpoint throughput
echo ""
echo "[2/6] Throughput test: /api/v1/cotahist/tickers (cached)..."
${WRK_CMD} -t${THREADS} -c${CONNECTIONS} -d${DURATION} \
    --latency \
    "${BASE_URL}/api/v1/cotahist/tickers" \
    | tee /tmp/wrk_cached.txt

# Extract metrics
RPS=$(grep "Requests/sec" /tmp/wrk_cached.txt | awk '{print $2}')
P50=$(grep "50%" /tmp/wrk_cached.txt | awk '{print $2}')
P99=$(grep "99%" /tmp/wrk_cached.txt | awk '{print $2}')

echo ""
echo "  Results:"
echo "    Requests/sec: ${RPS}"
echo "    p50 latency:  ${P50}"
echo "    p99 latency:  ${P99}"

# Verify targets
RPS_INT=${RPS%.*}
if [ "${RPS_INT}" -lt 10000 ]; then
    echo "  ✗ FAIL: Throughput ${RPS} < 10000 req/s target"
else
    echo "  ✓ PASS: Throughput meets target"
fi

# Test 2: Cold endpoint latency
echo ""
echo "[3/6] Latency test: /api/v1/cotahist/tickers/VALE3/latest (cold)..."
${WRK_CMD} -t2 -c10 -d10s \
    --latency \
    -s scripts/wrk_unique.lua \
    "${BASE_URL}/api/v1/cotahist/tickers/VALE3/quotes" \
    | tee /tmp/wrk_cold.txt

# Test 3: IBOV endpoint
echo ""
echo "[4/6] IBOV endpoint test..."
${WRK_CMD} -t2 -c50 -d10s \
    --latency \
    "${BASE_URL}/api/v1/ibov/history?dias=30" \
    | tee /tmp/wrk_ibov.txt

# Test 4: Health endpoint (should be fast)
echo ""
echo "[5/6] Health endpoint test..."
${WRK_CMD} -t1 -c10 -d5s \
    --latency \
    "${BASE_URL}/api/v1/health" \
    | tee /tmp/wrk_health.txt

# Test 5: Metrics endpoint
echo ""
echo "[6/6] Metrics endpoint test..."
START=$(date +%s%N)
curl -sf "${BASE_URL}/metrics" > /dev/null
END=$(date +%s%N)
METRICS_MS=$(( (END - START) / 1000000 ))
echo "  Metrics response time: ${METRICS_MS}ms"

echo ""
echo "============================================"
echo "Load test complete"
echo "============================================"
