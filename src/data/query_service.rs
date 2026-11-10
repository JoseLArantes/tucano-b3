//! Query service for tucano-b3.
//!
//! Wraps DataFusion to provide a typed query interface over Parquet data.
//! This is the primary data access layer used by all API endpoints.

use crate::data::reader::DiscoveredFiles;
use crate::data::schema::{cotahist_schema, ibov_schema};
use arrow::array::RecordBatch;
use datafusion::datasource::file_format::parquet::ParquetFormat;
use datafusion::datasource::listing::{
    ListingOptions, ListingTable, ListingTableConfig, ListingTableUrl,
};
use datafusion::prelude::*;
use std::sync::Arc;
use thiserror::Error;

/// Errors during query execution.
#[derive(Error, Debug)]
pub enum QueryError {
    #[error("DataFusion error: {0}")]
    DataFusion(#[from] datafusion::error::DataFusionError),

    #[error("Table not found: {table}")]
    TableNotFound { table: String },

    #[error("No data files loaded")]
    NoDataLoaded,
}

/// The query service provides SQL query execution over Parquet data.
///
/// Thread-safe: can be shared across multiple handlers via Arc.
pub struct QueryService {
    ctx: SessionContext,
    files: DiscoveredFiles,
}

impl QueryService {
    /// Create a new QueryService from discovered files.
    ///
    /// Registers all Parquet files as DataFusion tables:
    /// - `cotahist`: union of all COTAHIST Parquet files
    /// - `ibov`: single IBOV historical Parquet file
    pub fn new(files: DiscoveredFiles) -> Result<Self, QueryError> {
        let ctx = SessionContext::new();

        // Register COTAHIST tables
        if !files.cotahist_files.is_empty() {
            let listing_options = ListingOptions::new(Arc::new(ParquetFormat::default()))
                .with_file_extension(".parquet");

            // Register each file or parent directory as listing table
            for file_info in &files.cotahist_files {
                let path_str = file_info.path.to_string_lossy().to_string();
                let table_url = ListingTableUrl::parse(&path_str).map_err(|e| {
                    QueryError::DataFusion(datafusion::error::DataFusionError::External(Box::new(
                        e,
                    )))
                })?;

                let config = ListingTableConfig::new(table_url)
                    .with_listing_options(listing_options.clone())
                    .with_schema(cotahist_schema());

                let table = ListingTable::try_new(config)?;
                // If single table or union
                let _ = table;
            }

            // In DataFusion, to register multiple files under one table name 'cotahist',
            // if they share a common parent directory, we can register the directory or
            // create listing table with parent path. If files are in data/processed/cotahist,
            // the parent directory is used.
            let parent_dir = files.cotahist_files[0]
                .path
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| files.cotahist_files[0].path.to_string_lossy().to_string());

            let table_url = ListingTableUrl::parse(&parent_dir).map_err(|e| {
                QueryError::DataFusion(datafusion::error::DataFusionError::External(Box::new(e)))
            })?;

            let config = ListingTableConfig::new(table_url)
                .with_listing_options(listing_options)
                .with_schema(cotahist_schema());

            let table = ListingTable::try_new(config)?;
            ctx.register_table("cotahist", Arc::new(table))?;

            tracing::info!(
                files = files.cotahist_files.len(),
                "Tabela 'cotahist' registrada"
            );
        }

        // Register IBOV table
        if let Some(ref ibov_info) = files.ibov_file {
            let listing_options = ListingOptions::new(Arc::new(ParquetFormat::default()))
                .with_file_extension(".parquet");

            let ibov_path_str = ibov_info.path.to_string_lossy().to_string();
            let table_url = ListingTableUrl::parse(&ibov_path_str).map_err(|e| {
                QueryError::DataFusion(datafusion::error::DataFusionError::External(Box::new(e)))
            })?;

            let config = ListingTableConfig::new(table_url)
                .with_listing_options(listing_options)
                .with_schema(ibov_schema());

            let table = ListingTable::try_new(config)?;
            ctx.register_table("ibov", Arc::new(table))?;

            tracing::info!("Tabela 'ibov' registrada");
        }

        Ok(Self { ctx, files })
    }

    /// Execute a SQL query and return the results as RecordBatches.
    ///
    /// This is the primary query method used by all API endpoints.
    pub async fn execute_sql(&self, sql: &str) -> Result<Vec<RecordBatch>, QueryError> {
        let df = self.ctx.sql(sql).await?;
        let batches = df.collect().await?;
        Ok(batches)
    }

    /// Check if a table is registered.
    pub fn has_table(&self, table_name: &str) -> bool {
        self.ctx.table_exist(table_name).unwrap_or(false)
    }

    /// Get the number of COTAHIST files loaded.
    pub fn cotahist_file_count(&self) -> usize {
        self.files.cotahist_files.len()
    }

    /// Get the total number of rows across all COTAHIST files.
    pub fn total_cotahist_rows(&self) -> usize {
        self.files.cotahist_files.iter().map(|f| f.row_count).sum()
    }

    /// Get the number of IBOV rows.
    pub fn ibov_row_count(&self) -> usize {
        self.files.ibov_file.as_ref().map_or(0, |f| f.row_count)
    }

    /// Get the data date range for COTAHIST.
    pub async fn cotahist_date_range(&self) -> Result<Option<(String, String)>, QueryError> {
        if !self.has_table("cotahist") {
            return Ok(None);
        }

        let sql = "SELECT MIN(date) as min_date, MAX(date) as max_date FROM cotahist";
        let batches = self.execute_sql(sql).await?;

        if batches.is_empty() || batches[0].num_rows() == 0 {
            return Ok(None);
        }

        // Extract min/max dates from the result
        // Implementation depends on Arrow date format
        Ok(None) // Placeholder — will be fully implemented in M2
    }

    /// Get the data date range for IBOV.
    pub async fn ibov_date_range(&self) -> Result<Option<(String, String)>, QueryError> {
        if !self.has_table("ibov") {
            return Ok(None);
        }

        let sql = "SELECT MIN(date) as min_date, MAX(date) as max_date FROM ibov";
        let batches = self.execute_sql(sql).await?;

        if batches.is_empty() || batches[0].num_rows() == 0 {
            return Ok(None);
        }

        Ok(None) // Placeholder — will be fully implemented in M3
    }
}
