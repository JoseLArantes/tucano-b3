//! Parse and validate a CSV batch before writing to the ledger.

use super::{validate_record, ProventosError, ProventosService};
use crate::models::proventos::ProventosCsvUploadResponse;
use chrono::Utc;
use std::collections::HashSet;

const REQUIRED_COLUMNS: [&str; 3] = ["ticker", "tipo", "valor"];
const OPTIONAL_COLUMNS: [&str; 3] = ["data_com", "data_pagamento", "obs"];
const MAX_ROWS: usize = 10_000;

impl ProventosService {
    /// Validate each row, then commit new records as one atomic batch.
    pub fn import_csv(
        &self,
        contents: &[u8],
    ) -> Result<ProventosCsvUploadResponse, ProventosError> {
        let mut reader = csv::Reader::from_reader(contents);
        let headers = reader.headers().map_err(csv_error)?.clone();
        let mut seen = HashSet::new();
        for column in &headers {
            if !REQUIRED_COLUMNS.contains(&column) && !OPTIONAL_COLUMNS.contains(&column) {
                return Err(ProventosError::Invalid(format!(
                    "coluna desconhecida no cabeçalho CSV: {column}"
                )));
            }
            if !seen.insert(column) {
                return Err(ProventosError::Invalid(format!(
                    "coluna repetida no cabeçalho CSV: {column}"
                )));
            }
        }
        for required in REQUIRED_COLUMNS {
            if !seen.contains(required) {
                return Err(ProventosError::Invalid(format!(
                    "coluna obrigatória ausente no cabeçalho CSV: {required}"
                )));
            }
        }

        let mut records = Vec::new();
        for row in reader.records() {
            if records.len() == MAX_ROWS {
                return Err(ProventosError::Invalid(format!(
                    "o arquivo CSV aceita no máximo {MAX_ROWS} registros"
                )));
            }
            let row = row.map_err(csv_error)?;
            let request = row.deserialize(Some(&headers)).map_err(csv_error)?;
            let record = validate_record(request).map_err(|error| match error {
                ProventosError::Invalid(message) => ProventosError::Invalid(format!(
                    "linha {} do CSV: {message}",
                    row.position().map_or(0, csv::Position::line),
                )),
                error => error,
            })?;
            records.push(record);
        }
        if records.is_empty() {
            return Err(ProventosError::Invalid(
                "o arquivo CSV deve conter pelo menos um registro".into(),
            ));
        }

        let recebidos = records.len();
        let inseridos = self
            .ledger
            .import_proventos_csv(&records, &Utc::now().to_rfc3339())?;
        Ok(ProventosCsvUploadResponse {
            recebidos,
            inseridos,
            ignorados: recebidos - inseridos,
        })
    }
}

fn csv_error(error: csv::Error) -> ProventosError {
    ProventosError::Invalid(format!("CSV inválido: {error}"))
}
