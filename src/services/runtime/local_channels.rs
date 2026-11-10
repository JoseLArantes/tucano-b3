//! Explicit, narrowly scoped registration of manually discovered local COTAHIST files.
use super::discovery_locations::DiscoveryLocation;
use super::{discovery_locations, RuntimeSettings};
use crate::{
    db::enums::SourceFileType,
    error::{AppError, Result},
};
use chrono::NaiveDate;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::{fs, path::PathBuf, str::FromStr};

/// Registered local channel details returned to an administrative operator.
#[derive(Debug, Clone)]
pub struct LocalChannelRegistration {
    pub channel_id: String,
    pub source_file_type: String,
    pub source_locator: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub already_registered: bool,
}

/// Registers only a candidate from the latest manual discovery scan and an imports folder.
/// The caller must require an explicit internal-development attestation before calling this.
pub async fn register_candidate(
    pool: &PgPool,
    settings: &RuntimeSettings,
    source_locator: &str,
) -> Result<LocalChannelRegistration> {
    let (folder_index, relative) = parse_locator(source_locator)?;
    let locations = discovery_locations::parse_locations(
        &settings.discovery_locations,
        &settings.allowed_hosts,
    )?;
    let Some(DiscoveryLocation::Folder(configured_root)) = locations.get(folder_index - 1) else {
        return Err(validation(
            "O candidato não pertence a uma pasta local configurada.",
        ));
    };

    let imports_root = settings.raw_root.join("imports");
    let expected_root = imports_root
        .canonicalize()
        .map_err(|_| validation("A pasta local de importação não está disponível."))?;
    let candidate_root = configured_root
        .canonicalize()
        .map_err(|_| validation("A pasta de descoberta não está disponível."))?;
    if candidate_root != expected_root {
        return Err(validation(
            "Importação local exige que a pasta descoberta seja B3_RAW_DATA_DIR/imports.",
        ));
    }

    let location_label = format!("folder-{folder_index}");
    let candidate = sqlx::query(
        "SELECT c.check_id,c.source_file_type,c.content_length,l.checked_at FROM b3_discovery_location_checks l JOIN b3_discovery_location_candidates c ON c.check_id=l.id WHERE l.id=(SELECT id FROM b3_discovery_location_checks WHERE location_kind='folder' AND location_label=$1 AND scheduled=FALSE ORDER BY checked_at DESC,id DESC LIMIT 1) AND c.source_locator=$2 AND l.outcome='available'",
    )
    .bind(&location_label)
    .bind(source_locator)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Candidato ausente na última descoberta manual.".into()))?;
    let checked_at: chrono::DateTime<chrono::Utc> = candidate.get("checked_at");
    if chrono::Utc::now() - checked_at > chrono::Duration::hours(24) {
        return Err(validation(
            "A descoberta expirou. Execute uma nova varredura.",
        ));
    }

    let source_file_type: String = candidate.get("source_file_type");
    let source = SourceFileType::from_str(&source_file_type)
        .map_err(|_| validation("A família do candidato não é suportada."))?;
    if !source.supports_record_date_cuts() {
        return Err(validation(
            "A família não oferece corte por data de negócio.",
        ));
    }
    let (period_start, period_end) = parse_period(source, &relative)?;

    let full_path = expected_root.join(&relative);
    reject_symlink_components(&expected_root, &relative)?;
    let canonical_path = full_path
        .canonicalize()
        .map_err(|_| validation("O arquivo descoberto não está mais disponível."))?;
    if !canonical_path.starts_with(&expected_root) || !canonical_path.is_file() {
        return Err(validation(
            "O arquivo está fora da pasta de importação permitida.",
        ));
    }
    let metadata = fs::metadata(&canonical_path)?;
    let observed_size: Option<i64> = candidate.get("content_length");
    if observed_size.is_some_and(|size| size != i64::try_from(metadata.len()).unwrap_or(i64::MAX)) {
        return Err(validation(
            "O arquivo mudou desde a descoberta. Execute outra varredura.",
        ));
    }

    let local_relative_path = relative.to_string_lossy().replace('\\', "/");
    let channel_id = channel_id(source.as_str(), source_locator);
    let publication_key = format!("{}-{}", source.as_str().to_ascii_lowercase(), period_start);
    let evidence_reference = format!(
        "maintainer-attestation://local-development; source_locator={source_locator}; scope=acquisition,raw_retention,processing; external_delivery=false"
    );
    let mut tx = pool.begin().await?;
    let existing = sqlx::query("SELECT source_file_type,local_relative_path,period_start,period_end FROM b3_source_channels WHERE id=$1 FOR UPDATE")
        .bind(&channel_id)
        .fetch_optional(&mut *tx)
        .await?;
    if let Some(row) = &existing {
        if row.get::<String, _>("source_file_type") != source.as_str()
            || row
                .get::<Option<String>, _>("local_relative_path")
                .as_deref()
                != Some(local_relative_path.as_str())
            || row.get::<NaiveDate, _>("period_start") != period_start
            || row.get::<NaiveDate, _>("period_end") != period_end
        {
            return Err(validation(
                "O canal local existente diverge do candidato descoberto.",
            ));
        }
    }
    sqlx::query("INSERT INTO b3_source_channels(id,source_file_type,acquisition_mode,local_relative_path,enabled,discovery_enabled,publication_key,format_profile,period_start,period_end) VALUES($1,$2,'local',$3,TRUE,FALSE,$4,'cotahist_latin1',$5,$6) ON CONFLICT(id) DO UPDATE SET enabled=TRUE")
        .bind(&channel_id)
        .bind(source.as_str())
        .bind(&local_relative_path)
        .bind(&publication_key)
        .bind(period_start)
        .bind(period_end)
        .execute(&mut *tx)
        .await?;
    for purpose in ["acquisition", "raw_retention", "processing"] {
        sqlx::query("INSERT INTO b3_source_eligibility(source_file_type,channel,purpose,status,zero_cost,evidence_reference,verified_at) VALUES($1,$2,$3,'eligible',TRUE,$4,NOW()) ON CONFLICT(source_file_type,channel,purpose) DO UPDATE SET status='eligible',zero_cost=TRUE,evidence_reference=EXCLUDED.evidence_reference,verified_at=EXCLUDED.verified_at")
            .bind(source.as_str())
            .bind(&channel_id)
            .bind(purpose)
            .bind(&evidence_reference)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;

    Ok(LocalChannelRegistration {
        channel_id,
        source_file_type: source.as_str().to_string(),
        source_locator: source_locator.to_string(),
        period_start,
        period_end,
        already_registered: existing.is_some(),
    })
}

fn parse_locator(locator: &str) -> Result<(usize, PathBuf)> {
    if locator.len() > 4096 {
        return Err(validation("Localizador de arquivo inválido."));
    }
    let Some(rest) = locator.strip_prefix("folder:") else {
        return Err(validation(
            "Somente candidatos de pasta local podem ser importados.",
        ));
    };
    let Some((index, relative)) = rest.split_once('/') else {
        return Err(validation("Localizador de pasta incompleto."));
    };
    let index = index
        .parse::<usize>()
        .ok()
        .filter(|value| (1..=32).contains(value))
        .ok_or_else(|| validation("Índice da pasta de descoberta inválido."))?;
    let relative = PathBuf::from(relative);
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(validation("Caminho relativo do candidato inválido."));
    }
    Ok((index, relative))
}

