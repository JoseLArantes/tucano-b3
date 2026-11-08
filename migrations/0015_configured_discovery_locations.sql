CREATE TABLE b3_discovery_location_checks (
    id UUID PRIMARY KEY,
    location_kind TEXT NOT NULL CHECK (location_kind IN ('folder', 'b3_url', 'object_storage_bucket')),
    location_label TEXT NOT NULL,
    scheduled BOOLEAN NOT NULL,
    checked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    outcome TEXT NOT NULL CHECK (outcome IN ('available', 'absent', 'inconclusive', 'failed')),
    candidate_count INT NOT NULL CHECK (candidate_count >= 0),
    truncated BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX ix_b3_discovery_location_checks_checked
    ON b3_discovery_location_checks (checked_at DESC, id);

CREATE TABLE b3_discovery_location_candidates (
    check_id UUID NOT NULL REFERENCES b3_discovery_location_checks(id) ON DELETE CASCADE,
    source_file_type VARCHAR(64) NOT NULL CHECK (source_file_type IN ('COTAHIST_YEARLY', 'COTAHIST_MONTHLY', 'COTAHIST_DAILY')),
    source_locator TEXT NOT NULL,
    remote_etag TEXT,
    remote_last_modified TEXT,
    content_length BIGINT CHECK (content_length IS NULL OR content_length >= 0),
    PRIMARY KEY (check_id, source_locator)
);

CREATE INDEX ix_b3_discovery_location_candidates_file_type
    ON b3_discovery_location_candidates (source_file_type, check_id);
