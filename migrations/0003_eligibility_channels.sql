-- Manual imports and automated delivery channels cannot share an implicit grant.
ALTER TABLE b3_source_eligibility ADD COLUMN channel TEXT NOT NULL DEFAULT 'unspecified';
ALTER TABLE b3_source_eligibility DROP CONSTRAINT b3_source_eligibility_pkey;
ALTER TABLE b3_source_eligibility ADD PRIMARY KEY (source_file_type, channel, purpose);
ALTER TABLE b3_source_eligibility ADD CONSTRAINT eligibility_channel_nonempty CHECK (length(trim(channel)) > 0);
