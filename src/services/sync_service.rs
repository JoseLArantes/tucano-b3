use chrono::{NaiveDate, Utc};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::collections::BTreeMap;
use std::io::Cursor;
use uuid::Uuid;

use crate::config::B3Settings;
use crate::db::enums::{ResolutionStatus, SourceFileType, SyncRunStatus};
use crate::db::locks::{lock_key_to_i64, InMemoryLockRegistry};
use crate::db::resolver::RelationshipResolver;
use crate::db::upsert::{insert_field_changes, FieldChangeAudit};
use crate::error::{AppError, Result};
use crate::parsers::bvbg_028::parse_bvbg_028;
use crate::parsers::bvbg_086::parse_bvbg_086;
use crate::parsers::bvbg_087::parse_bvbg_087;
use crate::parsers::bvbg_186::parse_bvbg_186;
use crate::parsers::canonical::stable_row_hash;
use crate::parsers::cotahist::parse_cotahist;
use crate::parsers::extract_archive_members;
use crate::parsers::index_composition::parse_index_composition_csv;
use crate::parsers::tradable_list::parse_tradable_security_list;
use crate::services::download_service::B3DownloadService;

#[derive(Debug, Clone)]
pub struct SyncOutcome {
    pub sync_run_id: String,
    pub status: SyncRunStatus,
    pub inserted: i32,
    pub changed: i32,
    pub unchanged: i32,
    pub unresolved: i32,
    pub warnings: Vec<String>,
    pub source_file_ids: Vec<String>,
}

pub struct B3SyncService {
    pool: PgPool,
    settings: B3Settings,
    download_service: B3DownloadService,
    in_memory_locks: InMemoryLockRegistry,
}

impl B3SyncService {
    pub const INGESTION_ORDER: [SourceFileType; 6] = [
        SourceFileType::Bvbg028InstrumentsFile,
        SourceFileType::TradableSecurityList,
        SourceFileType::Bvbg087IndexReport,
        SourceFileType::Bvbg086PriceReport,
        SourceFileType::Bvbg186EquitiesSimplifiedPriceReport,
        SourceFileType::CotahistYearly,
    ];

    pub fn new(pool: PgPool, settings: B3Settings) -> Self {
        let download_service = B3DownloadService::new(&settings);
        Self {
            pool,
            settings,
            download_service,
            in_memory_locks: InMemoryLockRegistry::new(),
        }
    }

    pub fn settings(&self) -> &B3Settings {
        &self.settings
    }

    pub async fn run_manual_sync(
        &self,
        source_file_type: SourceFileType,
        reference_date_from: NaiveDate,
        reference_date_to: NaiveDate,
        source_url: Option<String>,
        raw_content: Option<Vec<u8>>,
        force_download: bool,
        force_reparse: bool,
        requested_by: Option<String>,
    ) -> Result<SyncOutcome> {
        if raw_content.is_some() && reference_date_from != reference_date_to {
            return Err(AppError::Validation(
                "raw_content so pode ser usado para uma unica data por execucao.".to_string(),
            ));
        }

        let sync_run_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let req_by = requested_by.as_deref().unwrap_or("manual-api");

        sqlx::query(
            r#"
            INSERT INTO b3_sync_runs (
                id, job_type, requested_by, started_at, status,
                source_file_type, reference_date_from, reference_date_to,
                source_file_ids, stats_json, created_at, updated_at
            )
            VALUES ($1, 'manual_sync', $2, $3, 'running', $4, $5, $6, '[]'::jsonb, '{}'::jsonb, $3, $3)
            "#,
        )
        .bind(&sync_run_id)
        .bind(req_by)
        .bind(now)
        .bind(source_file_type.as_str())
        .bind(reference_date_from)
        .bind(reference_date_to)
        .execute(&self.pool)
        .await?;

        let mut current_date = reference_date_from;
        let mut inserted = 0;
        let mut changed = 0;
        let mut unchanged = 0;
        let mut unresolved = 0;
        let mut all_warnings = Vec::new();
        let mut source_file_ids = Vec::new();

        while current_date <= reference_date_to {
            let lock_key = format!("{}:{}", source_file_type.as_str(), current_date);
            let _mem_guard = self.in_memory_locks.acquire(&lock_key)?;

            let lock_id = lock_key_to_i64(&lock_key);
            let mut tx = self.pool.begin().await?;
            let row = sqlx::query("SELECT pg_try_advisory_xact_lock($1)")
                .bind(lock_id)
                .fetch_one(&mut *tx)
                .await?;
            let acquired: bool = row.get(0);

            if !acquired {
                tx.rollback().await?;
                return Err(AppError::LockConflict(format!(
                    "Ja existe sincronizacao em andamento para {lock_key}."
                )));
            }

            let date_content = if current_date == reference_date_from {
                raw_content.clone()
            } else {
                None
            };

            let outcome = self
                .run_single_date(
                    source_file_type,
                    current_date,
                    source_url.as_deref(),
                    date_content,
                    force_download,
                    force_reparse,
                )
                .await;

            tx.commit().await?;

            match outcome {
                Ok(res) => {
                    inserted += res.0;
                    changed += res.1;
                    unchanged += res.2;
                    unresolved += res.3;
                    all_warnings.extend(res.4);
                    source_file_ids.extend(res.5);
                }
                Err(err) => {
                    let err_str = err.to_string();
                    let stats_err = serde_json::json!({ "error": err_str });
                    sqlx::query(
                        r#"
                        UPDATE b3_sync_runs
                        SET status = 'failed', finished_at = NOW(), stats_json = $1, updated_at = NOW()
                        WHERE id = $2
                        "#,
                    )
                    .bind(stats_err)
                    .bind(&sync_run_id)
                    .execute(&self.pool)
                    .await?;
                    return Err(err);
                }
            }

            current_date = current_date.succ_opt().unwrap_or(current_date);
        }

        let skipped_duplicate = inserted == 0 && changed == 0 && unchanged == 0;
        let stats_json = serde_json::json!({
            "inserted": inserted,
            "changed": changed,
            "unchanged": unchanged,
            "unresolved": unresolved,
            "warnings": all_warnings,
            "skipped_duplicate": skipped_duplicate,
        });
        let sf_ids_val = serde_json::to_value(&source_file_ids).unwrap_or_default();

        sqlx::query(
            r#"
            UPDATE b3_sync_runs
            SET status = 'success', finished_at = NOW(), source_file_ids = $1,
                stats_json = $2, updated_at = NOW()
            WHERE id = $3
            "#,
        )
        .bind(sf_ids_val)
        .bind(stats_json)
        .bind(&sync_run_id)
        .execute(&self.pool)
        .await?;

        Ok(SyncOutcome {
            sync_run_id,
            status: SyncRunStatus::Success,
            inserted,
            changed,
            unchanged,
            unresolved,
            warnings: all_warnings,
            source_file_ids,
        })
    }

