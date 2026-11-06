-- Durable execution foundation. No source is eligible by default.
CREATE TABLE b3_source_eligibility (
    source_file_type VARCHAR(64) NOT NULL,
    purpose VARCHAR(32) NOT NULL CHECK (purpose IN ('acquisition', 'raw_retention', 'processing', 'rest', 'mcp', 'third_party_delivery')),
    status VARCHAR(16) NOT NULL DEFAULT 'unreviewed' CHECK (status IN ('unreviewed', 'eligible', 'restricted', 'ineligible')),
    zero_cost BOOLEAN NOT NULL DEFAULT FALSE,
    evidence_reference TEXT,
    verified_at TIMESTAMPTZ,
    PRIMARY KEY (source_file_type, purpose),
    CHECK (status <> 'eligible' OR (zero_cost AND evidence_reference IS NOT NULL AND verified_at IS NOT NULL))
);

CREATE TABLE b3_jobs (
    id UUID PRIMARY KEY,
    idempotency_key TEXT NOT NULL UNIQUE,
    scope_key TEXT NOT NULL,
    command JSONB NOT NULL,
    command_hash VARCHAR(64) NOT NULL,
    priority TEXT NOT NULL CHECK (priority IN ('maintenance', 'backfill', 'rebuild')),
    status TEXT NOT NULL DEFAULT 'queued' CHECK (status IN ('queued', 'running', 'succeeded', 'succeeded_with_issues', 'failed', 'cancelled')),
    attempt INT NOT NULL DEFAULT 0,
    max_attempts INT NOT NULL CHECK (max_attempts BETWEEN 1 AND 100),
    generation BIGINT NOT NULL DEFAULT 0,
    owner_id UUID,
    lease_expires_at TIMESTAMPTZ,
    heartbeat_at TIMESTAMPTZ,
    cancel_requested_at TIMESTAMPTZ,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    error_code TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at TIMESTAMPTZ,
    CHECK ((status = 'running') = (owner_id IS NOT NULL AND lease_expires_at IS NOT NULL))
);
CREATE UNIQUE INDEX uq_b3_jobs_active_scope ON b3_jobs(scope_key) WHERE status IN ('queued', 'running');
CREATE INDEX idx_b3_jobs_claim ON b3_jobs(priority, next_attempt_at, created_at, id) WHERE status IN ('queued', 'running');

CREATE TABLE b3_job_attempts (
    job_id UUID NOT NULL REFERENCES b3_jobs(id),
    generation BIGINT NOT NULL,
    owner_id UUID NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at TIMESTAMPTZ,
    outcome TEXT CHECK (outcome IN ('succeeded', 'succeeded_with_issues', 'failed', 'cancelled', 'expired', 'retry')),
    PRIMARY KEY (job_id, generation)
);