fn parse_period(
    source: SourceFileType,
    relative: &std::path::Path,
) -> Result<(NaiveDate, NaiveDate)> {
    let filename = relative
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| validation("Nome de arquivo COTAHIST inválido."))?;
    let (stem, extension) = filename
        .rsplit_once('.')
        .ok_or_else(|| validation("Extensão COTAHIST ausente."))?;
    if !matches!(extension.to_ascii_uppercase().as_str(), "ZIP" | "TXT") {
        return Err(validation("Extensão COTAHIST não suportada."));
    }
    let stem = stem.to_ascii_uppercase();
    let (start, end) = match source {
        SourceFileType::CotahistYearly => {
            let year = stem
                .strip_prefix("COTAHIST_A")
                .filter(|value| value.len() == 4 && value.bytes().all(|byte| byte.is_ascii_digit()))
                .and_then(|value| value.parse::<i32>().ok())
                .ok_or_else(|| validation("Ano não reconhecido no arquivo anual."))?;
            (date(year, 1, 1)?, date(year, 12, 31)?)
        }
        SourceFileType::CotahistMonthly => {
            let value = stem
                .strip_prefix("COTAHIST_M")
                .filter(|value| value.len() == 6 && value.bytes().all(|byte| byte.is_ascii_digit()))
                .ok_or_else(|| validation("Mês/ano não reconhecidos no arquivo mensal."))?;
            let month = value[0..2].parse::<u32>().unwrap_or(0);
            let year = value[2..6].parse::<i32>().unwrap_or(0);
            let start = date(year, month, 1)?;
            let end = last_day_of_month(year, month)
                .ok_or_else(|| validation("Data mensal inválida."))?;
            (start, end)
        }
        SourceFileType::CotahistDaily => {
            let value = stem
                .strip_prefix("COTAHIST_D")
                .filter(|value| value.len() == 8 && value.bytes().all(|byte| byte.is_ascii_digit()))
                .ok_or_else(|| validation("Dia/mês/ano não reconhecidos no arquivo diário."))?;
            let day = value[0..2].parse::<u32>().unwrap_or(0);
            let month = value[2..4].parse::<u32>().unwrap_or(0);
            let year = value[4..8].parse::<i32>().unwrap_or(0);
            let date = date(year, month, day)?;
            (date, date)
        }
        _ => {
            return Err(validation(
                "Apenas famílias COTAHIST podem ser registradas.",
            ))
        }
    };
    Ok((start, end))
}

