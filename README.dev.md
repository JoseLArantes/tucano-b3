# tucano-b3 — Developer Guide

## Architecture

See `architecture.md` for the full architecture document.

## Building

```bash
cargo build --release
```

## Testing

```bash
cargo test --all-features --all-targets
```

## Coverage

```bash
cargo tarpaulin --out Html
open coverage/html/index.html
```

100% line + branch coverage is mandatory for all new code.

## Language Policy

- User-facing strings: Brazilian Portuguese
- Code identifiers: English
- Dev docs: English

## Contributing

1. Fork the repository
2. Create a feature branch
3. Write tests (100% coverage required)
4. Run `cargo fmt` and `cargo clippy`
5. Submit a PR

## Release Process

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create tag: `git tag v0.1.0`
4. Push: `git push && git push --tags`
5. CI creates release automatically

## Monitoring

The service exposes Prometheus metrics at `/metrics`.

Key metrics:
- `tucano_http_requests_total` — request count
- `tucano_http_request_duration_seconds` — latency histogram
- `tucano_cache_hits_total` — cache hits
- `tucano_rate_limited_total` — rate limit rejections