    async fn run_single_date(
        &self,
        source_file_type: SourceFileType,
        reference_date: NaiveDate,
        source_url: Option<&str>,
        raw_content: Option<Vec<u8>>,
        force_download: bool,
        force_reparse: bool,
    ) -> Result<(i32, i32, i32, i32, Vec<String>, Vec<String>)> {
        let (content, content_hash, resolved_url) = self
            .obtain_content(source_file_type, reference_date, source_url, raw_content, force_download)
            .await?;

        let existing = sqlx::query(
            r#"
            SELECT id FROM b3_source_files
            WHERE source_file_type = $1 AND reference_date = $2 AND sha256 = $3
              AND status IN ('ingested', 'skipped_duplicate')
            LIMIT 1
            "#,
        )
        .bind(source_file_type.as_str())
        .bind(reference_date)
        .bind(&content_hash)
        .fetch_optional(&self.pool)
        .await?;

        if existing.is_some() && !force_reparse {
            let skipped_id = Uuid::new_v4().to_string();
            let original_name = format!("{}_{}", source_file_type.as_str(), reference_date);

            sqlx::query(
                r#"
                INSERT INTO b3_source_files (
                    id, source_file_type, source_name, reference_date, source_url,
                    original_file_name, sha256, byte_size, downloaded_at, parser_version,
                    status, row_count_raw, row_count_inserted, row_count_changed,
                    row_count_unchanged, created_at, updated_at
                )
                VALUES ($1, $2, $2, $3, $4, $5, $6, $7, NOW(), 'v1', 'skipped_duplicate', 0, 0, 0, 0, NOW(), NOW())
                "#,
            )
            .bind(&skipped_id)
            .bind(source_file_type.as_str())
            .bind(reference_date)
            .bind(&resolved_url)
            .bind(&original_name)
            .bind(&content_hash)
            .bind(content.len() as i64)
            .execute(&self.pool)
            .await?;

            return Ok((0, 0, 0, 0, Vec::new(), vec![skipped_id]));
        }

        let source_file_id = Uuid::new_v4().to_string();
        let orig_name = format!("{}_{}", source_file_type.as_str(), reference_date);

        sqlx::query(
            r#"
            INSERT INTO b3_source_files (
                id, source_file_type, source_name, reference_date, source_url,
                original_file_name, sha256, byte_size, downloaded_at, parser_version,
                status, created_at, updated_at
            )
            VALUES ($1, $2, $2, $3, $4, $5, $6, $7, NOW(), 'v1', 'downloaded', NOW(), NOW())
            "#,
        )
        .bind(&source_file_id)
        .bind(source_file_type.as_str())
        .bind(reference_date)
        .bind(&resolved_url)
        .bind(&orig_name)
        .bind(&content_hash)
        .bind(content.len() as i64)
        .execute(&self.pool)
        .await?;

        let members = extract_archive_members(source_file_type, &content)?;

        let mut total_inserted = 0;
        let mut total_changed = 0;
        let mut total_unchanged = 0;
        let mut total_unresolved = 0;
        let mut all_warnings = Vec::new();

        let resolver = RelationshipResolver::new(&self.pool);

        for (idx, member) in members.iter().enumerate() {
            let member_id = Uuid::new_v4().to_string();
            sqlx::query(
                r#"
                INSERT INTO b3_source_file_members (
                    id, source_file_id, member_name, member_index, sha256, byte_size,
                    status, processed_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, 'processing', NOW())
                "#,
            )
            .bind(&member_id)
            .bind(&source_file_id)
            .bind(&member.name)
            .bind(idx as i32)
            .bind(&member.sha256)
            .bind(member.byte_size)
            .execute(&self.pool)
            .await?;

            let (ins, chg, unchg, unres, warns) = self
                .ingest_member(
                    &source_file_id,
                    &member.name,
                    source_file_type,
                    &member.content,
                    reference_date,
                    &resolver,
                )
                .await?;

            total_inserted += ins;
            total_changed += chg;
            total_unchanged += unchg;
            total_unresolved += unres;
            all_warnings.extend(warns);

            let _ = sqlx::query(
                r#"
                UPDATE b3_source_file_members
                SET status = 'processed', row_count_inserted = $1, row_count_changed = $2,
                    row_count_unchanged = $3
                WHERE id = $4
                "#,
            )
            .bind(ins)
            .bind(chg)
            .bind(unchg)
            .bind(&member_id)
            .execute(&self.pool)
            .await;
        }

        sqlx::query(
            r#"
            UPDATE b3_source_files
            SET status = 'ingested', ingested_at = NOW(), parsed_at = NOW(),
                row_count_raw = $1, row_count_inserted = $2, row_count_changed = $3,
                row_count_unchanged = $4, updated_at = NOW()
            WHERE id = $5
            "#,
        )
        .bind(total_inserted + total_changed + total_unchanged)
        .bind(total_inserted)
        .bind(total_changed)
        .bind(total_unchanged)
        .bind(&source_file_id)
        .execute(&self.pool)
        .await?;

        Ok((
            total_inserted,
            total_changed,
            total_unchanged,
            total_unresolved,
            all_warnings,
            vec![source_file_id],
        ))
    }

