-- Keep acquisition metadata separate from the user-requested business-date window.
ALTER TABLE b3_source_files
    ADD COLUMN requested_from DATE,
    ADD COLUMN requested_to DATE,
    ADD CONSTRAINT chk_source_files_requested_window
        CHECK (requested_from IS NULL OR requested_to IS NULL OR requested_from <= requested_to);

ALTER TABLE b3_history_revision_members
    ADD COLUMN out_of_scope BIGINT NOT NULL DEFAULT 0 CHECK (out_of_scope >= 0);

ALTER TABLE b3_history_revisions
    DROP CONSTRAINT b3_history_revisions_revision_kind_check;
ALTER TABLE b3_history_revisions
    ADD CONSTRAINT b3_history_revisions_revision_kind_check
        CHECK (revision_kind IN ('ingest', 'identity', 'withdrawal'));

-- A single requested import can require several overlapping official publications.
-- One candidate revision owns all selected artifacts and is promoted atomically.
CREATE TABLE b3_history_revision_sources (
    revision_id UUID NOT NULL REFERENCES b3_history_revisions(id),
    source_file_id VARCHAR(36) NOT NULL REFERENCES b3_source_files(id),
    requested_from DATE NOT NULL,
    requested_to DATE NOT NULL CHECK (requested_to >= requested_from),
    PRIMARY KEY (revision_id, source_file_id)
);

INSERT INTO b3_history_revision_sources(revision_id, source_file_id, requested_from, requested_to)
SELECT id, source_file_id, period_start, period_end
FROM b3_history_revisions
WHERE revision_kind = 'ingest'
ON CONFLICT DO NOTHING;

CREATE INDEX idx_history_revision_sources_file ON b3_history_revision_sources(source_file_id);
