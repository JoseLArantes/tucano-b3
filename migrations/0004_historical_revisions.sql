CREATE TABLE b3_history_revisions (
 id UUID PRIMARY KEY, job_id UUID NOT NULL REFERENCES b3_jobs(id), base_revision_id UUID REFERENCES b3_history_revisions(id),
 source_file_id VARCHAR(36) NOT NULL REFERENCES b3_source_files(id), channel TEXT NOT NULL,
 period_start DATE NOT NULL, period_end DATE NOT NULL CHECK (period_end>=period_start),
 publication_key TEXT NOT NULL, official_revision BIGINT, replaces_publication BOOLEAN NOT NULL DEFAULT FALSE,
 status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','ready','published','invalid')),
 expected_members INT NOT NULL CHECK (expected_members>0), rejected BIGINT NOT NULL DEFAULT 0,
 conflicts BIGINT NOT NULL DEFAULT 0, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), published_at TIMESTAMPTZ
);
CREATE TABLE b3_history_active(singleton BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK(singleton), revision_id UUID REFERENCES b3_history_revisions(id));
INSERT INTO b3_history_active(singleton,revision_id) VALUES(TRUE,NULL);
CREATE TABLE b3_cotahist_facts (
 id UUID PRIMARY KEY DEFAULT gen_random_uuid(), trade_date DATE NOT NULL, ticker_symbol TEXT NOT NULL,
 market_type TEXT, bdi_code TEXT, distribution_number TEXT, term_days TEXT,
 UNIQUE NULLS NOT DISTINCT(trade_date,ticker_symbol,market_type,bdi_code,distribution_number,term_days)
);
CREATE TABLE b3_cotahist_versions (
 id UUID PRIMARY KEY DEFAULT gen_random_uuid(), fact_id UUID NOT NULL REFERENCES b3_cotahist_facts(id), business_hash TEXT NOT NULL,
 issuer_short_name TEXT, paper_specification TEXT, currency TEXT,
 open_price NUMERIC(38,10), high_price NUMERIC(38,10), low_price NUMERIC(38,10), average_price NUMERIC(38,10),
 last_price NUMERIC(38,10), best_bid_price NUMERIC(38,10), best_ask_price NUMERIC(38,10),
 trades_count INT, traded_quantity NUMERIC(38,10), traded_volume NUMERIC(38,10),
 option_exercise_price NUMERIC(38,10), price_correction_indicator TEXT, maturity_date DATE,
 quotation_factor INT, exercise_price_points NUMERIC(38,10), isin TEXT,
 created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), data_changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
 field_changed_at JSONB NOT NULL DEFAULT '{}'::jsonb,
 UNIQUE(fact_id,business_hash), UNIQUE(id,fact_id)
);
CREATE TABLE b3_history_revision_members (
 revision_id UUID NOT NULL REFERENCES b3_history_revisions(id), source_member_id VARCHAR(36) NOT NULL REFERENCES b3_source_file_members(id),
 status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','complete','invalid')),
 detail_read BIGINT NOT NULL DEFAULT 0, rejected BIGINT NOT NULL DEFAULT 0,
 PRIMARY KEY(revision_id,source_member_id)
);
CREATE TABLE b3_history_revision_quotes (
 revision_id UUID NOT NULL REFERENCES b3_history_revisions(id), fact_id UUID NOT NULL REFERENCES b3_cotahist_facts(id),
 version_id UUID NOT NULL, source_member_id VARCHAR(36) NOT NULL REFERENCES b3_source_file_members(id), record_index BIGINT NOT NULL,
 instrument_id VARCHAR(36) REFERENCES b3_instruments(id), resolution_status TEXT NOT NULL DEFAULT 'unresolved' CHECK(resolution_status IN ('resolved','unresolved','ambiguous')),
 selected_publication_key TEXT NOT NULL, selected_official_revision BIGINT,
 PRIMARY KEY(revision_id,fact_id), FOREIGN KEY(version_id,fact_id) REFERENCES b3_cotahist_versions(id,fact_id),
 CHECK((resolution_status='resolved')=(instrument_id IS NOT NULL))
);
CREATE TABLE b3_cotahist_occurrences (
 revision_id UUID NOT NULL REFERENCES b3_history_revisions(id), source_member_id VARCHAR(36) NOT NULL REFERENCES b3_source_file_members(id),
 record_index BIGINT NOT NULL CHECK(record_index>0), version_id UUID NOT NULL REFERENCES b3_cotahist_versions(id),
 PRIMARY KEY(revision_id,source_member_id,record_index)
);
CREATE TABLE b3_history_conflicts (
 revision_id UUID NOT NULL REFERENCES b3_history_revisions(id), fact_id UUID NOT NULL REFERENCES b3_cotahist_facts(id),
 existing_version_id UUID NOT NULL REFERENCES b3_cotahist_versions(id), incoming_version_id UUID NOT NULL REFERENCES b3_cotahist_versions(id),
 source_member_id VARCHAR(36) NOT NULL REFERENCES b3_source_file_members(id), record_index BIGINT NOT NULL,
 PRIMARY KEY(revision_id,fact_id,incoming_version_id)
);
CREATE TABLE b3_history_rejections (
 revision_id UUID NOT NULL REFERENCES b3_history_revisions(id), source_member_id VARCHAR(36) NOT NULL REFERENCES b3_source_file_members(id),
 record_index BIGINT NOT NULL CHECK(record_index>0), byte_offset BIGINT, reason_code TEXT NOT NULL, message TEXT NOT NULL, raw_payload TEXT NOT NULL,
 PRIMARY KEY(revision_id,source_member_id,record_index)
);
CREATE TABLE b3_history_publications (
 id UUID PRIMARY KEY DEFAULT gen_random_uuid(), revision_id UUID NOT NULL REFERENCES b3_history_revisions(id),
 previous_revision_id UUID REFERENCES b3_history_revisions(id), job_id UUID NOT NULL REFERENCES b3_jobs(id),
 action TEXT NOT NULL CHECK(action IN ('publish','rollback')), created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
ALTER TABLE b3_field_changes ADD COLUMN history_revision_id UUID REFERENCES b3_history_revisions(id);
CREATE INDEX idx_history_quotes_version ON b3_history_revision_quotes(version_id);
CREATE INDEX idx_cotahist_facts_lookup ON b3_cotahist_facts(ticker_symbol,trade_date,id);

ALTER TABLE b3_instruments DROP CONSTRAINT b3_instruments_security_id_key;
ALTER TABLE b3_instruments ADD CONSTRAINT uq_instrument_qualified_id UNIQUE NULLS NOT DISTINCT(security_id,security_source,market_identifier_code);
CREATE TABLE b3_registry_snapshots (
 id UUID PRIMARY KEY, job_id UUID NOT NULL REFERENCES b3_jobs(id), source_file_id VARCHAR(36) NOT NULL REFERENCES b3_source_files(id),
 source_file_type TEXT NOT NULL, channel TEXT NOT NULL, reference_date DATE NOT NULL, expected_members INT NOT NULL CHECK(expected_members>0),
 base_snapshot_id UUID REFERENCES b3_registry_snapshots(id), status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','ready','published','invalid')),
 rejected BIGINT NOT NULL DEFAULT 0, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), published_at TIMESTAMPTZ
);
CREATE TABLE b3_registry_active(source_file_type TEXT PRIMARY KEY,snapshot_id UUID NOT NULL REFERENCES b3_registry_snapshots(id));
CREATE TABLE b3_registry_records (
 snapshot_id UUID NOT NULL REFERENCES b3_registry_snapshots(id), source_member_id VARCHAR(36) NOT NULL REFERENCES b3_source_file_members(id), record_index BIGINT NOT NULL,
 security_id TEXT, security_source TEXT NOT NULL, market_identifier_code TEXT,
 ticker_symbol TEXT, isin TEXT, asset TEXT, asset_description TEXT, market_code TEXT, market_name TEXT,
 segment_code TEXT, segment_name TEXT, instrument_type TEXT, security_category TEXT,cfi_code TEXT,instrument_status TEXT,maturity_date DATE,
 tradable_indicator BOOLEAN,trading_start_date DATE,trading_end_date DATE,business_hash TEXT NOT NULL,
 instrument_id VARCHAR(36) REFERENCES b3_instruments(id), PRIMARY KEY(snapshot_id,source_member_id,record_index)
);
CREATE TABLE b3_registry_members(snapshot_id UUID NOT NULL REFERENCES b3_registry_snapshots(id),source_member_id VARCHAR(36) NOT NULL REFERENCES b3_source_file_members(id),structural_valid BOOLEAN NOT NULL,PRIMARY KEY(snapshot_id,source_member_id));
CREATE TABLE b3_registry_publications(id UUID PRIMARY KEY DEFAULT gen_random_uuid(),snapshot_id UUID NOT NULL REFERENCES b3_registry_snapshots(id),previous_snapshot_id UUID REFERENCES b3_registry_snapshots(id),job_id UUID NOT NULL REFERENCES b3_jobs(id),action TEXT NOT NULL CHECK(action IN ('publish','rollback')),created_at TIMESTAMPTZ NOT NULL DEFAULT NOW());
CREATE TABLE b3_registry_rejections(snapshot_id UUID NOT NULL REFERENCES b3_registry_snapshots(id),source_member_id VARCHAR(36) NOT NULL REFERENCES b3_source_file_members(id),record_index BIGINT NOT NULL,byte_offset BIGINT,reason_code TEXT NOT NULL,message TEXT NOT NULL,raw_payload TEXT NOT NULL,PRIMARY KEY(snapshot_id,source_member_id,record_index));
ALTER TABLE b3_quarantine_records ADD COLUMN history_revision_id UUID REFERENCES b3_history_revisions(id), ADD COLUMN registry_snapshot_id UUID REFERENCES b3_registry_snapshots(id), ADD COLUMN source_record_index BIGINT, ADD COLUMN byte_offset BIGINT;
CREATE UNIQUE INDEX uq_history_quarantine ON b3_quarantine_records(history_revision_id,source_member_name,source_record_index) WHERE history_revision_id IS NOT NULL;
CREATE UNIQUE INDEX uq_registry_quarantine ON b3_quarantine_records(registry_snapshot_id,source_member_name,source_record_index) WHERE registry_snapshot_id IS NOT NULL;
CREATE FUNCTION b3_reject_immutable_fact_change() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'historical facts and business versions are immutable' USING ERRCODE='55000'; END $$;
CREATE TRIGGER immutable_cotahist_fact BEFORE UPDATE OR DELETE ON b3_cotahist_facts FOR EACH ROW EXECUTE FUNCTION b3_reject_immutable_fact_change();
CREATE TRIGGER immutable_cotahist_version BEFORE UPDATE OR DELETE ON b3_cotahist_versions FOR EACH ROW EXECUTE FUNCTION b3_reject_immutable_fact_change();
