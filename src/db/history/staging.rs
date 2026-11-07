//! Bounded batch staging; occurrence, fact, version and counter writes share a fenced transaction.
use super::HistoryRepository;
use crate::{
    error::{AppError, Result},
    parsers::{
        canonical::{canonicalize_string, stable_row_hash},
        cotahist::CotahistRow,
    },
    services::jobs::{JobClaim, JobService},
};
use chrono::NaiveDate;
use sqlx::{Postgres, QueryBuilder, Row};
use std::collections::BTreeMap;
use uuid::Uuid;

pub struct LocatedQuote {
    pub record_index: i64,
    pub row: CotahistRow,
}
pub struct LocatedRejection {
    pub record_index: i64,
    pub byte_offset: Option<i64>,
    pub raw_payload: String,
    pub reason_code: String,
    pub message: String,
}
pub struct MemberOutcome {
    pub detail_read: i64,
    pub rejected: i64,
    pub out_of_scope: i64,
    pub structural_valid: bool,
}
#[derive(Debug, Default)]
pub struct BatchStats {
    pub inserted: i64,
    pub changed: i64,
    pub unchanged: i64,
    pub duplicate_in_scope: i64,
}

fn canonical(row: &CotahistRow) -> Result<(CotahistRow, String)> {
    let mut row = row.clone();
    row.ticker_symbol = canonicalize_string(row.ticker_symbol.as_deref());
    row.market_type = canonicalize_string(row.market_type.as_deref());
    row.bdi_code = canonicalize_string(row.bdi_code.as_deref());
    row.distribution_number = canonicalize_string(row.distribution_number.as_deref());
    row.term_days = canonicalize_string(row.term_days.as_deref());
    row.issuer_short_name = canonicalize_string(row.issuer_short_name.as_deref());
    row.paper_specification = canonicalize_string(row.paper_specification.as_deref());
    row.currency = canonicalize_string(row.currency.as_deref());
    row.price_correction_indicator = canonicalize_string(row.price_correction_indicator.as_deref());
    row.isin = canonicalize_string(row.isin.as_deref());
    row.ticker_symbol = row.ticker_symbol.map(|v| v.to_uppercase());
    row.isin = row.isin.map(|v| v.to_uppercase());
    if row.trade_date.is_none() || row.ticker_symbol.is_none() {
        return Err(AppError::Validation("Data/ticker obrigatórios.".into()));
    }
    let mut fields = BTreeMap::new();
    fields.insert(
        "trade_date".into(),
        row.trade_date
            .map(|v| serde_json::Value::String(v.to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    fields.insert(
        "ticker_symbol".into(),
        row.ticker_symbol
            .clone()
            .map(|v| serde_json::Value::String(v.to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    fields.insert(
        "market_type".into(),
        row.market_type
            .clone()
            .map(|v| serde_json::Value::String(v.to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    fields.insert(
        "bdi_code".into(),
        row.bdi_code
            .clone()
            .map(|v| serde_json::Value::String(v.to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    fields.insert(
        "distribution_number".into(),
        row.distribution_number
            .clone()
            .map(|v| serde_json::Value::String(v.to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    fields.insert(
        "term_days".into(),
        row.term_days
            .clone()
            .map(|v| serde_json::Value::String(v.to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    fields.insert(
        "issuer_short_name".into(),
        row.issuer_short_name
            .clone()
            .map(|v| serde_json::Value::String(v.to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    fields.insert(
        "paper_specification".into(),
        row.paper_specification
            .clone()
            .map(|v| serde_json::Value::String(v.to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    fields.insert(
        "currency".into(),
        row.currency
            .clone()
            .map(|v| serde_json::Value::String(v.to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    fields.insert(
        "open_price".into(),
        row.open_price
            .map(|v| serde_json::Value::String(v.normalize().to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    fields.insert(
        "high_price".into(),
        row.high_price
            .map(|v| serde_json::Value::String(v.normalize().to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    fields.insert(
        "low_price".into(),
        row.low_price
            .map(|v| serde_json::Value::String(v.normalize().to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    fields.insert(
        "average_price".into(),
        row.average_price
            .map(|v| serde_json::Value::String(v.normalize().to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    fields.insert(
        "last_price".into(),
        row.last_price
            .map(|v| serde_json::Value::String(v.normalize().to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    fields.insert(
        "best_bid_price".into(),
        row.best_bid_price
            .map(|v| serde_json::Value::String(v.normalize().to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    fields.insert(
        "best_ask_price".into(),
        row.best_ask_price
            .map(|v| serde_json::Value::String(v.normalize().to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    fields.insert(
        "trades_count".into(),
        row.trades_count
            .map(|v| serde_json::Value::String(v.to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    fields.insert(
        "traded_quantity".into(),
        row.traded_quantity
            .map(|v| serde_json::Value::String(v.normalize().to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    fields.insert(
        "traded_volume".into(),
        row.traded_volume
            .map(|v| serde_json::Value::String(v.normalize().to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    fields.insert(
        "option_exercise_price".into(),
        row.option_exercise_price
            .map(|v| serde_json::Value::String(v.normalize().to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    fields.insert(
        "price_correction_indicator".into(),
        row.price_correction_indicator
            .clone()
            .map(|v| serde_json::Value::String(v.to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    fields.insert(
        "maturity_date".into(),
        row.maturity_date
            .map(|v| serde_json::Value::String(v.to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    fields.insert(
        "quotation_factor".into(),
        row.quotation_factor
            .map(|v| serde_json::Value::String(v.to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    fields.insert(
        "exercise_price_points".into(),
        row.exercise_price_points
            .map(|v| serde_json::Value::String(v.normalize().to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    fields.insert(
        "isin".into(),
        row.isin
            .clone()
            .map(|v| serde_json::Value::String(v.to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    Ok((row, stable_row_hash(&fields)))
}

impl HistoryRepository {
    pub async fn stage_batch(
        &self,
        claim: &JobClaim,
        revision: Uuid,
        member: &str,
        rows: &[LocatedQuote],
    ) -> Result<BatchStats> {
        if rows.len() > 1000 || rows.iter().any(|r| r.record_index <= 0) {
            return Err(AppError::Validation("Lote ou localizador inválido.".into()));
        }
        let rows: Vec<_> = rows
            .iter()
            .map(|r| canonical(&r.row).map(|(row, hash)| (r.record_index, row, hash)))
            .collect::<Result<_>>()?;
        let jobs = JobService::new(self.pool.clone());
        let mut tx = jobs.begin_confirmation(claim).await?;
        let spec = Self::draft(&mut tx, revision).await?;
        let start: NaiveDate = spec.get("period_start");
        let end: NaiveDate = spec.get("period_end");
        if rows
            .iter()
            .any(|(_, r, _)| r.trade_date.is_none_or(|d| d < start || d > end))
        {
            return Err(AppError::Validation(
                "Registro fora do período da publicação.".into(),
            ));
        }
        let valid: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM b3_source_file_members m JOIN b3_history_revision_sources s ON s.source_file_id=m.source_file_id WHERE m.id=$1 AND s.revision_id=$2)",
        )
        .bind(member)
        .bind(revision)
        .fetch_one(&mut *tx)
        .await?;
        if !valid {
            return Err(AppError::Validation(
                "Member não pertence ao arquivo da revisão.".into(),
            ));
        }
        sqlx::query("INSERT INTO b3_history_revision_members(revision_id,source_member_id) VALUES($1,$2) ON CONFLICT DO NOTHING").bind(revision).bind(member).execute(&mut *tx).await?;
        let member_status:String=sqlx::query_scalar("SELECT status FROM b3_history_revision_members WHERE revision_id=$1 AND source_member_id=$2 FOR UPDATE").bind(revision).bind(member).fetch_one(&mut *tx).await?;
        if member_status != "pending" {
            return Err(AppError::Validation("Member já finalizado.".into()));
        }
        sqlx::query("CREATE TEMP TABLE b3_history_stage(record_index BIGINT,business_hash TEXT,trade_date DATE,ticker_symbol TEXT,market_type TEXT,bdi_code TEXT,distribution_number TEXT,term_days TEXT,issuer_short_name TEXT,paper_specification TEXT,currency TEXT,open_price NUMERIC(38,10),high_price NUMERIC(38,10),low_price NUMERIC(38,10),average_price NUMERIC(38,10),last_price NUMERIC(38,10),best_bid_price NUMERIC(38,10),best_ask_price NUMERIC(38,10),trades_count INT,traded_quantity NUMERIC(38,10),traded_volume NUMERIC(38,10),option_exercise_price NUMERIC(38,10),price_correction_indicator TEXT,maturity_date DATE,quotation_factor INT,exercise_price_points NUMERIC(38,10),isin TEXT) ON COMMIT DROP").execute(&mut *tx).await?;
        if !rows.is_empty() {
            let mut q=QueryBuilder::<Postgres>::new("INSERT INTO b3_history_stage(record_index,business_hash,trade_date,ticker_symbol,market_type,bdi_code,distribution_number,term_days,issuer_short_name,paper_specification,currency,open_price,high_price,low_price,average_price,last_price,best_bid_price,best_ask_price,trades_count,traded_quantity,traded_volume,option_exercise_price,price_correction_indicator,maturity_date,quotation_factor,exercise_price_points,isin) ");
            q.push_values(&rows, |mut b, (index, row, hash)| {
                b.push_bind(index)
                    .push_bind(hash)
                    .push_bind(row.trade_date)
                    .push_bind(row.ticker_symbol.clone())
                    .push_bind(row.market_type.clone())
                    .push_bind(row.bdi_code.clone())
                    .push_bind(row.distribution_number.clone())
                    .push_bind(row.term_days.clone())
                    .push_bind(row.issuer_short_name.clone())
                    .push_bind(row.paper_specification.clone())
                    .push_bind(row.currency.clone())
                    .push_bind(row.open_price)
                    .push_bind(row.high_price)
                    .push_bind(row.low_price)
                    .push_bind(row.average_price)
                    .push_bind(row.last_price)
                    .push_bind(row.best_bid_price)
                    .push_bind(row.best_ask_price)
                    .push_bind(row.trades_count)
                    .push_bind(row.traded_quantity)
                    .push_bind(row.traded_volume)
                    .push_bind(row.option_exercise_price)
                    .push_bind(row.price_correction_indicator.clone())
                    .push_bind(row.maturity_date)
                    .push_bind(row.quotation_factor)
                    .push_bind(row.exercise_price_points)
                    .push_bind(row.isin.clone());
            });
            q.build().execute(&mut *tx).await?;
        }
        let mismatch:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM b3_history_stage s JOIN b3_cotahist_occurrences o ON o.revision_id=$1 AND o.source_member_id=$2 AND o.record_index=s.record_index JOIN b3_cotahist_versions v ON v.id=o.version_id WHERE v.business_hash<>s.business_hash)").bind(revision).bind(member).fetch_one(&mut *tx).await?;
        if mismatch {
            return Err(AppError::Validation(
                "Ocorrência já persistida possui conteúdo diferente.".into(),
            ));
        }
        sqlx::query("DELETE FROM b3_history_stage s USING b3_cotahist_occurrences o WHERE o.revision_id=$1 AND o.source_member_id=$2 AND o.record_index=s.record_index").bind(revision).bind(member).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO b3_cotahist_facts(trade_date,ticker_symbol,market_type,bdi_code,distribution_number,term_days) SELECT DISTINCT trade_date,ticker_symbol,market_type,bdi_code,distribution_number,term_days FROM b3_history_stage ON CONFLICT DO NOTHING").execute(&mut *tx).await?;
        sqlx::query(
            r#"
            INSERT INTO b3_cotahist_versions(
                fact_id,business_hash,issuer_short_name,paper_specification,currency,
                open_price,high_price,low_price,average_price,last_price,best_bid_price,best_ask_price,
                trades_count,traded_quantity,traded_volume,option_exercise_price,
                price_correction_indicator,maturity_date,quotation_factor,exercise_price_points,isin,
                field_changed_at
            )
            SELECT
                f.id,s.business_hash,s.issuer_short_name,s.paper_specification,s.currency,
                s.open_price,s.high_price,s.low_price,s.average_price,s.last_price,s.best_bid_price,s.best_ask_price,
                s.trades_count,s.traded_quantity,s.traded_volume,s.option_exercise_price,
                s.price_correction_indicator,s.maturity_date,s.quotation_factor,s.exercise_price_points,s.isin,
                COALESCE(old.field_changed_at,'{}'::jsonb)||COALESCE((
                    SELECT jsonb_object_agg(n.key,to_jsonb(NOW()))
                    FROM jsonb_each(to_jsonb(s)-ARRAY['record_index','business_hash','trade_date','ticker_symbol','market_type','bdi_code','distribution_number','term_days']) n
                    LEFT JOIN LATERAL jsonb_each(to_jsonb(old)) o ON o.key=n.key
                    WHERE n.value IS DISTINCT FROM o.value AND (old.id IS NOT NULL OR n.value<>'null'::jsonb)
                ),'{}'::jsonb)
            FROM b3_history_stage s
            JOIN LATERAL (
                SELECT f.id
                FROM b3_cotahist_facts f
                WHERE f.trade_date=s.trade_date
                  AND f.ticker_symbol=s.ticker_symbol
                  AND f.market_type=s.market_type
                  AND f.distribution_number=s.distribution_number
                  AND f.bdi_code IS NOT DISTINCT FROM s.bdi_code
                  AND f.term_days IS NOT DISTINCT FROM s.term_days
                LIMIT 1
            ) f ON TRUE
            LEFT JOIN LATERAL (
                SELECT previous.version_id
                FROM b3_history_revision_quotes previous
                WHERE previous.revision_id=$1 AND previous.fact_id=f.id
                LIMIT 1
            ) previous ON TRUE
            LEFT JOIN b3_cotahist_versions old ON old.id=previous.version_id
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(revision)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            r#"
            CREATE TEMP TABLE b3_history_incoming ON COMMIT DROP AS
            SELECT s.record_index,f.id fact_id,v.id version_id
            FROM b3_history_stage s
            JOIN LATERAL (
                SELECT f.id
                FROM b3_cotahist_facts f
                WHERE f.trade_date=s.trade_date
                  AND f.ticker_symbol=s.ticker_symbol
                  AND f.market_type=s.market_type
                  AND f.distribution_number=s.distribution_number
                  AND f.bdi_code IS NOT DISTINCT FROM s.bdi_code
                  AND f.term_days IS NOT DISTINCT FROM s.term_days
                LIMIT 1
            ) f ON TRUE
            JOIN b3_cotahist_versions v ON v.fact_id=f.id AND v.business_hash=s.business_hash
            "#,
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO b3_history_conflicts(revision_id,fact_id,existing_version_id,incoming_version_id,source_member_id,record_index) SELECT $1,i.fact_id,q.version_id,i.version_id,$2,i.record_index FROM b3_history_incoming i JOIN LATERAL (SELECT q.version_id,q.selected_publication_key,q.selected_official_revision FROM b3_history_revision_quotes q WHERE q.revision_id=$1 AND q.fact_id=i.fact_id LIMIT 1) q ON TRUE JOIN b3_source_file_members m ON m.id=$2 JOIN b3_source_files incoming ON incoming.id=m.source_file_id JOIN b3_history_revisions r ON r.id=$1 WHERE q.version_id<>i.version_id AND NOT COALESCE(incoming.replaces_publication AND q.selected_publication_key=incoming.publication_key AND incoming.official_revision>q.selected_official_revision,FALSE) ON CONFLICT DO NOTHING").bind(revision).bind(member).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO b3_history_conflicts(revision_id,fact_id,existing_version_id,incoming_version_id,source_member_id,record_index) SELECT $1,a.fact_id,a.version_id,b.version_id,$2,b.record_index FROM b3_history_incoming a JOIN b3_history_incoming b ON a.fact_id=b.fact_id AND a.version_id<>b.version_id AND a.record_index<b.record_index ON CONFLICT DO NOTHING").bind(revision).bind(member).execute(&mut *tx).await?;
        let counts=sqlx::query("WITH classified AS (SELECT i.*,row_number() OVER(PARTITION BY i.fact_id ORDER BY i.record_index) AS rn,EXISTS(SELECT 1 FROM b3_cotahist_versions v JOIN LATERAL (SELECT 1 FROM b3_cotahist_occurrences o WHERE o.revision_id=$1 AND o.version_id=v.id LIMIT 1) o ON TRUE WHERE v.fact_id=i.fact_id) AS seen FROM b3_history_incoming i) SELECT count(*) FILTER(WHERE rn=1 AND NOT seen AND q.fact_id IS NULL)::bigint inserted,count(*) FILTER(WHERE rn=1 AND NOT seen AND q.version_id<>i.version_id)::bigint changed,count(*) FILTER(WHERE rn=1 AND NOT seen AND q.version_id=i.version_id)::bigint unchanged,count(*) FILTER(WHERE rn>1 OR seen)::bigint duplicates FROM classified i LEFT JOIN LATERAL (SELECT q.fact_id,q.version_id FROM b3_history_revision_quotes q WHERE q.revision_id=$1 AND q.fact_id=i.fact_id LIMIT 1) q ON TRUE").bind(revision).fetch_one(&mut *tx).await?;
        let stats = BatchStats {
            inserted: counts.get("inserted"),
            changed: counts.get("changed"),
            unchanged: counts.get("unchanged"),
            duplicate_in_scope: counts.get("duplicates"),
        };
        sqlx::query("INSERT INTO b3_field_changes(id,source_file_id,table_name,record_pk,field_name,old_value_text,new_value_text,history_revision_id) SELECT gen_random_uuid()::text,m.source_file_id,'b3_cotahist_versions',i.fact_id::text,n.key,o.value #>> '{}',n.value #>> '{}',$1 FROM (SELECT DISTINCT fact_id,version_id FROM b3_history_incoming) i JOIN LATERAL (SELECT q.version_id FROM b3_history_revision_quotes q WHERE q.revision_id=$1 AND q.fact_id=i.fact_id AND q.version_id<>i.version_id LIMIT 1) q ON TRUE JOIN b3_cotahist_versions old ON old.id=q.version_id JOIN b3_cotahist_versions new ON new.id=i.version_id JOIN b3_source_file_members m ON m.id=$2 CROSS JOIN LATERAL jsonb_each(to_jsonb(new)-ARRAY['id','fact_id','business_hash','created_at','data_changed_at','field_changed_at']) n JOIN LATERAL jsonb_each(to_jsonb(old)) o ON o.key=n.key WHERE n.value IS DISTINCT FROM o.value AND NOT EXISTS(SELECT 1 FROM b3_history_conflicts c WHERE c.revision_id=$1 AND c.fact_id=i.fact_id)").bind(revision).bind(member).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO b3_cotahist_occurrences(revision_id,source_member_id,record_index,version_id) SELECT $1,$2,record_index,version_id FROM b3_history_incoming ON CONFLICT DO NOTHING").bind(revision).bind(member).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO b3_history_revision_quotes(revision_id,fact_id,version_id,source_member_id,record_index,selected_publication_key,selected_official_revision) SELECT DISTINCT ON(i.fact_id) $1,i.fact_id,i.version_id,$2,i.record_index,COALESCE(incoming.publication_key,r.publication_key),COALESCE(incoming.official_revision,r.official_revision) FROM b3_history_incoming i JOIN b3_history_revisions r ON r.id=$1 JOIN b3_source_file_members m ON m.id=$2 JOIN b3_source_files incoming ON incoming.id=m.source_file_id ORDER BY i.fact_id,i.record_index ON CONFLICT(revision_id,fact_id) DO UPDATE SET version_id=EXCLUDED.version_id,source_member_id=EXCLUDED.source_member_id,record_index=EXCLUDED.record_index,selected_publication_key=EXCLUDED.selected_publication_key,selected_official_revision=EXCLUDED.selected_official_revision WHERE NOT EXISTS(SELECT 1 FROM b3_history_conflicts c WHERE c.revision_id=$1 AND c.fact_id=EXCLUDED.fact_id)").bind(revision).bind(member).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(stats)
    }

    pub async fn quarantine_batch(
        &self,
        claim: &JobClaim,
        revision: Uuid,
        member: &str,
        rejected: &[LocatedRejection],
    ) -> Result<()> {
        if rejected.len() > 1000
            || rejected
                .iter()
                .any(|r| r.record_index <= 0 || r.raw_payload.len() > 4096)
        {
            return Err(AppError::Validation("Lote de rejeições inválido.".into()));
        }
        let jobs = JobService::new(self.pool.clone());
        let mut tx = jobs.begin_confirmation(claim).await?;
        Self::draft(&mut tx, revision).await?;
        let valid: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM b3_source_file_members m JOIN b3_history_revision_sources s ON s.source_file_id=m.source_file_id WHERE m.id=$1 AND s.revision_id=$2)",
        )
        .bind(member)
        .bind(revision)
        .fetch_one(&mut *tx)
        .await?;
        if !valid {
            return Err(AppError::Validation(
                "Member não pertence à revisão.".into(),
            ));
        }
        sqlx::query("INSERT INTO b3_history_revision_members(revision_id,source_member_id) VALUES($1,$2) ON CONFLICT DO NOTHING").bind(revision).bind(member).execute(&mut *tx).await?;
        if !rejected.is_empty() {
            let mut q=QueryBuilder::<Postgres>::new("INSERT INTO b3_history_rejections(revision_id,source_member_id,record_index,byte_offset,reason_code,message,raw_payload) ");
            q.push_values(rejected, |mut b, r| {
                b.push_bind(revision)
                    .push_bind(member)
                    .push_bind(r.record_index)
                    .push_bind(r.byte_offset)
                    .push_bind(&r.reason_code)
                    .push_bind(&r.message)
                    .push_bind(&r.raw_payload);
            });
            q.push(" ON CONFLICT DO NOTHING")
                .build()
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query("INSERT INTO b3_quarantine_records(id,source_file_id,source_member_name,source_file_type,reference_date,source_line_number,reason_code,error_message,raw_payload,history_revision_id,source_record_index,byte_offset) SELECT gen_random_uuid()::text,s.id,m.member_name,s.source_file_type,s.reference_date,CASE WHEN r.record_index<=2147483647 THEN r.record_index::int END,r.reason_code,r.message,jsonb_build_object('raw',r.raw_payload),$1,r.record_index,r.byte_offset FROM b3_history_rejections r JOIN b3_source_file_members m ON m.id=r.source_member_id JOIN b3_source_files s ON s.id=m.source_file_id WHERE r.revision_id=$1 AND r.source_member_id=$2 ON CONFLICT DO NOTHING").bind(revision).bind(member).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn complete_member(
        &self,
        claim: &JobClaim,
        revision: Uuid,
        member: &str,
        outcome: &MemberOutcome,
    ) -> Result<()> {
        let jobs = JobService::new(self.pool.clone());
        let mut tx = jobs.begin_confirmation(claim).await?;
        Self::draft(&mut tx, revision).await?;
        let valid: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM b3_source_file_members m JOIN b3_history_revision_sources s ON s.source_file_id=m.source_file_id WHERE m.id=$1 AND s.revision_id=$2)",
        )
        .bind(member)
        .bind(revision)
        .fetch_one(&mut *tx)
        .await?;
        if !valid || outcome.detail_read < 0 || outcome.rejected < 0 || outcome.out_of_scope < 0 {
            return Err(AppError::Validation("Resultado de member inválido.".into()));
        }
        let accepted:i64=sqlx::query_scalar("SELECT count(*) FROM b3_cotahist_occurrences WHERE revision_id=$1 AND source_member_id=$2").bind(revision).bind(member).fetch_one(&mut *tx).await?;
        let rejected:i64=sqlx::query_scalar("SELECT count(*) FROM b3_history_rejections WHERE revision_id=$1 AND source_member_id=$2").bind(revision).bind(member).fetch_one(&mut *tx).await?;
        if outcome.structural_valid
            && (accepted + rejected + outcome.out_of_scope != outcome.detail_read
                || rejected != outcome.rejected)
        {
            return Err(AppError::Validation(
                "Contadores não fecham com ocorrências/rejeições persistidas.".into(),
            ));
        }
        let status = if outcome.structural_valid {
            "complete"
        } else {
            "invalid"
        };
        sqlx::query("INSERT INTO b3_history_revision_members(revision_id,source_member_id,status,detail_read,rejected,out_of_scope) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(revision_id,source_member_id) DO UPDATE SET status=EXCLUDED.status,detail_read=EXCLUDED.detail_read,rejected=EXCLUDED.rejected,out_of_scope=EXCLUDED.out_of_scope").bind(revision).bind(member).bind(status).bind(outcome.detail_read).bind(rejected).bind(outcome.out_of_scope).execute(&mut *tx).await?;
        if !outcome.structural_valid {
            sqlx::query("UPDATE b3_history_revisions SET status='invalid' WHERE id=$1")
                .bind(revision)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
