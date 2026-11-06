//! Bounded registry profiles. Official production compatibility is homologated per source catalog.
use super::{bvbg_028::Bvbg028Row, canonical::*, tradable_list::TradableSecurityRow};
use crate::error::{AppError, Result};
use quick_xml::{events::Event, name::ResolveResult, NsReader, Writer};
use std::io::{self, BufRead, Read};

const MAX_RECORD_BYTES: usize = 1024 * 1024;
const MAX_TOKEN_BYTES: usize = 64 * 1024;

#[derive(Debug)]
pub enum RegistryEvent<T> {
    Valid {
        row: T,
        record_index: u64,
        byte_offset: u64,
    },
    Rejected {
        record_index: u64,
        byte_offset: u64,
        raw: Vec<u8>,
        reason: String,
    },
}

#[derive(Debug, Default)]
pub struct RegistrySummary {
    pub records: u64,
    pub valid_records: u64,
    pub rejected_records: u64,
}

/// Limits each quick-xml read before it can buffer an unbounded tag/text/CDATA node.
struct TokenLimited<R> {
    inner: R,
    count: usize,
}
impl<R: BufRead> TokenLimited<R> {
    fn new(inner: R) -> Self {
        Self { inner, count: 0 }
    }
}
impl<R: BufRead> Read for TokenLimited<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let bytes = self.fill_buf()?;
        let n = bytes.len().min(out.len());
        out[..n].copy_from_slice(&bytes[..n]);
        self.consume(n);
        Ok(n)
    }
}
impl<R: BufRead> BufRead for TokenLimited<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        let bytes = self.inner.fill_buf()?;
        let permitted = bytes.len().min(MAX_TOKEN_BYTES.saturating_sub(self.count));
        if permitted == 0 && !bytes.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Token XML excede limite.",
            ));
        }
        Ok(&bytes[..permitted])
    }
    fn consume(&mut self, n: usize) {
        self.count = self.count.saturating_add(n);
        self.inner.consume(n);
    }
}

pub fn parse_bvbg028_stream<R: BufRead, F: FnMut(RegistryEvent<Bvbg028Row>) -> Result<()>>(
    reader: R,
    mut on_record: F,
) -> Result<RegistrySummary> {
    let mut reader = NsReader::from_reader(TokenLimited::new(reader));
    reader.config_mut().check_end_names = true;
    let mut buffer = Vec::new();
    let mut stack: Vec<(String, Vec<u8>, String)> = Vec::new();
    let mut record_depth = None;
    let mut namespace = Vec::new();
    let mut raw = Writer::new(Vec::new());
    let mut row = Bvbg028Row::default();
    let mut error = None;
    let mut summary = RegistrySummary::default();
    let mut start = 0;
    loop {
        reader.get_mut().count = 0;
        let before = reader.buffer_position();
        let (resolved, event) = reader.read_resolved_event_into(&mut buffer)?;
        let ns = match resolved {
            ResolveResult::Bound(ns) => ns.as_ref().to_vec(),
            ResolveResult::Unbound => Vec::new(),
            ResolveResult::Unknown(_) => return Err(layout("Namespace XML não declarado.")),
        };
        if record_depth.is_some() {
            raw.write_event(event.clone())?;
            if raw.get_ref().len() > MAX_RECORD_BYTES {
                return Err(layout("Registro XML excede limite."));
            }
        }
        match event {
            Event::Start(e) => {
                if stack.len() >= 64 {
                    return Err(layout("Profundidade XML excede limite."));
                }
                let local = String::from_utf8_lossy(e.local_name().as_ref()).to_ascii_lowercase();
                if local == "instrm" || local == "instrument" {
                    if !ns.is_empty() && !ns.starts_with(b"urn:bvmf.") {
                        return Err(layout("Namespace de cadastro não suportado."));
                    }
                    if record_depth.is_some() {
                        return Err(layout("Registros de instrumento aninhados."));
                    }
                    record_depth = Some(stack.len());
                    namespace = ns.clone();
                    row = Bvbg028Row::default();
                    error = None;
                    start = before;
                    raw = Writer::new(Vec::new());
                    raw.write_event(Event::Start(e.clone()))?;
                }
                stack.push((local, ns, String::new()));
            }
            Event::Text(e) => {
                if let Some(node) = stack.last_mut().filter(|_| record_depth.is_some()) {
                    node.2.push_str(&e.unescape()?);
                    if node.2.len() > MAX_TOKEN_BYTES {
                        return Err(layout("Texto XML excede limite."));
                    }
                }
            }
            Event::CData(e) => {
                if let Some(node) = stack.last_mut().filter(|_| record_depth.is_some()) {
                    node.2.push_str(
                        std::str::from_utf8(e.as_ref())
                            .map_err(|_| layout("Texto XML com encoding não suportado."))?,
                    );
                    if node.2.len() > MAX_TOKEN_BYTES {
                        return Err(layout("Texto XML excede limite."));
                    }
                }
            }
            Event::End(_) => {
                if let Some(depth) = record_depth {
                    if stack.len() == depth + 1 {
                        summary.records += 1;
                        if row.security_id.trim().is_empty() {
                            error = Some(
                                "Instrumento sem security_id publicado; ticker não é identidade."
                                    .into(),
                            );
                        }
                        if let Some(reason) = error.take() {
                            on_record(RegistryEvent::Rejected {
                                record_index: summary.records,
                                byte_offset: start,
                                raw: raw.into_inner(),
                                reason,
                            })?;
                            summary.rejected_records += 1;
                        } else {
                            on_record(RegistryEvent::Valid {
                                row: row.clone(),
                                record_index: summary.records,
                                byte_offset: start,
                            })?;
                            summary.valid_records += 1;
                        }
                        raw = Writer::new(Vec::new());
                        record_depth = None;
                    } else if let Some(node) = stack.last() {
                        if node.1 == namespace && stack[depth..].iter().all(|n| n.1 == namespace) {
                            let path: Vec<_> =
                                stack[depth + 1..].iter().map(|n| n.0.as_str()).collect();
                            if let Err(e) = assign_xml_field(&mut row, &path, &node.2) {
                                if error.is_none() {
                                    error = Some(e.to_string());
                                }
                            }
                        }
                    }
                }
                if stack.pop().is_none() {
                    return Err(layout("Estrutura XML inválida."));
                }
            }
            Event::DocType(_) => {
                return Err(layout("DTD e entidades externas não são suportados."))
            }
            Event::Eof => {
                if !stack.is_empty() || record_depth.is_some() {
                    return Err(layout("XML truncado."));
                }
                break;
            }
            _ => {}
        }
        buffer.clear();
    }
    if summary.records == 0 {
        return Err(layout("Nenhum registro do perfil BVBG.028 suportado."));
    }
    Ok(summary)
}

