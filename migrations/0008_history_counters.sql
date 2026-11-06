ALTER TABLE b3_source_files ADD COLUMN row_count_duplicate BIGINT NOT NULL DEFAULT 0;
ALTER TABLE b3_source_file_members ADD COLUMN row_count_duplicate BIGINT NOT NULL DEFAULT 0;
ALTER TABLE b3_field_changes ADD COLUMN registry_snapshot_id UUID REFERENCES b3_registry_snapshots(id);
ALTER TABLE b3_history_revisions ADD COLUMN revision_kind TEXT NOT NULL DEFAULT 'ingest' CHECK(revision_kind IN ('ingest','identity'));
