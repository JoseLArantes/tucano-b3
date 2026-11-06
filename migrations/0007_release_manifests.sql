ALTER TABLE b3_source_files ADD COLUMN period_start DATE;
ALTER TABLE b3_source_files ADD COLUMN period_end DATE;
ALTER TABLE b3_source_files ADD COLUMN publication_key TEXT;
ALTER TABLE b3_source_files ADD COLUMN official_revision BIGINT;
ALTER TABLE b3_source_files ADD COLUMN replaces_publication BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE b3_source_files ADD COLUMN format_profile TEXT NOT NULL DEFAULT 'unreviewed';
ALTER TABLE b3_source_channels ADD COLUMN revalidation_seconds INT NOT NULL DEFAULT 86400 CHECK(revalidation_seconds BETWEEN 60 AND 31536000);
CREATE TABLE b3_channel_baselines (
 channel_id TEXT PRIMARY KEY REFERENCES b3_source_channels(id),
 source_file_id VARCHAR(36) NOT NULL REFERENCES b3_source_files(id),
 etag TEXT, last_modified TEXT, content_length BIGINT,
 acknowledged_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), reason TEXT NOT NULL CHECK(length(trim(reason))>0)
);
ALTER TABLE b3_pending_updates ADD COLUMN resolution_reason TEXT;
ALTER TABLE b3_pending_updates ADD COLUMN resolved_source_file_id VARCHAR(36) REFERENCES b3_source_files(id);
ALTER TABLE b3_pending_updates ADD COLUMN handled_at TIMESTAMPTZ;