fn date(year: i32, month: u32, day: u32) -> Result<NaiveDate> {
    NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| validation("Data do arquivo inválida."))
}

fn last_day_of_month(year: i32, month: u32) -> Option<NaiveDate> {
    let first_of_next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year.checked_add(1)?, 1, 1)?
    } else {
        NaiveDate::from_ymd_opt(year, month.checked_add(1)?, 1)?
    };
    first_of_next_month.pred_opt()
}

fn reject_symlink_components(root: &std::path::Path, relative: &std::path::Path) -> Result<()> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let std::path::Component::Normal(part) = component else {
            return Err(validation("Caminho relativo do candidato inválido."));
        };
        current.push(part);
        if fs::symlink_metadata(&current)?.file_type().is_symlink() {
            return Err(validation("Importação por link simbólico não é permitida."));
        }
    }
    Ok(())
}

fn channel_id(source: &str, locator: &str) -> String {
    let digest = Sha256::digest(format!("{source}\0{locator}").as_bytes());
    let digest = hex::encode(digest);
    format!("local-{}", &digest[..24])
}

fn validation(message: &str) -> AppError {
    AppError::Validation(message.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_safe_local_folder_locators() {
        assert_eq!(
            parse_locator("folder:1/2026/COTAHIST_A2026.ZIP").unwrap().0,
            1
        );
        for locator in [
            "https://www.b3.com.br/COTAHIST_A2026.ZIP",
            "s3://bucket/COTAHIST_A2026.ZIP",
            "folder:1/../outside.zip",
            "folder:0/COTAHIST_A2026.ZIP",
            "folder:1",
        ] {
            assert!(parse_locator(locator).is_err(), "accepted {locator}");
        }
    }

    #[test]
    fn infers_inclusive_periods_from_supported_filenames() {
        assert_eq!(
            parse_period(
                SourceFileType::CotahistYearly,
                std::path::Path::new("COTAHIST_A2026.ZIP")
            )
            .unwrap(),
            (date(2026, 1, 1).unwrap(), date(2026, 12, 31).unwrap())
        );
        assert_eq!(
            parse_period(
                SourceFileType::CotahistMonthly,
                std::path::Path::new("COTAHIST_M022026.ZIP")
            )
            .unwrap(),
            (date(2026, 2, 1).unwrap(), date(2026, 2, 28).unwrap())
        );
        assert_eq!(
            parse_period(
                SourceFileType::CotahistDaily,
                std::path::Path::new("COTAHIST_D29022024.TXT")
            )
            .unwrap(),
            (date(2024, 2, 29).unwrap(), date(2024, 2, 29).unwrap())
        );
        assert!(parse_period(
            SourceFileType::CotahistDaily,
            std::path::Path::new("COTAHIST_D31022026.TXT")
        )
        .is_err());
    }
}