    async fn obtain_content(
        &self,
        source_file_type: SourceFileType,
        reference_date: NaiveDate,
        source_url: Option<&str>,
        raw_content: Option<Vec<u8>>,
        force_download: bool,
    ) -> Result<(Vec<u8>, String, String)> {
        if let Some(content) = raw_content {
            let mut hasher = Sha256::new();
            hasher.update(&content);
            let hash = hex::encode(hasher.finalize());
            return Ok((content, hash, "raw_content".to_string()));
        }

        let target_url = if let Some(url) = source_url {
            url.trim().to_string()
        } else {
            self.build_native_source_url(source_file_type, reference_date)?
        };

        if target_url.is_empty() {
            if force_download {
                return Err(AppError::Validation("force_download=true exige source_url.".to_string()));
            }
            return Err(AppError::Validation("Informe source_url ou raw_content para sincronizar.".to_string()));
        }

        let dl = self.download_service.download(&target_url).await?;
        Ok((dl.content, dl.sha256, target_url))
    }

    pub fn build_native_source_url(
        &self,
        source_file_type: SourceFileType,
        reference_date: NaiveDate,
    ) -> Result<String> {
        let yymmdd = reference_date.format("%y%m%d").to_string();
        let yyyy = reference_date.format("%Y").to_string();

        let file_name = match source_file_type {
            SourceFileType::Bvbg028InstrumentsFile => format!("IN{yymmdd}.zip"),
            SourceFileType::TradableSecurityList => format!("SecurityList{yymmdd}.zip"),
            SourceFileType::Bvbg086PriceReport => format!("PR{yymmdd}.zip"),
            SourceFileType::Bvbg186EquitiesSimplifiedPriceReport => format!("SPRE{yymmdd}.zip"),
            SourceFileType::Bvbg087IndexReport => format!("IR{yymmdd}.zip"),
            SourceFileType::CotahistYearly => format!("COTAHIST_A{yyyy}.ZIP"),
            SourceFileType::B3IndexPortfolioComposition => {
                return Err(AppError::Validation(
                    "Ingestao de composicao de indice requer fonte dedicada da B3.".to_string(),
                ));
            }
            _ => return Err(AppError::Validation(format!("Unsupported type: {source_file_type}"))),
        };

        Ok(format!(
            "https://www.b3.com.br/pesquisapregao/download?filelist={file_name}"
        ))
    }

