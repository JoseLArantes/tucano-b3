---
status: accepted
date: 2026-11-08
---

# Configurable metadata-only discovery locations

## Context

The original discovery scanner checked only enabled, database-registered source channels. Operators also need to locate local COTAHIST archives and inspect configured official B3 or S3-compatible locations before registering an acquisition channel. Discovery must remain lightweight and must not turn an observation into a download, ingestion, eligibility decision, or publication.

## Decision

Add `B3_DISCOVERY_LOCATIONS` as a comma- or semicolon-separated `kind:value` list. Supported kinds are `folder`, `b3_url` (with `host` as a compatibility alias), and `object_storage_bucket`. Unknown kinds and invalid values fail configuration validation.

- Folders require absolute paths, are scanned to bounded depth and entry count, do not follow symlinks, and expose only a root-relative locator.
- B3 locations are direct HTTPS URLs to named COTAHIST `.TXT`/`.ZIP` artifacts. They use metadata-only `HEAD` through the existing exact host, DNS, redirect, and timeout validation. Index pages, query strings, fragments, credentials, and unapproved hosts are rejected.
- S3-compatible locations use `s3://bucket/prefix`, the AWS default credential chain, optional `AWS_ENDPOINT_URL_S3`, and path-style addressing. Discovery uses paginated `ListObjectsV2` only; it does not issue `GetObject`.
- The local Compose default scans `./data/imports`, mounted read-only at `/data/raw/imports`. It never defaults to the whole `./data`, which may contain PostgreSQL files.
- Each location check and its bounded candidate metadata are persisted in dedicated discovery tables. The manual scan response returns an aggregate and at most 1,000 candidate DTOs. These observations do not create channel-backed pending updates or jobs.

## Consequences

Operators can inspect local, official B3, and S3-compatible COTAHIST locations using the existing administrative scan operation. Discovery still never creates channels or eligibility. A separate administrative action may register a candidate from the latest manual scan only when it is in the configured local imports folder and the maintainer explicitly attests to internal-development use. That action grants only manual acquisition, raw retention, and processing for the one local channel; it does not grant REST, MCP, or third-party delivery. An ingest command may carry the registered channel ID to bind the job to exactly the selected candidate. S3 credentials require list-only access to the intended bucket/prefix; ETags are not content checksums. HTML scraping and generic web locations are intentionally unsupported.
