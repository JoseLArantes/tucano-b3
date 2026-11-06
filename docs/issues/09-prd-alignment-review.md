# 09 — Review implementation alignment and establish the migration baseline

Status: ready-for-agent

Date: 2026-11-06. Milestones: R0–R6 in the [review plan](../19-plan.md). Dependencies: none for offline review. Docker/PostgreSQL availability and source permissions constrain particular evidence, not the ability to produce the assessment.

## Problem

The accepted feature backlog needs a current, reproducible assessment of the existing code before implementation changes are scoped. Preliminary PRD observations may be incomplete or superseded; documentation claims are not runtime evidence.

## Scope

- Execute R0–R6 from `docs/19-plan.md`, recording commit, dirty-tree state, PRD identity and validation environment without secrets.
- Inventory effective REST/OpenAPI, MCP, CLI, schema, ingestion and worker behavior, including known compatibility obligations.
- Review acquisition, parsers, raw recovery, financial precision, identity windows, idempotency, audit atomicity, counters, locking and publication visibility.
- Run existing deterministic checks with isolated PostgreSQL 16 through Compose when available; inspect and report database-dependent tests that return early.
- Create the requirement matrix in `.scratch/prd-alignment/assessment.md`, with source symbols, evidence, alignment, confidence, severity and retain/extend/replace disposition.
- Reconcile findings with [issues 01–08](README.md), identify scoped remediation and document migration/cutover requirements. New accepted issues belong in `docs/issues/` using the next available number.

## Acceptance criteria

- Every PRD subsystem has traceable assessment coverage and an explicit evidence level. Runtime invariants supported only by static inspection remain unverified.
- Preliminary findings are confirmed, corrected or rejected based on the recorded baseline; recent OpenAPI work is reviewed afresh.
- Each confirmed integrity or recovery blocker has a concrete scenario, affected scope, dependency, compatibility impact and proposed validation.
- Existing tests are reported with actual exit results and database execution limits. Missing infrastructure and unavailable external evidence are explicit, without claiming compliance.
- Reuse recommendations preserve the stack, source-family boundaries, temporal identity, provenance, precision and audit history.
- The deliverable identifies the next executable implementation slice and the entry conditions needed to release dependent feature issues.

## Validation

Use the commands and isolation rules in [plan section 8](../19-plan.md#8-validation-protocol). Do not point tests at an operational database or start the worker with uncontrolled live acquisition. Report unavailable checks and continue independent review.

This issue is an assessment delivery. Do not change production code, dependencies, migrations or public contracts as an incidental fix during review. Record missing regression tests as remediation work; deeper failure-injection experiments requiring code changes receive a scoped follow-up issue.

## Out of scope

Live historical backfills, contacting the B3 without explicit authorization, paid services, production migrations, publishing data, automatic commits, and executing the feature implementations in issues 02–08.

## Comments

- 2026-11-06 — Added while promoting accepted research, glossary and issues into permanent project documentation. Ready for an agent to execute the existing review plan; downstream implementation prerequisites remain open.
