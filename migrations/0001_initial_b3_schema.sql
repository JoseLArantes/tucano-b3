-- Phase 1 Initial B3 Schema with PostgreSQL 15+ UNIQUE NULLS NOT DISTINCT

CREATE TABLE IF NOT EXISTS b3_pending_updates (
    id VARCHAR(36) PRIMARY KEY,
    source_file_type VARCHAR(64) NOT NULL,
    reference_date DATE,
    source_url VARCHAR(1024),
    remote_etag VARCHAR(255),
    remote_last_modified VARCHAR(255),
    remote_content_length BIGINT,
    detected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    sync_run_id VARCHAR(36),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS b3_source_files (
    id VARCHAR(36) PRIMARY KEY,
    source_file_type VARCHAR(64) NOT NULL,
    source_name VARCHAR(255),
    reference_date DATE,
    source_url VARCHAR(1024),
    original_file_name VARCHAR(255),
    local_path VARCHAR(1024),
    sha256 VARCHAR(64) NOT NULL,
    byte_size BIGINT,
    downloaded_at TIMESTAMPTZ,
    parsed_at TIMESTAMPTZ,
    ingested_at TIMESTAMPTZ,
    parser_version VARCHAR(32),
    status VARCHAR(32) NOT NULL DEFAULT 'downloaded',
    member_count INT DEFAULT 1,
    row_count_raw INT,
    row_count_inserted INT,
    row_count_changed INT,
    row_count_unchanged INT,
    row_count_quarantined INT DEFAULT 0,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS b3_source_file_members (
    id VARCHAR(36) PRIMARY KEY,
    source_file_id VARCHAR(36) NOT NULL REFERENCES b3_source_files(id) ON DELETE CASCADE,
    member_name VARCHAR(255) NOT NULL,
    member_index INT NOT NULL DEFAULT 0,
    sha256 VARCHAR(64) NOT NULL,
    byte_size BIGINT,
    status VARCHAR(32) NOT NULL DEFAULT 'processed',
    row_count_raw INT DEFAULT 0,
    row_count_inserted INT DEFAULT 0,
    row_count_changed INT DEFAULT 0,
    row_count_unchanged INT DEFAULT 0,
    row_count_quarantined INT DEFAULT 0,
    processed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS b3_sync_runs (
    id VARCHAR(36) PRIMARY KEY,
    job_type VARCHAR(64) NOT NULL,
    requested_by VARCHAR(128),
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    source_file_type VARCHAR(64),
    reference_date_from DATE,
    reference_date_to DATE,
    source_file_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
    stats_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS b3_quarantine_records (
    id VARCHAR(36) PRIMARY KEY,
    sync_run_id VARCHAR(36),
    source_file_id VARCHAR(36) REFERENCES b3_source_files(id) ON DELETE SET NULL,
    source_member_name VARCHAR(255),
    source_file_type VARCHAR(64) NOT NULL,
    reference_date DATE,
    source_line_number INT,
    reason_code VARCHAR(64) NOT NULL,
    error_message TEXT NOT NULL,
    raw_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    resolution_status VARCHAR(32) NOT NULL DEFAULT 'quarantined',
    reprocessed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS b3_field_changes (
    id VARCHAR(36) PRIMARY KEY,
    sync_run_id VARCHAR(36),
    source_file_id VARCHAR(36) REFERENCES b3_source_files(id) ON DELETE CASCADE,
    table_name VARCHAR(128) NOT NULL,
    record_pk VARCHAR(64) NOT NULL,
    field_name VARCHAR(128) NOT NULL,
    old_value_text TEXT,
    new_value_text TEXT,
    old_value_hash VARCHAR(64),
    new_value_hash VARCHAR(64),
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS b3_trading_calendar (
    calendar_date DATE PRIMARY KEY,
    is_trading_day BOOLEAN NOT NULL DEFAULT TRUE,
    holiday_name VARCHAR(255),
    source_note VARCHAR(255),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Domain: Canonical Instruments
CREATE TABLE IF NOT EXISTS b3_instruments (
    id VARCHAR(36) PRIMARY KEY,
    security_id VARCHAR(64) NOT NULL UNIQUE,
    security_source VARCHAR(64),
    market_identifier_code VARCHAR(16),
    asset VARCHAR(32),
    asset_description VARCHAR(255),
    market_code VARCHAR(16),
    market_name VARCHAR(255),
    segment_code VARCHAR(16),
    segment_name VARCHAR(255),
    instrument_type VARCHAR(64),
    security_category VARCHAR(64),
    cfi_code VARCHAR(32),
    instrument_status VARCHAR(64),
    maturity_date DATE,
    source_file_id VARCHAR(36) NOT NULL REFERENCES b3_source_files(id),
    source_member_name VARCHAR(255),
    source_line_number INT,
    source_row_hash VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    data_changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    field_changed_at JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE TABLE IF NOT EXISTS b3_instrument_identifiers (
    id VARCHAR(36) PRIMARY KEY,
    instrument_id VARCHAR(36) NOT NULL REFERENCES b3_instruments(id) ON DELETE CASCADE,
    identifier_type VARCHAR(32) NOT NULL,
    identifier_value VARCHAR(64) NOT NULL,
    market_code VARCHAR(16),
    valid_from DATE,
    valid_to DATE,
    source_file_id VARCHAR(36) NOT NULL REFERENCES b3_source_files(id),
    source_member_name VARCHAR(255),
    source_line_number INT,
    source_row_hash VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    data_changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    field_changed_at JSONB NOT NULL DEFAULT '{}'::jsonb,
    CONSTRAINT uq_identifier_window UNIQUE NULLS NOT DISTINCT (
        instrument_id, identifier_type, identifier_value, market_code, valid_from, valid_to
    )
);

-- Domain: Tradable Securities
CREATE TABLE IF NOT EXISTS b3_tradable_securities (
    id VARCHAR(36) PRIMARY KEY,
    source_file_id VARCHAR(36) NOT NULL REFERENCES b3_source_files(id),
    instrument_id VARCHAR(36) REFERENCES b3_instruments(id),
    reference_date DATE NOT NULL,
    security_id VARCHAR(64),
    ticker_symbol VARCHAR(32),
    isin VARCHAR(16),
    market_code VARCHAR(16),
    segment_code VARCHAR(16),
    tradable_indicator BOOLEAN,
    trading_start_date DATE,
    trading_end_date DATE,
    resolution_status VARCHAR(32) NOT NULL DEFAULT 'unresolved',
    source_member_name VARCHAR(255),
    source_line_number INT,
    source_row_hash VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    data_changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    field_changed_at JSONB NOT NULL DEFAULT '{}'::jsonb,
    CONSTRAINT uq_tradable_nk UNIQUE NULLS NOT DISTINCT (
        reference_date, security_id, ticker_symbol, market_code
    )
);

CREATE TABLE IF NOT EXISTS b3_tradable_securities_staging (
    id VARCHAR(36) PRIMARY KEY,
    source_file_id VARCHAR(36) NOT NULL REFERENCES b3_source_files(id),
    reference_date DATE NOT NULL,
    security_id VARCHAR(64),
    ticker_symbol VARCHAR(32),
    isin VARCHAR(16),
    market_code VARCHAR(16),
    segment_code VARCHAR(16),
    tradable_indicator BOOLEAN,
    trading_start_date DATE,
    trading_end_date DATE,
    resolution_status VARCHAR(32) NOT NULL DEFAULT 'unresolved',
    raw_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Domain: Price Report Quotes (BVBG.086.01)
CREATE TABLE IF NOT EXISTS b3_price_report_quotes (
    id VARCHAR(36) PRIMARY KEY,
    source_file_id VARCHAR(36) NOT NULL REFERENCES b3_source_files(id),
    instrument_id VARCHAR(36) REFERENCES b3_instruments(id),
    trade_date DATE NOT NULL,
    ticker_symbol VARCHAR(32),
    security_id VARCHAR(64),
    market_identifier_code VARCHAR(16),
    days_to_settlement INT,
    trade_quantity NUMERIC(30, 6),
    financial_volume_brl NUMERIC(30, 6),
    financial_volume_usd NUMERIC(30, 6),
    open_interest NUMERIC(30, 6),
    financial_instrument_quantity NUMERIC(30, 6),
    best_bid_price NUMERIC(24, 10),
    best_ask_price NUMERIC(24, 10),
    first_price NUMERIC(24, 10),
    minimum_price NUMERIC(24, 10),
    average_price NUMERIC(24, 10),
    maximum_price NUMERIC(24, 10),
    last_price NUMERIC(24, 10),
    regular_transactions_quantity INT,
    non_regular_transactions_quantity INT,
    regular_traded_contracts NUMERIC(30, 6),
    non_regular_traded_contracts NUMERIC(30, 6),
    national_regular_volume NUMERIC(30, 6),
    national_non_regular_volume NUMERIC(30, 6),
    international_regular_volume NUMERIC(30, 6),
    international_non_regular_volume NUMERIC(30, 6),
    oscillation_percentage NUMERIC(16, 8),
    maximum_trade_limit NUMERIC(24, 10),
    minimum_trade_limit NUMERIC(24, 10),
    resolution_status VARCHAR(32) NOT NULL DEFAULT 'unresolved',
    source_member_name VARCHAR(255),
    source_line_number INT,
    source_row_hash VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    data_changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    field_changed_at JSONB NOT NULL DEFAULT '{}'::jsonb,
    CONSTRAINT uq_price_report_nk UNIQUE NULLS NOT DISTINCT (
        trade_date, security_id, ticker_symbol, market_identifier_code, days_to_settlement
    )
);

-- Domain: Equities Simplified Price Quotes (BVBG.186.01)
CREATE TABLE IF NOT EXISTS b3_equities_simplified_price_quotes (
    id VARCHAR(36) PRIMARY KEY,
    source_file_id VARCHAR(36) NOT NULL REFERENCES b3_source_files(id),
    instrument_id VARCHAR(36) REFERENCES b3_instruments(id),
    trade_date DATE NOT NULL,
    ticker_symbol VARCHAR(32),
    security_id VARCHAR(64),
    market_identifier_code VARCHAR(16),
    first_price NUMERIC(24, 10),
    minimum_price NUMERIC(24, 10),
    average_price NUMERIC(24, 10),
    maximum_price NUMERIC(24, 10),
    last_price NUMERIC(24, 10),
    financial_volume_brl NUMERIC(30, 6),
    resolution_status VARCHAR(32) NOT NULL DEFAULT 'unresolved',
    source_member_name VARCHAR(255),
    source_line_number INT,
    source_row_hash VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    data_changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    field_changed_at JSONB NOT NULL DEFAULT '{}'::jsonb,
    CONSTRAINT uq_equities_simplified_nk UNIQUE NULLS NOT DISTINCT (
        trade_date, security_id, ticker_symbol, market_identifier_code
    )
);

-- Domain: Indices & Index Reports (BVBG.087.01)
CREATE TABLE IF NOT EXISTS b3_indices (
    id VARCHAR(36) PRIMARY KEY,
    index_code VARCHAR(32) NOT NULL UNIQUE,
    index_name VARCHAR(255),
    source_file_id VARCHAR(36) NOT NULL REFERENCES b3_source_files(id),
    source_member_name VARCHAR(255),
    source_line_number INT,
    source_row_hash VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    data_changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    field_changed_at JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE TABLE IF NOT EXISTS b3_index_report_values (
    id VARCHAR(36) PRIMARY KEY,
    source_file_id VARCHAR(36) NOT NULL REFERENCES b3_source_files(id),
    index_id VARCHAR(36) REFERENCES b3_indices(id),
    trade_date DATE NOT NULL,
    index_code VARCHAR(32) NOT NULL,
    index_name VARCHAR(255),
    value_type VARCHAR(64) NOT NULL,
    value NUMERIC(30, 10),
    currency VARCHAR(8),
    market_identifier_code VARCHAR(16),
    source_member_name VARCHAR(255),
    source_line_number INT,
    source_row_hash VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    data_changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    field_changed_at JSONB NOT NULL DEFAULT '{}'::jsonb,
    CONSTRAINT uq_index_report_nk UNIQUE NULLS NOT DISTINCT (
        trade_date, index_code, value_type
    )
);

-- Domain: Index Portfolio Composition
CREATE TABLE IF NOT EXISTS b3_index_portfolio_compositions (
    id VARCHAR(36) PRIMARY KEY,
    source_file_id VARCHAR(36) NOT NULL REFERENCES b3_source_files(id),
    index_id VARCHAR(36) REFERENCES b3_indices(id),
    instrument_id VARCHAR(36) REFERENCES b3_instruments(id),
    composition_date DATE NOT NULL,
    index_code VARCHAR(32) NOT NULL,
    ticker_symbol VARCHAR(32),
    isin VARCHAR(16),
    asset_name VARCHAR(255),
    theoretical_quantity NUMERIC(30, 10),
    portfolio_weight NUMERIC(16, 10),
    participation_percentage NUMERIC(16, 10),
    resolution_status VARCHAR(32) NOT NULL DEFAULT 'unresolved',
    source_member_name VARCHAR(255),
    source_line_number INT,
    source_row_hash VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    data_changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    field_changed_at JSONB NOT NULL DEFAULT '{}'::jsonb,
    CONSTRAINT uq_index_composition_nk UNIQUE NULLS NOT DISTINCT (
        composition_date, index_code, ticker_symbol, instrument_id
    )
);

-- Domain: COTAHIST Historical Quotes
CREATE TABLE IF NOT EXISTS b3_cotahist_quotes (
    id VARCHAR(36) PRIMARY KEY,
    source_file_id VARCHAR(36) NOT NULL REFERENCES b3_source_files(id),
    instrument_id VARCHAR(36) REFERENCES b3_instruments(id),
    trade_date DATE NOT NULL,
    bdi_code VARCHAR(8),
    ticker_symbol VARCHAR(32),
    market_type VARCHAR(8),
    issuer_short_name VARCHAR(255),
    paper_specification VARCHAR(64),
    term_days VARCHAR(8),
    currency VARCHAR(8),
    open_price NUMERIC(24, 10),
    high_price NUMERIC(24, 10),
    low_price NUMERIC(24, 10),
    average_price NUMERIC(24, 10),
    last_price NUMERIC(24, 10),
    best_bid_price NUMERIC(24, 10),
    best_ask_price NUMERIC(24, 10),
    trades_count INT,
    traded_quantity NUMERIC(30, 6),
    traded_volume NUMERIC(30, 6),
    option_exercise_price NUMERIC(24, 10),
    price_correction_indicator VARCHAR(4),
    maturity_date DATE,
    quotation_factor INT,
    exercise_price_points NUMERIC(24, 10),
    isin VARCHAR(16),
    distribution_number VARCHAR(8),
    resolution_status VARCHAR(32) NOT NULL DEFAULT 'unresolved',
    source_member_name VARCHAR(255),
    source_line_number INT,
    source_row_hash VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    data_changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    field_changed_at JSONB NOT NULL DEFAULT '{}'::jsonb,
    CONSTRAINT uq_cotahist_nk UNIQUE NULLS NOT DISTINCT (
        trade_date, ticker_symbol, market_type, bdi_code, distribution_number
    )
);

