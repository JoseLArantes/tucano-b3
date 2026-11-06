ALTER TABLE b3_source_files ADD COLUMN acquisition_channel TEXT NOT NULL DEFAULT 'unreviewed';
ALTER TABLE b3_source_files ADD COLUMN job_id UUID REFERENCES b3_jobs(id);
ALTER TABLE b3_source_files ADD COLUMN job_generation BIGINT;
ALTER TABLE b3_source_files ADD COLUMN replayed_from_file_id VARCHAR(36) REFERENCES b3_source_files(id);
ALTER TABLE b3_source_file_members ADD COLUMN parent_index INT;
ALTER TABLE b3_source_file_members ADD COLUMN local_path TEXT;
ALTER TABLE b3_source_file_members ADD COLUMN selected BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE b3_source_file_members ADD COLUMN exclusion_reason TEXT;

CREATE TABLE b3_source_channels (
    id TEXT PRIMARY KEY,
    source_file_type TEXT NOT NULL,
    acquisition_mode TEXT NOT NULL CHECK (acquisition_mode IN ('https','local')),
    source_url TEXT,
    local_relative_path TEXT,
    enabled BOOLEAN NOT NULL DEFAULT FALSE,
    discovery_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    publication_key TEXT NOT NULL,
    format_profile TEXT NOT NULL DEFAULT 'unreviewed',
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    official_revision BIGINT,
    replaces_publication BOOLEAN NOT NULL DEFAULT FALSE,
    CHECK (period_start <= period_end),
    CHECK ((acquisition_mode='https' AND source_url IS NOT NULL AND local_relative_path IS NULL) OR
           (acquisition_mode='local' AND local_relative_path IS NOT NULL AND source_url IS NULL))
);

CREATE TABLE b3_worker_heartbeats (
    worker_id UUID PRIMARY KEY,
    lane TEXT NOT NULL,
    heartbeat_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_discovery_at TIMESTAMPTZ
);
CREATE TABLE b3_discovery_checks (
    id UUID PRIMARY KEY,
    channel_id TEXT NOT NULL REFERENCES b3_source_channels(id),
    scheduled BOOLEAN NOT NULL,
    checked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    outcome TEXT NOT NULL CHECK(outcome IN ('available','absent','inconclusive','ineligible','calendar_unknown','failed')),
    etag TEXT,
    last_modified TEXT,
    content_length BIGINT,
    error_code TEXT
);
ALTER TABLE b3_pending_updates ADD COLUMN channel_id TEXT REFERENCES b3_source_channels(id);
CREATE UNIQUE INDEX uq_b3_pending_channel ON b3_pending_updates(channel_id) WHERE status='pending' AND channel_id IS NOT NULL;

CREATE TABLE b3_calendar_evidence (
    id UUID PRIMARY KEY,
    source_reference TEXT NOT NULL,
    sha256 VARCHAR(64) NOT NULL CHECK(length(sha256)=64),
    observed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
ALTER TABLE b3_trading_calendar ADD COLUMN evidence_id UUID REFERENCES b3_calendar_evidence(id);
ALTER TABLE b3_trading_calendar ADD COLUMN session_close TIME;
CREATE TABLE b3_job_outputs (
    job_id UUID NOT NULL REFERENCES b3_jobs(id),
    output_kind TEXT NOT NULL CHECK(output_kind IN ('source_file','revision','identity_rebuild')),
    output_id TEXT NOT NULL,
    PRIMARY KEY(job_id,output_kind,output_id)
);
