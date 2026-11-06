//! Registry parsing is bounded and source-specific; no ticker-derived identity is created.
use crate::{
    db::{
        enums::SourceFileType,
        history::{HistoryRepository, LocatedRejection},
    },
    error::{AppError, Result},
    parsers::{
        bvbg_028::Bvbg028Row,
        registry_stream::{parse_bvbg028_stream, parse_tradable_csv_stream, RegistryEvent},
        security_definition::{
            parse_security_definition_csv, parse_security_definition_json, SecurityDefinitionRecord,
        },
        tradable_list::TradableSecurityRow,
    },
    services::jobs::JobClaim,
};
use std::{fs::File, io::BufReader, path::PathBuf};
use uuid::Uuid;

enum Record {
    Instrument(u64, Box<Bvbg028Row>),
    Tradable(u64, TradableSecurityRow),
    Definition(u64, Box<SecurityDefinitionRecord>),
    Rejected(LocatedRejection),
}

fn event<T>(event: RegistryEvent<T>, valid: impl FnOnce(u64, T) -> Record) -> Record {
    match event {
        RegistryEvent::Valid {
            row, record_index, ..
        } => valid(record_index, row),
        RegistryEvent::Rejected {
            raw,
            reason,
            record_index,
            byte_offset,
        } => Record::Rejected(LocatedRejection {
            record_index: record_index as i64,
            byte_offset: Some(byte_offset as i64),
            raw_payload: hex::encode(raw),
            reason_code: "INVALID_REGISTRY_RECORD".into(),
            message: reason,
        }),
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn ingest_member(
    history: &HistoryRepository,
    claim: &JobClaim,
    snapshot: Uuid,
    source: SourceFileType,
    member: &str,
    path: PathBuf,
    profile: &str,
    reference_date: chrono::NaiveDate,
    max_records: usize,
    max_bytes: usize,
) -> Result<()> {
    let (sender, mut receiver) = tokio::sync::mpsc::channel(8);
    let profile = profile.to_owned();
    let parser = tokio::task::spawn_blocking(move || {
        let reader = BufReader::new(File::open(path)?);
        if source == SourceFileType::Bvbg028InstrumentsFile {
            parse_bvbg028_stream(reader, |record| {
                sender
                    .blocking_send(event(record, |index, row| {
                        Record::Instrument(index, Box::new(row))
                    }))
                    .map_err(|_| cancelled())
            })
        } else if profile == "tradable_b3_security_definition_v101_csv" {
            parse_security_definition_csv(reader, reference_date, |record| {
                sender
                    .blocking_send(event(record, |index, row| {
                        Record::Definition(index, Box::new(row))
                    }))
                    .map_err(|_| cancelled())
            })
        } else if profile == "tradable_b3_security_definition_v101_json" {
            parse_security_definition_json(reader, reference_date, |record| {
                sender
                    .blocking_send(event(record, |index, row| {
                        Record::Definition(index, Box::new(row))
                    }))
                    .map_err(|_| cancelled())
            })
        } else {
            parse_tradable_csv_stream(reader, b';', |record| {
                sender
                    .blocking_send(event(record, Record::Tradable))
                    .map_err(|_| cancelled())
            })
        }
    });
    let mut instruments = Vec::new();
    let mut tradables = Vec::new();
    let mut definitions = Vec::new();
    let mut rejections = Vec::new();
    let mut bytes = 0;
    while let Some(record) = receiver.recv().await {
        let record_bytes = match &record {
            Record::Instrument(_, row) => instrument_size(row),
            Record::Tradable(_, row) => {
                std::mem::size_of_val(row)
                    + [
                        &row.security_id,
                        &row.ticker_symbol,
                        &row.isin,
                        &row.market_code,
                        &row.segment_code,
                    ]
                    .into_iter()
                    .map(|v| v.as_ref().map_or(0, String::len))
                    .sum::<usize>()
            }
            Record::Definition(_, row) => serde_json::to_vec(row)?.len(),
            Record::Rejected(row) => {
                row.raw_payload.len() + row.message.len() + std::mem::size_of_val(row)
            }
        };
        if record_bytes > max_bytes {
            return Err(AppError::Validation(
                "Registro excede orçamento do lote; ampliar configuração antes de retomar.".into(),
            ));
        }
        if bytes + record_bytes > max_bytes {
            flush(
                history,
                claim,
                snapshot,
                member,
                &mut instruments,
                &mut tradables,
                &mut definitions,
                &mut rejections,
            )
            .await?;
            bytes = 0;
        }
        match record {
            Record::Instrument(index, row) => {
                instruments.push((index as i64, *row));
            }
            Record::Tradable(index, row) => {
                tradables.push((index as i64, row));
            }
            Record::Definition(index, row) => {
                definitions.push((index as i64, *row));
            }
            Record::Rejected(record) => {
                rejections.push(record);
            }
        }
        bytes += record_bytes;
        if instruments.len() + tradables.len() + definitions.len() + rejections.len()
            >= max_records.min(128)
            || bytes >= max_bytes
        {
            flush(
                history,
                claim,
                snapshot,
                member,
                &mut instruments,
                &mut tradables,
                &mut definitions,
                &mut rejections,
            )
            .await?;
            bytes = 0;
        }
    }
    flush(
        history,
        claim,
        snapshot,
        member,
        &mut instruments,
        &mut tradables,
        &mut definitions,
        &mut rejections,
    )
    .await?;
    match parser.await.map_err(|_| cancelled())? {
        Ok(_) => {
            history
                .complete_registry_member(claim, snapshot, member, true)
                .await
        }
        Err(error) => {
            history
                .complete_registry_member(claim, snapshot, member, false)
                .await?;
            Err(error)
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn flush(
    history: &HistoryRepository,
    claim: &JobClaim,
    snapshot: Uuid,
    member: &str,
    instruments: &mut Vec<(i64, Bvbg028Row)>,
    tradables: &mut Vec<(i64, TradableSecurityRow)>,
    definitions: &mut Vec<(i64, SecurityDefinitionRecord)>,
    rejections: &mut Vec<LocatedRejection>,
) -> Result<()> {
    if !definitions.is_empty() {
        history
            .stage_security_definition_projection_batch(
                claim,
                snapshot,
                member,
                &definitions
                    .iter()
                    .map(|(index, record)| {
                        record
                            .to_instrument_row()
                            .map(|projection| (*index, projection))
                    })
                    .collect::<Result<Vec<_>>>()?,
            )
            .await?;
        history
            .stage_security_definition_groups(claim, snapshot, member, definitions)
            .await?;
        definitions.clear();
    }
    if !instruments.is_empty() {
        history
            .stage_registry_batch(claim, snapshot, member, instruments)
            .await?;
        instruments.clear();
    }
    if !tradables.is_empty() {
        history
            .stage_tradable_batch(claim, snapshot, member, tradables)
            .await?;
        tradables.clear();
    }
    if !rejections.is_empty() {
        history
            .quarantine_registry_batch(claim, snapshot, member, rejections)
            .await?;
        rejections.clear();
    }
    Ok(())
}

fn instrument_size(row: &Bvbg028Row) -> usize {
    std::mem::size_of_val(row)
        + row.security_id.len()
        + [
            &row.security_source,
            &row.market_identifier_code,
            &row.asset,
            &row.asset_description,
            &row.market_code,
            &row.market_name,
            &row.segment_code,
            &row.segment_name,
            &row.ticker_symbol,
            &row.isin,
            &row.cfi_code,
            &row.security_category,
            &row.instrument_status,
            &row.instrument_type,
        ]
        .into_iter()
        .map(|value| value.as_ref().map_or(0, String::len))
        .sum::<usize>()
}
fn cancelled() -> AppError {
    AppError::Validation("Interpretação cadastral interrompida.".into())
}
