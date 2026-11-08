CREATE TABLE b3_quote_export_artifacts (
    export_key TEXT PRIMARY KEY CHECK (length(export_key) = 64),
    revision_id UUID NOT NULL REFERENCES b3_history_revisions(id),
    ticker_symbol TEXT NOT NULL,
    schema_version SMALLINT NOT NULL CHECK (schema_version > 0),
    status TEXT NOT NULL CHECK (status IN ('building', 'ready', 'failed')),
    lease_token UUID,
    lease_expires_at TIMESTAMPTZ,
    relative_path TEXT NOT NULL,
    artifact_sha256 TEXT,
    byte_size BIGINT CHECK (byte_size IS NULL OR byte_size >= 0),
    gzip_sha256 TEXT,
    gzip_byte_size BIGINT CHECK (gzip_byte_size IS NULL OR gzip_byte_size >= 0),
    record_count BIGINT CHECK (record_count IS NULL OR record_count >= 0),
    failure_code TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    CHECK (
        (status = 'building' AND lease_token IS NOT NULL AND lease_expires_at IS NOT NULL)
        OR (status <> 'building' AND lease_token IS NULL AND lease_expires_at IS NULL)
    ),
    CHECK (
        status <> 'ready'
        OR (artifact_sha256 IS NOT NULL AND byte_size IS NOT NULL AND record_count IS NOT NULL)
    )
);

CREATE INDEX idx_quote_export_artifacts_expiry
    ON b3_quote_export_artifacts (expires_at)
    WHERE status <> 'building';

CREATE INDEX idx_quote_export_artifacts_revision
    ON b3_quote_export_artifacts (revision_id, ticker_symbol, status);

CREATE INDEX idx_history_quotes_resolved_instrument
    ON b3_history_revision_quotes (revision_id, instrument_id)
    WHERE instrument_id IS NOT NULL;
