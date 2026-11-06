ALTER TABLE b3_jobs ADD COLUMN phase TEXT NOT NULL DEFAULT 'queued' CHECK(phase IN ('queued','acquisition','inventory','parsing','staging','sealing','ready','publication','identity','completed'));
ALTER TABLE b3_jobs ADD COLUMN error_message TEXT;
CREATE INDEX idx_b3_source_files_job ON b3_source_files(job_id);
CREATE INDEX idx_history_publications_job ON b3_history_publications(job_id);
CREATE INDEX idx_registry_publications_job ON b3_registry_publications(job_id);
-- Existing jobs receive an operational run without fabricating historical market counters.
INSERT INTO b3_sync_runs(id,job_type,status,started_at,finished_at,created_at,updated_at)
SELECT id::text,command->>'operation',status,heartbeat_at,finished_at,created_at,NOW() FROM b3_jobs ON CONFLICT DO NOTHING;