fn assign_xml_field(row: &mut Bvbg028Row, path: &[&str], text: &str) -> Result<()> {
    let Some(tag) = path.last() else {
        return Ok(());
    };
    if (path.len() == 1 && matches!(*tag, "securityid" | "security_id" | "sctyid"))
        || path.ends_with(&["fininstrmid", "othrid", "id"])
    {
        if let Some(id) = canonicalize_string(Some(text)) {
            row.security_id = id;
        }
        return Ok(());
    }
    match *tag {
        "securitysource" | "security_source" => {
            row.security_source = canonicalize_string(Some(text))
        }
        "marketidentifiercode" | "mic" | "mktidrcd" => {
            row.market_identifier_code = canonicalize_string(Some(text))
        }
        "asset" | "asst" => row.asset = canonicalize_string(Some(text)),
        "assetdescription" | "asset_description" | "asstdesc" | "desc" | "description" => {
            row.asset_description = canonicalize_string(Some(text))
        }
        "marketcode" | "market_code" | "mkt" => row.market_code = canonicalize_string(Some(text)),
        "marketname" | "market_name" | "mktdesc" => {
            row.market_name = canonicalize_string(Some(text))
        }
        "segmentcode" | "segment_code" | "sgmt" => {
            row.segment_code = canonicalize_string(Some(text))
        }
        "segmentname" | "segment_name" | "sgmtdesc" => {
            row.segment_name = canonicalize_string(Some(text))
        }
        "tickersymbol" | "ticker_symbol" | "ticker" | "tckrsymb" | "plrtytckrsymb" => {
            row.ticker_symbol = canonicalize_ticker(Some(text))
        }
        "isin" => row.isin = canonicalize_isin(Some(text)),
        "cficode" | "cfi" | "cficd" => row.cfi_code = canonicalize_string(Some(text)),
        "securitycategory" | "security_category" | "sctyctgy" => {
            row.security_category = canonicalize_string(Some(text))
        }
        "instrumentstatus" | "instrument_status" | "status" | "instrmsts" => {
            row.instrument_status = canonicalize_string(Some(text))
        }
        "maturitydate" | "maturity_date" | "mtrtydt" | "xprtndt" => {
            row.maturity_date = canonicalize_date(Some(text))?
        }
        "referencedate" | "reference_date" => row.reference_date = canonicalize_date(Some(text))?,
        _ => {}
    }
    Ok(())
}

/// UTF-8, single-physical-line CSV profile; delimiter is declared, not guessed.
pub fn parse_tradable_csv_stream<
    R: BufRead,
    F: FnMut(RegistryEvent<TradableSecurityRow>) -> Result<()>,
