#!/usr/bin/env python3
"""Measure authenticated COTAHIST REST reads with a bounded thread pool."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
import math
import statistics
import time
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode
from urllib.request import Request, urlopen


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-url", required=True, help="API origin, e.g. http://127.0.0.1:8010")
    parser.add_argument("--token", required=True, help="Read-only Bearer token")
    parser.add_argument("--requests", type=int, default=160)
    parser.add_argument("--concurrency", type=int, default=8)
    parser.add_argument("--limit", type=int, default=100)
    parser.add_argument("--timeout-seconds", type=float, default=30.0)
    args = parser.parse_args()
    if args.requests < 1 or args.concurrency < 1 or not 1 <= args.limit <= 1000:
        parser.error("requests/concurrency must be positive and limit must be 1..1000")

    url = args.base_url.rstrip("/") + "/api/v1/b3/cotahist/quotes?" + urlencode(
        {"limit": args.limit, "offset": 0}
    )

    def request_once() -> tuple[float, int]:
        request = Request(url, headers={"Authorization": f"Bearer {args.token}"})
        started = time.perf_counter()
        try:
            with urlopen(request, timeout=args.timeout_seconds) as response:
                body = json.load(response)
                if response.status != 200 or not body.get("items"):
                    raise RuntimeError("API returned no queryable rows")
                return (time.perf_counter() - started) * 1000, len(body["items"])
        except (HTTPError, URLError) as error:
            raise RuntimeError("REST read benchmark failed; check API/auth and target rows") from error

    started = time.perf_counter()
    with ThreadPoolExecutor(max_workers=args.concurrency) as pool:
        results = list(pool.map(lambda _: request_once(), range(args.requests)))
    elapsed = time.perf_counter() - started
    latencies = sorted(duration for duration, _ in results)
    p50 = statistics.median(latencies)
    p95 = latencies[math.ceil(0.95 * len(latencies)) - 1]
    rows = sum(row_count for _, row_count in results)
    print(
        f"requests={len(results)} concurrency={args.concurrency} rows_returned={rows} "
        f"elapsed_seconds={elapsed:.2f} throughput_rps={len(results) / elapsed:.2f} "
        f"p50_ms={p50:.2f} p95_ms={p95:.2f}"
    )


if __name__ == "__main__":
    main()
