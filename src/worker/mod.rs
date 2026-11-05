use chrono::{Datelike, Local};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

use crate::config::B3Settings;
use crate::db::enums::SourceFileType;
use crate::services::sync_service::B3SyncService;

pub struct B3Worker {
    settings: Arc<B3Settings>,
    sync_service: Arc<B3SyncService>,
}

impl B3Worker {
    pub fn new(settings: Arc<B3Settings>, sync_service: Arc<B3SyncService>) -> Self {
        Self {
            settings,
            sync_service,
        }
    }

    pub async fn run_loop(&self) {
        info!("Starting Tucano B3 background worker daemon...");

        if !self.settings.b3_sync_enabled {
            info!("B3_SYNC_ENABLED is false; worker running in passive standby mode.");
        }

        loop {
            if self.settings.b3_sync_enabled {
                if let Err(e) = self.run_scheduled_tasks().await {
                    error!("Error executing worker scheduled tasks: {:?}", e);
                }
            }

            // Sleep 60 seconds between polling rounds
            sleep(Duration::from_secs(60)).await;
        }
    }

    pub async fn run_scheduled_tasks(&self) -> anyhow::Result<()> {
        let today = Local::now().date_naive();
        // Skip weekends
        if today.weekday().number_from_monday() > 5 {
            return Ok(());
        }

        // Daily ingestion pipeline in specified order
        let order = [
            SourceFileType::Bvbg028InstrumentsFile,
            SourceFileType::TradableSecurityList,
            SourceFileType::Bvbg087IndexReport,
            SourceFileType::Bvbg086PriceReport,
            SourceFileType::Bvbg186EquitiesSimplifiedPriceReport,
            SourceFileType::CotahistDaily,
        ];

        for sft in order {
            match self
                .sync_service
                .run_manual_sync(
                    sft,
                    today,
                    today,
                    None,
                    None,
                    false,
                    false,
                    Some("b3_worker_cron".to_string()),
                )
                .await
            {
                Ok(outcome) => {
                    info!(
                        "Worker sync {} for {} completed: {} inserted, {} changed, {} unchanged",
                        sft, today, outcome.inserted, outcome.changed, outcome.unchanged
                    );
                }
                Err(crate::error::AppError::LockConflict(_)) => {
                    // Another instance or manual process is syncing this date
                    warn!("Worker skipped {} (lock already held by another job)", sft);
                }
                Err(e) => {
                    warn!("Worker sync {} failed: {:?}", sft, e);
                }
            }
        }

        Ok(())
    }
}
