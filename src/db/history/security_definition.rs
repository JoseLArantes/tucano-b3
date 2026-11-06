use super::HistoryRepository;
use crate::{
    error::{AppError, Result},
    parsers::{canonical::stable_row_hash, security_definition::SecurityDefinitionRecord},
    services::jobs::{JobClaim, JobService},
};
use sqlx::{Postgres, QueryBuilder};
use std::collections::BTreeMap;
use uuid::Uuid;

fn business_hash(record: &SecurityDefinitionRecord) -> Result<String> {
    let mut normalized = record.clone();
    normalized.strike_price = normalized.strike_price.map(|v| v.normalize());
    normalized.contract_multiplier = normalized.contract_multiplier.map(|v| v.normalize());
    normalized.price_divisor = normalized.price_divisor.map(|v| v.normalize());
    normalized.min_price_increment = normalized.min_price_increment.map(|v| v.normalize());
    normalized.min_order_qty = normalized.min_order_qty.map(|v| v.normalize());
    normalized.max_order_qty = normalized.max_order_qty.map(|v| v.normalize());
    normalized.no_shares_issued = normalized.no_shares_issued.map(|v| v.normalize());
    normalized.min_cross_qty = normalized.min_cross_qty.map(|v| v.normalize());
    normalized.min_trade_vol = normalized.min_trade_vol.map(|v| v.normalize());
    for item in &mut normalized.underlying {
        item.index_pct = item.index_pct.map(|v| v.normalize());
    }
    for item in &mut normalized.underlying {
        item.index_theoretical_qty = item.index_theoretical_qty.map(|v| v.normalize());
    }
    for item in &mut normalized.legs {
        item.ratio_quantity = item.ratio_quantity.map(|v| v.normalize());
    }
    for item in &mut normalized.tick_rules {
        item.start_price = item.start_price.map(|v| v.normalize());
    }
    for item in &mut normalized.tick_rules {
        item.end_price = item.end_price.map(|v| v.normalize());
    }
    for item in &mut normalized.tick_rules {
        item.increment = item.increment.map(|v| v.normalize());
    }
    for item in &mut normalized.lot_rules {
        item.min_lot_size = item.min_lot_size.map(|v| v.normalize());
    }
    let mut fields: BTreeMap<String, serde_json::Value> =
        serde_json::from_value(serde_json::to_value(normalized)?)?;
    for key in [
        "NoApplIDs",
        "NoUnderlyings",
        "NoInstrAttrib",
        "NoLegs",
        "NoTickRules",
        "NoLotTypeRules",
    ] {
        if let Some(serde_json::Value::Array(values)) = fields.get_mut(key) {
            values.sort_by_cached_key(|v| v.to_string());
        }
    }
    Ok(stable_row_hash(&fields))
}
impl HistoryRepository {
    pub async fn stage_security_definition_groups(
        &self,
        claim: &JobClaim,
        snapshot: Uuid,
        member: &str,
        rows: &[(i64, SecurityDefinitionRecord)],
    ) -> Result<()> {
        if rows.len() > 1000 {
            return Err(AppError::Validation(
                "Lote de Security Definition excedido.".into(),
            ));
        }
        let normalized_rows: Vec<_> = rows
            .iter()
            .map(|(i, r)| {
                let mut r = r.clone();
                r.normalize();
                r.validate()?;
                Ok((*i, r))
            })
            .collect::<Result<_>>()?;
        let rows = normalized_rows.as_slice();
        let mut bytes = 0usize;
        for (_, record) in rows {
            bytes = bytes.saturating_add(serde_json::to_vec(record)?.len());
            if bytes > 4 * 1024 * 1024 {
                return Err(AppError::Validation(
                    "Lote de Security Definition excede orçamento de bytes.".into(),
                ));
            }
        }
        let jobs = JobService::new(self.pool.clone());
        let mut tx = jobs.begin_confirmation(claim).await?;
        let allowed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM b3_registry_snapshots s JOIN b3_source_file_members m ON m.source_file_id=s.source_file_id WHERE s.id=$1 AND s.status='draft' AND s.source_file_type='TRADABLE_SECURITY_LIST' AND m.id=$2)").bind(snapshot).bind(member).fetch_one(&mut *tx).await?;
        if !allowed {
            return Err(AppError::Validation(
                "Fotografia/member Security Definition inválido.".into(),
            ));
        }
        let rows: Vec<_> = rows
            .iter()
            .map(|(i, r)| business_hash(r).map(|h| (*i, r, h)))
            .collect::<Result<_>>()?;
        if !rows.is_empty() {
            let mut q=QueryBuilder::<Postgres>::new("INSERT INTO b3_security_definitions(snapshot_id,source_member_id,record_index,symbol,security_id,security_id_source,security_exchange,isin_number,product,cfi_code,security_group,security_type,security_sub_type,maturity_month_year,maturity_date,issue_date,country_of_issue,put_or_call,strike_price,strike_currency,exercise_style,contract_multiplier,security_desc,contract_settl_month,dated_date,settl_type,settl_date,price_divisor,min_price_increment,tick_size_denominator,min_order_qty,max_order_qty,multi_leg_model,multi_leg_price_method,start_date,end_date,currency,settl_currency,security_strategy_type,asset,no_shares_issued,security_validity_timestamp,market_segment_id,governance_indicator,corporate_action_event_id,security_match_type,implied_market_indicator,min_cross_qty,min_trade_vol,clearing_house_id,business_hash) ");
            q.push_values(&rows, |mut b, (index, r, hash)| {
                b.push_bind(snapshot)
                    .push_bind(member)
                    .push_bind(index)
                    .push_bind(&r.symbol)
                    .push_bind(&r.security_id)
                    .push_bind(&r.security_id_source)
                    .push_bind(&r.security_exchange)
                    .push_bind(&r.isin_number)
                    .push_bind(&r.product)
                    .push_bind(&r.cfi_code)
                    .push_bind(&r.security_group)
                    .push_bind(&r.security_type)
                    .push_bind(&r.security_sub_type)
                    .push_bind(&r.maturity_month_year)
                    .push_bind(&r.maturity_date)
                    .push_bind(&r.issue_date)
                    .push_bind(&r.country_of_issue)
                    .push_bind(&r.put_or_call)
                    .push_bind(r.strike_price)
                    .push_bind(&r.strike_currency)
                    .push_bind(&r.exercise_style)
                    .push_bind(r.contract_multiplier)
                    .push_bind(&r.security_desc)
                    .push_bind(&r.contract_settl_month)
                    .push_bind(&r.dated_date)
                    .push_bind(&r.settl_type)
                    .push_bind(&r.settl_date)
                    .push_bind(r.price_divisor)
                    .push_bind(r.min_price_increment)
                    .push_bind(&r.tick_size_denominator)
                    .push_bind(r.min_order_qty)
                    .push_bind(r.max_order_qty)
                    .push_bind(&r.multi_leg_model)
                    .push_bind(&r.multi_leg_price_method)
                    .push_bind(&r.start_date)
                    .push_bind(&r.end_date)
                    .push_bind(&r.currency)
                    .push_bind(&r.settl_currency)
                    .push_bind(&r.security_strategy_type)
                    .push_bind(&r.asset)
                    .push_bind(r.no_shares_issued)
                    .push_bind(&r.security_validity_timestamp)
                    .push_bind(&r.market_segment_id)
                    .push_bind(&r.governance_indicator)
                    .push_bind(&r.corporate_action_event_id)
                    .push_bind(&r.security_match_type)
                    .push_bind(&r.implied_market_indicator)
                    .push_bind(r.min_cross_qty)
                    .push_bind(r.min_trade_vol)
                    .push_bind(&r.clearing_house_id)
                    .push_bind(hash);
            });
            let written=q.push(" ON CONFLICT(snapshot_id,source_member_id,record_index) DO UPDATE SET business_hash=b3_security_definitions.business_hash WHERE b3_security_definitions.business_hash=EXCLUDED.business_hash").build().execute(&mut *tx).await?.rows_affected();
            if written != rows.len() as u64 {
                return Err(AppError::Validation(
                    "Ocorrência Security Definition já possui conteúdo diferente.".into(),
                ));
            }
        }
        let mut children = Vec::new();
        for (index, r, _) in &rows {
            for (group_index, item) in r.appl_ids.iter().enumerate() {
                children.push((*index, group_index as i32, item));
            }
        }
        for chunk in children.chunks(1000) {
            let mut q=QueryBuilder::<Postgres>::new("INSERT INTO b3_security_definition_appl_ids(snapshot_id,source_member_id,record_index,group_index,appl_id) ");
            q.push_values(chunk, |mut b, (index, group_index, r)| {
                b.push_bind(snapshot)
                    .push_bind(member)
                    .push_bind(index)
                    .push_bind(group_index)
                    .push_bind(&r.appl_id);
            });
            q.push(" ON CONFLICT DO NOTHING")
                .build()
                .execute(&mut *tx)
                .await?;
        }
        let mut children = Vec::new();
        for (index, r, _) in &rows {
            for (group_index, item) in r.underlying.iter().enumerate() {
                children.push((*index, group_index as i32, item));
            }
        }
        for chunk in children.chunks(1000) {
            let mut q=QueryBuilder::<Postgres>::new("INSERT INTO b3_security_definition_underlying(snapshot_id,source_member_id,record_index,group_index,symbol,security_id,security_id_source,security_exchange,index_pct,index_theoretical_qty) ");
            q.push_values(chunk, |mut b, (index, group_index, r)| {
                b.push_bind(snapshot)
                    .push_bind(member)
                    .push_bind(index)
                    .push_bind(group_index)
                    .push_bind(&r.symbol)
                    .push_bind(&r.security_id)
                    .push_bind(&r.security_id_source)
                    .push_bind(&r.security_exchange)
                    .push_bind(r.index_pct)
                    .push_bind(r.index_theoretical_qty);
            });
            q.push(" ON CONFLICT DO NOTHING")
                .build()
                .execute(&mut *tx)
                .await?;
        }
        let mut children = Vec::new();
        for (index, r, _) in &rows {
            for (group_index, item) in r.attributes.iter().enumerate() {
                children.push((*index, group_index as i32, item));
            }
        }
        for chunk in children.chunks(1000) {
            let mut q=QueryBuilder::<Postgres>::new("INSERT INTO b3_security_definition_attributes(snapshot_id,source_member_id,record_index,group_index,attribute_type,value) ");
            q.push_values(chunk, |mut b, (index, group_index, r)| {
                b.push_bind(snapshot)
                    .push_bind(member)
                    .push_bind(index)
                    .push_bind(group_index)
                    .push_bind(&r.attribute_type)
                    .push_bind(&r.value);
            });
            q.push(" ON CONFLICT DO NOTHING")
                .build()
                .execute(&mut *tx)
                .await?;
        }
        let mut children = Vec::new();
        for (index, r, _) in &rows {
            for (group_index, item) in r.legs.iter().enumerate() {
                children.push((*index, group_index as i32, item));
            }
        }
        for chunk in children.chunks(1000) {
            let mut q=QueryBuilder::<Postgres>::new("INSERT INTO b3_security_definition_legs(snapshot_id,source_member_id,record_index,group_index,symbol,security_id,security_id_source,security_exchange,security_type,side,ratio_quantity) ");
            q.push_values(chunk, |mut b, (index, group_index, r)| {
                b.push_bind(snapshot)
                    .push_bind(member)
                    .push_bind(index)
                    .push_bind(group_index)
                    .push_bind(&r.symbol)
                    .push_bind(&r.security_id)
                    .push_bind(&r.security_id_source)
                    .push_bind(&r.security_exchange)
                    .push_bind(&r.security_type)
                    .push_bind(&r.side)
                    .push_bind(r.ratio_quantity);
            });
            q.push(" ON CONFLICT DO NOTHING")
                .build()
                .execute(&mut *tx)
                .await?;
        }
        let mut children = Vec::new();
        for (index, r, _) in &rows {
            for (group_index, item) in r.tick_rules.iter().enumerate() {
                children.push((*index, group_index as i32, item));
            }
        }
        for chunk in children.chunks(1000) {
            let mut q=QueryBuilder::<Postgres>::new("INSERT INTO b3_security_definition_tick_rules(snapshot_id,source_member_id,record_index,group_index,start_price,end_price,increment,rule_type) ");
            q.push_values(chunk, |mut b, (index, group_index, r)| {
                b.push_bind(snapshot)
                    .push_bind(member)
                    .push_bind(index)
                    .push_bind(group_index)
                    .push_bind(r.start_price)
                    .push_bind(r.end_price)
                    .push_bind(r.increment)
                    .push_bind(&r.rule_type);
            });
            q.push(" ON CONFLICT DO NOTHING")
                .build()
                .execute(&mut *tx)
                .await?;
        }
        let mut children = Vec::new();
        for (index, r, _) in &rows {
            for (group_index, item) in r.lot_rules.iter().enumerate() {
                children.push((*index, group_index as i32, item));
            }
        }
        for chunk in children.chunks(1000) {
            let mut q=QueryBuilder::<Postgres>::new("INSERT INTO b3_security_definition_lot_rules(snapshot_id,source_member_id,record_index,group_index,lot_type,min_lot_size) ");
            q.push_values(chunk, |mut b, (index, group_index, r)| {
                b.push_bind(snapshot)
                    .push_bind(member)
                    .push_bind(index)
                    .push_bind(group_index)
                    .push_bind(&r.lot_type)
                    .push_bind(r.min_lot_size);
            });
            q.push(" ON CONFLICT DO NOTHING")
                .build()
                .execute(&mut *tx)
                .await?;
        }
        // Hash the complete typed record, including source-specific groups; no generic JSON record store.
        sqlx::query("UPDATE b3_registry_records r SET business_hash=s.business_hash FROM b3_security_definitions s WHERE r.snapshot_id=$1 AND r.source_member_id=$2 AND s.snapshot_id=r.snapshot_id AND s.source_member_id=r.source_member_id AND s.record_index=r.record_index").bind(snapshot).bind(member).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
}