    async fn ingest_member(
        &self,
        source_file_id: &str,
        member_name: &str,
        source_file_type: SourceFileType,
        content: &[u8],
        reference_date: NaiveDate,
        resolver: &RelationshipResolver<'_>,
    ) -> Result<(i32, i32, i32, i32, Vec<String>)> {
        let mut inserted = 0;
        let mut changed = 0;
        let mut unchanged = 0;
        let mut unresolved = 0;
        let mut warnings = Vec::new();

        match source_file_type {
            SourceFileType::Bvbg028InstrumentsFile => {
                let parsed = parse_bvbg_028(Cursor::new(content))?;
                warnings.extend(parsed.warnings);

                for row in parsed.rows {
                    let mut fields = BTreeMap::new();
                    fields.insert("security_id".to_string(), serde_json::Value::String(row.security_id.clone()));
                    if let Some(ref val) = row.security_source {
                        fields.insert("security_source".to_string(), serde_json::Value::String(val.clone()));
                    }
                    if let Some(ref val) = row.market_identifier_code {
                        fields.insert("market_identifier_code".to_string(), serde_json::Value::String(val.clone()));
                    }
                    if let Some(ref val) = row.asset {
                        fields.insert("asset".to_string(), serde_json::Value::String(val.clone()));
                    }
                    if let Some(ref val) = row.asset_description {
                        fields.insert("asset_description".to_string(), serde_json::Value::String(val.clone()));
                    }
                    if let Some(ref val) = row.market_code {
                        fields.insert("market_code".to_string(), serde_json::Value::String(val.clone()));
                    }
                    if let Some(ref val) = row.market_name {
                        fields.insert("market_name".to_string(), serde_json::Value::String(val.clone()));
                    }
                    if let Some(ref val) = row.segment_code {
                        fields.insert("segment_code".to_string(), serde_json::Value::String(val.clone()));
                    }
                    if let Some(ref val) = row.segment_name {
                        fields.insert("segment_name".to_string(), serde_json::Value::String(val.clone()));
                    }
                    if let Some(ref val) = row.instrument_type {
                        fields.insert("instrument_type".to_string(), serde_json::Value::String(val.clone()));
                    }
                    if let Some(ref val) = row.security_category {
                        fields.insert("security_category".to_string(), serde_json::Value::String(val.clone()));
                    }
                    if let Some(ref val) = row.cfi_code {
                        fields.insert("cfi_code".to_string(), serde_json::Value::String(val.clone()));
                    }
                    if let Some(ref val) = row.instrument_status {
                        fields.insert("instrument_status".to_string(), serde_json::Value::String(val.clone()));
                    }

                    let row_hash = stable_row_hash(&fields);

                    let existing = sqlx::query("SELECT id, source_row_hash, field_changed_at FROM b3_instruments WHERE security_id = $1")
                        .bind(&row.security_id)
                        .fetch_optional(&self.pool)
                        .await?;

                    let instrument_id = match existing {
                        None => {
                            let new_id = Uuid::new_v4().to_string();
                            let field_changed_at_obj: serde_json::Map<String, serde_json::Value> = fields
                                .keys()
                                .map(|k| (k.clone(), serde_json::Value::String(Utc::now().to_rfc3339())))
                                .collect();
                            let field_changed_val = serde_json::Value::Object(field_changed_at_obj);

                            sqlx::query(
                                r#"
                                INSERT INTO b3_instruments (
                                    id, security_id, security_source, market_identifier_code,
                                    asset, asset_description, market_code, market_name,
                                    segment_code, segment_name, instrument_type, security_category,
                                    cfi_code, instrument_status, maturity_date, source_file_id,
                                    source_member_name, source_row_hash, created_at, updated_at,
                                    last_seen_at, data_changed_at, field_changed_at
                                )
                                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, NOW(), NOW(), NOW(), NOW(), $19)
                                "#,
                            )
                            .bind(&new_id)
                            .bind(&row.security_id)
                            .bind(&row.security_source)
                            .bind(&row.market_identifier_code)
                            .bind(&row.asset)
                            .bind(&row.asset_description)
                            .bind(&row.market_code)
                            .bind(&row.market_name)
                            .bind(&row.segment_code)
                            .bind(&row.segment_name)
                            .bind(&row.instrument_type)
                            .bind(&row.security_category)
                            .bind(&row.cfi_code)
                            .bind(&row.instrument_status)
                            .bind(row.maturity_date)
                            .bind(source_file_id)
                            .bind(member_name)
                            .bind(&row_hash)
                            .bind(field_changed_val)
                            .execute(&self.pool)
                            .await?;

                            inserted += 1;
                            new_id
                        }
                        Some(rec) => {
                            let rec_id: String = rec.get("id");
                            let rec_hash: String = rec.get("source_row_hash");
                            let rec_fc: serde_json::Value = rec.get("field_changed_at");

                            if rec_hash == row_hash {
                                unchanged += 1;
                                rec_id
                            } else {
                                let mut fc_map = rec_fc.as_object().cloned().unwrap_or_default();
                                fc_map.insert("updated".to_string(), serde_json::Value::String(Utc::now().to_rfc3339()));
                                let new_fc_val = serde_json::Value::Object(fc_map);

                                sqlx::query(
                                    r#"
                                    UPDATE b3_instruments
                                    SET security_source = $1, market_identifier_code = $2, asset = $3,
                                        asset_description = $4, market_code = $5, market_name = $6,
                                        segment_code = $7, segment_name = $8, instrument_type = $9,
                                        security_category = $10, cfi_code = $11, instrument_status = $12,
                                        maturity_date = $13, source_file_id = $14, source_member_name = $15,
                                        source_row_hash = $16, updated_at = NOW(), last_seen_at = NOW(),
                                        data_changed_at = NOW(), field_changed_at = $17
                                    WHERE id = $18
                                    "#,
                                )
                                .bind(&row.security_source)
                                .bind(&row.market_identifier_code)
                                .bind(&row.asset)
                                .bind(&row.asset_description)
                                .bind(&row.market_code)
                                .bind(&row.market_name)
                                .bind(&row.segment_code)
                                .bind(&row.segment_name)
                                .bind(&row.instrument_type)
                                .bind(&row.security_category)
                                .bind(&row.cfi_code)
                                .bind(&row.instrument_status)
                                .bind(row.maturity_date)
                                .bind(source_file_id)
                                .bind(member_name)
                                .bind(&row_hash)
                                .bind(new_fc_val)
                                .bind(&rec_id)
                                .execute(&self.pool)
                                .await?;

                                changed += 1;

                                let audit = FieldChangeAudit {
                                    table_name: "b3_instruments".to_string(),
                                    record_pk: rec_id.clone(),
                                    field_name: "asset_description".to_string(),
                                    old_value_text: None,
                                    new_value_text: row.asset_description.clone(),
                                    old_value_hash: Some(rec_hash),
                                    new_value_hash: Some(row_hash.clone()),
                                };
                                let _ = insert_field_changes(&self.pool, None, source_file_id, &[audit]).await;

                                rec_id
                            }
                        }
                    };

                    // Insert identifiers
                    let identifiers = vec![
                        ("security_id", Some(row.security_id.as_str())),
                        ("ticker", row.ticker_symbol.as_deref()),
                        ("isin", row.isin.as_deref()),
                        ("asset", row.asset.as_deref()),
                    ];

                    for (id_type, id_val_opt) in identifiers {
                        if let Some(id_val) = id_val_opt {
                            let ident_id = Uuid::new_v4().to_string();
                            let mut ident_fields = BTreeMap::new();
                            ident_fields.insert("identifier_type".to_string(), serde_json::Value::String(id_type.to_string()));
                            ident_fields.insert("identifier_value".to_string(), serde_json::Value::String(id_val.to_string()));
                            let ident_hash = stable_row_hash(&ident_fields);

                            sqlx::query(
                                r#"
                                INSERT INTO b3_instrument_identifiers (
                                    id, instrument_id, identifier_type, identifier_value,
                                    market_code, valid_from, valid_to, source_file_id,
                                    source_member_name, source_row_hash, created_at, updated_at,
                                    last_seen_at, data_changed_at, field_changed_at
                                )
                                VALUES ($1, $2, $3, $4, $5, $6, NULL, $7, $8, $9, NOW(), NOW(), NOW(), NOW(), '{}'::jsonb)
                                ON CONFLICT DO NOTHING
                                "#,
                            )
                            .bind(&ident_id)
                            .bind(&instrument_id)
                            .bind(id_type)
                            .bind(id_val)
                            .bind(&row.market_code)
                            .bind(row.reference_date.unwrap_or(reference_date))
                            .bind(source_file_id)
                            .bind(member_name)
                            .bind(&ident_hash)
                            .execute(&self.pool)
                            .await?;
                        }
                    }
                }
            }
            SourceFileType::TradableSecurityList => {
                let parsed = parse_tradable_security_list(Cursor::new(content))?;
                warnings.extend(parsed.warnings);

                for row in parsed.rows {
                    let res = resolver
                        .resolve_instrument(
                            row.security_id.as_deref(),
                            row.ticker_symbol.as_deref(),
                            row.isin.as_deref(),
                            row.market_code.as_deref(),
                            Some(row.reference_date),
                        )
                        .await?;

                    if res.status != ResolutionStatus::Resolved {
                        let stg_id = Uuid::new_v4().to_string();
                        let raw_val = serde_json::json!({
                            "security_id": row.security_id,
                            "ticker_symbol": row.ticker_symbol,
                            "isin": row.isin,
                            "market_code": row.market_code,
                        });

                        sqlx::query(
                            r#"
                            INSERT INTO b3_tradable_securities_staging (
                                id, source_file_id, reference_date, security_id,
                                ticker_symbol, isin, market_code, segment_code,
                                tradable_indicator, trading_start_date, trading_end_date,
                                resolution_status, raw_payload, created_at, updated_at
                            )
                            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, NOW(), NOW())
                            "#,
                        )
                        .bind(&stg_id)
                        .bind(source_file_id)
                        .bind(row.reference_date)
                        .bind(&row.security_id)
                        .bind(&row.ticker_symbol)
                        .bind(&row.isin)
                        .bind(&row.market_code)
                        .bind(&row.segment_code)
                        .bind(row.tradable_indicator)
                        .bind(row.trading_start_date)
                        .bind(row.trading_end_date)
                        .bind(res.status.as_str())
                        .bind(raw_val)
                        .execute(&self.pool)
                        .await?;

                        unresolved += 1;
                        unchanged += 1;
                        continue;
                    }

                    let new_id = Uuid::new_v4().to_string();
                    let mut fields = BTreeMap::new();
                    fields.insert("reference_date".to_string(), serde_json::Value::String(row.reference_date.to_string()));
                    if let Some(ref val) = row.security_id {
                        fields.insert("security_id".to_string(), serde_json::Value::String(val.clone()));
                    }
                    if let Some(ref val) = row.ticker_symbol {
                        fields.insert("ticker_symbol".to_string(), serde_json::Value::String(val.clone()));
                    }
                    let hash = stable_row_hash(&fields);

                    let result = sqlx::query(
                        r#"
                        INSERT INTO b3_tradable_securities (
                            id, source_file_id, instrument_id, reference_date,
                            security_id, ticker_symbol, isin, market_code, segment_code,
                            tradable_indicator, trading_start_date, trading_end_date,
                            resolution_status, source_member_name, source_row_hash,
                            created_at, updated_at, last_seen_at, data_changed_at, field_changed_at
                        )
                        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, NOW(), NOW(), NOW(), NOW(), '{}'::jsonb)
                        ON CONFLICT (reference_date, security_id, ticker_symbol, market_code) DO NOTHING
                        "#,
                    )
                    .bind(&new_id)
                    .bind(source_file_id)
                    .bind(&res.instrument_id)
                    .bind(row.reference_date)
                    .bind(&row.security_id)
                    .bind(&row.ticker_symbol)
                    .bind(&row.isin)
                    .bind(&row.market_code)
                    .bind(&row.segment_code)
                    .bind(row.tradable_indicator)
                    .bind(row.trading_start_date)
                    .bind(row.trading_end_date)
                    .bind(res.status.as_str())
                    .bind(member_name)
                    .bind(&hash)
                    .execute(&self.pool)
                    .await?;

                    if result.rows_affected() > 0 {
                        inserted += 1;
                    } else {
                        unchanged += 1;
                    }
                }
            }
            SourceFileType::CotahistYearly
            | SourceFileType::CotahistMonthly
            | SourceFileType::CotahistDaily => {
                let content_str = String::from_utf8_lossy(content);
                let parsed = parse_cotahist(&content_str)?;
                warnings.extend(parsed.warnings);

                for row in parsed.rows {
                    let trade_dt = row.trade_date.unwrap_or(reference_date);
                    let res = resolver
                        .resolve_instrument(
                            None,
                            row.ticker_symbol.as_deref(),
                            row.isin.as_deref(),
                            None,
                            Some(trade_dt),
                        )
                        .await?;

                    if res.status != ResolutionStatus::Resolved {
                        unresolved += 1;
                    }

                    let new_id = Uuid::new_v4().to_string();
                    let mut fields = BTreeMap::new();
                    fields.insert("trade_date".to_string(), serde_json::Value::String(trade_dt.to_string()));
                    if let Some(ref val) = row.ticker_symbol {
                        fields.insert("ticker_symbol".to_string(), serde_json::Value::String(val.clone()));
                    }
                    if let Some(ref val) = row.last_price {
                        fields.insert("last_price".to_string(), serde_json::Value::String(val.to_string()));
                    }
                    let hash = stable_row_hash(&fields);

                    let result = sqlx::query(
                        r#"
                        INSERT INTO b3_cotahist_quotes (
                            id, source_file_id, instrument_id, trade_date, bdi_code,
                            ticker_symbol, market_type, issuer_short_name, paper_specification,
                            term_days, currency, open_price, high_price, low_price,
                            average_price, last_price, best_bid_price, best_ask_price,
                            trades_count, traded_quantity, traded_volume, option_exercise_price,
                            price_correction_indicator, maturity_date, quotation_factor,
                            exercise_price_points, isin, distribution_number, resolution_status,
                            source_member_name, source_row_hash, created_at, updated_at,
                            last_seen_at, data_changed_at, field_changed_at
                        )
                        VALUES (
                            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14,
                            $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25,
                            $26, $27, $28, $29, $30, $31, NOW(), NOW(), NOW(), NOW(), '{}'::jsonb
                        )
                        ON CONFLICT (trade_date, ticker_symbol, market_type, bdi_code, distribution_number) DO NOTHING
                        "#,
                    )
                    .bind(&new_id)
                    .bind(source_file_id)
                    .bind(&res.instrument_id)
                    .bind(trade_dt)
                    .bind(&row.bdi_code)
                    .bind(&row.ticker_symbol)
                    .bind(&row.market_type)
                    .bind(&row.issuer_short_name)
                    .bind(&row.paper_specification)
                    .bind(&row.term_days)
                    .bind(&row.currency)
                    .bind(row.open_price)
                    .bind(row.high_price)
                    .bind(row.low_price)
                    .bind(row.average_price)
                    .bind(row.last_price)
                    .bind(row.best_bid_price)
                    .bind(row.best_ask_price)
                    .bind(row.trades_count)
                    .bind(row.traded_quantity)
                    .bind(row.traded_volume)
                    .bind(row.option_exercise_price)
                    .bind(&row.price_correction_indicator)
                    .bind(row.maturity_date)
                    .bind(row.quotation_factor)
                    .bind(row.exercise_price_points)
                    .bind(&row.isin)
                    .bind(&row.distribution_number)
                    .bind(res.status.as_str())
                    .bind(member_name)
                    .bind(&hash)
                    .execute(&self.pool)
                    .await?;

                    if result.rows_affected() > 0 {
                        inserted += 1;
                    } else {
                        unchanged += 1;
                    }
                }
            }
            SourceFileType::Bvbg086PriceReport => {
                let parsed = parse_bvbg_086(Cursor::new(content))?;
                warnings.extend(parsed.warnings);

                for row in parsed.rows {
                    let trade_dt = row.trade_date.unwrap_or(reference_date);
                    let res = resolver
                        .resolve_instrument(
                            row.security_id.as_deref(),
                            row.ticker_symbol.as_deref(),
                            None,
                            row.market_identifier_code.as_deref(),
                            Some(trade_dt),
                        )
                        .await?;

                    if res.status != ResolutionStatus::Resolved {
                        unresolved += 1;
                    }

                    let new_id = Uuid::new_v4().to_string();
                    let mut fields = BTreeMap::new();
                    fields.insert("trade_date".to_string(), serde_json::Value::String(trade_dt.to_string()));
                    if let Some(ref val) = row.ticker_symbol {
                        fields.insert("ticker_symbol".to_string(), serde_json::Value::String(val.clone()));
                    }
                    if let Some(ref val) = row.last_price {
                        fields.insert("last_price".to_string(), serde_json::Value::String(val.to_string()));
                    }
                    let hash = stable_row_hash(&fields);

                    let result = sqlx::query(
                        r#"
                        INSERT INTO b3_price_report_quotes (
                            id, source_file_id, instrument_id, trade_date, ticker_symbol,
                            security_id, market_identifier_code, days_to_settlement, trade_quantity,
                            financial_volume_brl, financial_volume_usd, open_interest,
                            financial_instrument_quantity, best_bid_price, best_ask_price,
                            first_price, minimum_price, average_price, maximum_price,
                            last_price, regular_transactions_quantity, non_regular_transactions_quantity,
                            regular_traded_contracts, non_regular_traded_contracts,
                            national_regular_volume, national_non_regular_volume,
                            international_regular_volume, international_non_regular_volume,
                            oscillation_percentage, maximum_trade_limit, minimum_trade_limit,
                            resolution_status, source_member_name, source_row_hash,
                            created_at, updated_at, last_seen_at, data_changed_at, field_changed_at
                        )
                        VALUES (
                            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14,
                            $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25,
                            $26, $27, $28, $29, $30, $31, $32, $33, $34, NOW(), NOW(), NOW(), NOW(), '{}'::jsonb
                        )
                        ON CONFLICT (trade_date, security_id, ticker_symbol, market_identifier_code, days_to_settlement) DO NOTHING
                        "#,
                    )
                    .bind(&new_id)
                    .bind(source_file_id)
                    .bind(&res.instrument_id)
                    .bind(trade_dt)
                    .bind(&row.ticker_symbol)
                    .bind(&row.security_id)
                    .bind(&row.market_identifier_code)
                    .bind(row.days_to_settlement)
                    .bind(row.trade_quantity)
                    .bind(row.financial_volume_brl)
                    .bind(row.financial_volume_usd)
                    .bind(row.open_interest)
                    .bind(row.financial_instrument_quantity)
                    .bind(row.best_bid_price)
                    .bind(row.best_ask_price)
                    .bind(row.first_price)
                    .bind(row.minimum_price)
                    .bind(row.average_price)
                    .bind(row.maximum_price)
                    .bind(row.last_price)
                    .bind(row.regular_transactions_quantity)
                    .bind(row.non_regular_transactions_quantity)
                    .bind(row.regular_traded_contracts)
                    .bind(row.non_regular_traded_contracts)
                    .bind(row.national_regular_volume)
                    .bind(row.national_non_regular_volume)
                    .bind(row.international_regular_volume)
                    .bind(row.international_non_regular_volume)
                    .bind(row.oscillation_percentage)
                    .bind(row.maximum_trade_limit)
                    .bind(row.minimum_trade_limit)
                    .bind(res.status.as_str())
                    .bind(member_name)
                    .bind(&hash)
                    .execute(&self.pool)
                    .await?;

                    if result.rows_affected() > 0 {
                        inserted += 1;
                    } else {
                        unchanged += 1;
                    }
                }
            }
            SourceFileType::Bvbg186EquitiesSimplifiedPriceReport => {
                let parsed = parse_bvbg_186(Cursor::new(content))?;
                warnings.extend(parsed.warnings);

                for row in parsed.rows {
                    let trade_dt = row.trade_date.unwrap_or(reference_date);
                    let res = resolver
                        .resolve_instrument(
                            row.security_id.as_deref(),
                            row.ticker_symbol.as_deref(),
                            None,
                            row.market_identifier_code.as_deref(),
                            Some(trade_dt),
                        )
                        .await?;

                    if res.status != ResolutionStatus::Resolved {
                        unresolved += 1;
                    }

                    let new_id = Uuid::new_v4().to_string();
                    let mut fields = BTreeMap::new();
                    fields.insert("trade_date".to_string(), serde_json::Value::String(trade_dt.to_string()));
                    if let Some(ref val) = row.ticker_symbol {
                        fields.insert("ticker_symbol".to_string(), serde_json::Value::String(val.clone()));
                    }
                    if let Some(ref val) = row.last_price {
                        fields.insert("last_price".to_string(), serde_json::Value::String(val.to_string()));
                    }
                    let hash = stable_row_hash(&fields);

                    let result = sqlx::query(
                        r#"
                        INSERT INTO b3_equities_simplified_price_quotes (
                            id, source_file_id, instrument_id, trade_date, ticker_symbol,
                            security_id, market_identifier_code, first_price, minimum_price,
                            average_price, maximum_price, last_price, financial_volume_brl,
                            resolution_status, source_member_name, source_row_hash,
                            created_at, updated_at, last_seen_at, data_changed_at, field_changed_at
                        )
                        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, NOW(), NOW(), NOW(), NOW(), '{}'::jsonb)
                        ON CONFLICT (trade_date, security_id, ticker_symbol, market_identifier_code) DO NOTHING
                        "#,
                    )
                    .bind(&new_id)
                    .bind(source_file_id)
                    .bind(&res.instrument_id)
                    .bind(trade_dt)
                    .bind(&row.ticker_symbol)
                    .bind(&row.security_id)
                    .bind(&row.market_identifier_code)
                    .bind(row.first_price)
                    .bind(row.minimum_price)
                    .bind(row.average_price)
                    .bind(row.maximum_price)
                    .bind(row.last_price)
                    .bind(row.financial_volume_brl)
                    .bind(res.status.as_str())
                    .bind(member_name)
                    .bind(&hash)
                    .execute(&self.pool)
                    .await?;

                    if result.rows_affected() > 0 {
                        inserted += 1;
                    } else {
                        unchanged += 1;
                    }
                }
            }
            SourceFileType::Bvbg087IndexReport => {
                let parsed = parse_bvbg_087(Cursor::new(content))?;
                warnings.extend(parsed.warnings);

                for row in parsed.rows {
                    let trade_dt = row.trade_date.unwrap_or(reference_date);
                    let index_code = row.index_code.unwrap_or_default();
                    let value_type = row.value_type.unwrap_or_else(|| "index_value".to_string());

                    let mut hash_fields = BTreeMap::new();
                    hash_fields.insert("index_code".to_string(), serde_json::Value::String(index_code.clone()));
                    let index_hash = stable_row_hash(&hash_fields);

                    let index_id = resolver
                        .resolve_or_create_index(source_file_id, &index_code, row.index_name.as_deref(), &index_hash)
                        .await?;

                    let new_id = Uuid::new_v4().to_string();
                    let mut fields = BTreeMap::new();
                    fields.insert("trade_date".to_string(), serde_json::Value::String(trade_dt.to_string()));
                    fields.insert("index_code".to_string(), serde_json::Value::String(index_code.clone()));
                    fields.insert("value_type".to_string(), serde_json::Value::String(value_type.clone()));
                    if let Some(ref v) = row.value {
                        fields.insert("value".to_string(), serde_json::Value::String(v.to_string()));
                    }
                    let hash = stable_row_hash(&fields);

                    let result = sqlx::query(
                        r#"
                        INSERT INTO b3_index_report_values (
                            id, source_file_id, index_id, trade_date, index_code,
                            index_name, value_type, value, currency, market_identifier_code,
                            source_member_name, source_row_hash, created_at, updated_at,
                            last_seen_at, data_changed_at, field_changed_at
                        )
                        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW(), NOW(), NOW(), NOW(), '{}'::jsonb)
                        ON CONFLICT (trade_date, index_code, value_type) DO NOTHING
                        "#,
                    )
                    .bind(&new_id)
                    .bind(source_file_id)
                    .bind(&index_id)
                    .bind(trade_dt)
                    .bind(&index_code)
                    .bind(&row.index_name)
                    .bind(&value_type)
                    .bind(row.value)
                    .bind(&row.currency)
                    .bind(&row.market_identifier_code)
                    .bind(member_name)
                    .bind(&hash)
                    .execute(&self.pool)
                    .await?;

                    if result.rows_affected() > 0 {
                        inserted += 1;
                    } else {
                        unchanged += 1;
                    }
                }
            }
            SourceFileType::B3IndexPortfolioComposition => {
                let parsed = parse_index_composition_csv(Cursor::new(content))?;
                warnings.extend(parsed.warnings);

                for row in parsed.rows {
                    let mut hash_fields = BTreeMap::new();
                    hash_fields.insert("index_code".to_string(), serde_json::Value::String(row.index_code.clone()));
                    let index_hash = stable_row_hash(&hash_fields);

                    let index_id = resolver
                        .resolve_or_create_index(source_file_id, &row.index_code, None, &index_hash)
                        .await?;

                    let res = resolver
                        .resolve_instrument(
                            None,
                            row.ticker_symbol.as_deref(),
                            row.isin.as_deref(),
                            None,
                            Some(row.composition_date),
                        )
                        .await?;

                    if res.status != ResolutionStatus::Resolved {
                        unresolved += 1;
                    }

                    let new_id = Uuid::new_v4().to_string();
                    let mut fields = BTreeMap::new();
                    fields.insert("composition_date".to_string(), serde_json::Value::String(row.composition_date.to_string()));
                    fields.insert("index_code".to_string(), serde_json::Value::String(row.index_code.clone()));
                    if let Some(ref t) = row.ticker_symbol {
                        fields.insert("ticker_symbol".to_string(), serde_json::Value::String(t.clone()));
                    }
                    let hash = stable_row_hash(&fields);

                    let result = sqlx::query(
                        r#"
                        INSERT INTO b3_index_portfolio_compositions (
                            id, source_file_id, index_id, instrument_id, composition_date,
                            index_code, ticker_symbol, isin, asset_name, theoretical_quantity,
                            portfolio_weight, participation_percentage, resolution_status,
                            source_member_name, source_row_hash, created_at, updated_at,
                            last_seen_at, data_changed_at, field_changed_at
                        )
                        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, NOW(), NOW(), NOW(), NOW(), '{}'::jsonb)
                        ON CONFLICT (composition_date, index_code, ticker_symbol, instrument_id) DO NOTHING
                        "#,
                    )
                    .bind(&new_id)
                    .bind(source_file_id)
                    .bind(&index_id)
                    .bind(&res.instrument_id)
                    .bind(row.composition_date)
                    .bind(&row.index_code)
                    .bind(&row.ticker_symbol)
                    .bind(&row.isin)
                    .bind(&row.asset_name)
                    .bind(row.theoretical_quantity)
                    .bind(row.portfolio_weight)
                    .bind(row.participation_percentage)
                    .bind(res.status.as_str())
                    .bind(member_name)
                    .bind(&hash)
                    .execute(&self.pool)
                    .await?;

                    if result.rows_affected() > 0 {
                        inserted += 1;
                    } else {
                        unchanged += 1;
                    }
                }
            }
        }

        Ok((inserted, changed, unchanged, unresolved, warnings))
    }
}
