ALTER TABLE b3_jobs ADD COLUMN retry_of UUID REFERENCES b3_jobs(id);
CREATE INDEX idx_b3_jobs_retry_of ON b3_jobs(retry_of) WHERE retry_of IS NOT NULL;