>(
    mut reader: R,
    delimiter: u8,
    mut on_record: F,
) -> Result<RegistrySummary> {
    if !b";,\t|".contains(&delimiter) {
        return Err(layout("Delimitador CSV não suportado."));
    }
    let mut buffer = Vec::new();
    let mut offset = 0u64;
    read_csv_line(&mut reader, &mut buffer)?;
    offset += buffer.len() as u64;
    let header = std::str::from_utf8(&buffer)
        .map_err(|_| layout("Encoding CSV deve ser UTF-8 neste perfil."))?
        .trim_start_matches('\u{feff}');
    if header.trim_start().starts_with(['{', '[']) {
        return Err(layout("Perfil JSON do cadastro ainda não homologado."));
    }
    let columns = csv_record(header.as_bytes(), delimiter)?;
    let columns: Vec<_> = columns
        .iter()
        .map(|v| v.trim().to_ascii_lowercase().replace('_', ""))
        .collect();
    let allowed = [
        "referencedate",
        "date",
        "securityid",
        "id",
        "tickersymbol",
        "ticker",
        "isin",
        "marketcode",
        "mkt",
        "segmentcode",
        "sgmt",
        "tradableindicator",
        "tradable",
        "available",
        "tradingstartdate",
        "startdate",
        "tradingenddate",
        "enddate",
    ];
    let mut unique = std::collections::HashSet::new();
    if columns.len() > 128
        || columns
            .iter()
            .any(|c| !allowed.contains(&c.as_str()) || !unique.insert(c.clone()))
        || !columns.iter().any(|c| c == "referencedate" || c == "date")
        || !columns.iter().any(|c| c == "securityid" || c == "id")
    {
        return Err(layout("Headers CSV fora do perfil de cadastro suportado."));
    }
    let mut summary = RegistrySummary::default();
    loop {
        buffer.clear();
        let start = offset;
        let n = read_csv_line(&mut reader, &mut buffer)?;
        if n == 0 {
            break;
        }
        offset += n as u64;
        summary.records += 1;
        let result = (|| -> Result<TradableSecurityRow> {
            let record = csv_record(&buffer, delimiter)?;
            if record.len() != columns.len() {
                return Err(layout("Quantidade de campos CSV divergente."));
            }
            let mut row = TradableSecurityRow::default();
            let mut dated = false;
            for (name, value) in columns.iter().zip(record.iter()) {
                match name.as_str() {
                    "referencedate" | "date" => {
                        row.reference_date = canonicalize_date(Some(value))?
                            .ok_or_else(|| layout("Data de referência CSV ausente."))?;
                        dated = true;
                    }
                    "securityid" | "id" => row.security_id = canonicalize_string(Some(value)),
                    "tickersymbol" | "ticker" => {
                        row.ticker_symbol = canonicalize_ticker(Some(value))
                    }
                    "isin" => row.isin = canonicalize_isin(Some(value)),
                    "marketcode" | "mkt" => row.market_code = canonicalize_string(Some(value)),
                    "segmentcode" | "sgmt" => row.segment_code = canonicalize_string(Some(value)),
                    "tradingstartdate" | "startdate" => {
                        row.trading_start_date = canonicalize_date(Some(value))?
                    }
                    "tradingenddate" | "enddate" => {
                        row.trading_end_date = canonicalize_date(Some(value))?
                    }
                    "tradableindicator" | "tradable" | "available" => {
                        row.tradable_indicator = match value.trim().to_ascii_uppercase().as_str() {
                            "" => None,
                            "1" | "Y" | "YES" | "S" | "SIM" | "TRUE" => Some(true),
                            "0" | "N" | "NO" | "NAO" | "FALSE" => Some(false),
                            _ => return Err(layout("Indicador de negociação CSV inválido.")),
                        }
                    }
                    _ => {}
                }
            }
            if !dated || row.security_id.is_none() {
                return Err(layout("Registro cadastral sem ID publicado ou referência."));
            }
            Ok(row)
        })();
        match result {
            Ok(row) => {
                on_record(RegistryEvent::Valid {
                    row,
                    record_index: summary.records,
                    byte_offset: start,
                })?;
                summary.valid_records += 1;
            }
            Err(e) => {
                on_record(RegistryEvent::Rejected {
                    record_index: summary.records,
                    byte_offset: start,
                    raw: buffer.clone(),
                    reason: e.to_string(),
                })?;
                summary.rejected_records += 1;
            }
        }
    }
    if summary.records == 0 {
        return Err(layout("Nenhum registro do perfil CSV suportado."));
    }
    Ok(summary)
}

fn read_csv_line<R: BufRead>(reader: &mut R, buffer: &mut Vec<u8>) -> Result<usize> {
    let n = reader
        .take((MAX_RECORD_BYTES + 1) as u64)
        .read_until(b'\n', buffer)?;
    if n > MAX_RECORD_BYTES {
        return Err(layout("Registro CSV excede limite."));
    }
    Ok(n)
}
fn csv_record(bytes: &[u8], delimiter: u8) -> Result<csv::StringRecord> {
    if bytes.iter().filter(|b| **b == b'"').count() % 2 != 0 {
        return Err(layout(
            "Aspas CSV incompletas; perfil não permite campos multilinha.",
        ));
    }
    let mut csv = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(false)
        .from_reader(bytes);
    let row = csv
        .records()
        .next()
        .transpose()?
        .ok_or_else(|| layout("Linha CSV vazia."))?;
    if csv.records().next().is_some() {
        return Err(layout("Registro CSV contém linhas adicionais."));
    }
    Ok(row)
}
fn layout(message: &str) -> AppError {
    AppError::Layout(message.into())
}
