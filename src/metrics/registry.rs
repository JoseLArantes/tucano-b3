//! Prometheus metrics registry.
//!
//! All metrics are registered once at startup and shared across the application.
//! Thread-safe: uses atomic counters internally.

use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounterVec, IntGauge, Opts, Registry, TextEncoder,
};

/// Application-wide Prometheus metrics.
///
/// Shared via Arc across all middleware and handlers.
/// All metrics use the `tucano_` prefix.
pub struct AppMetrics {
    pub registry: Registry,

    // === HTTP metrics ===
    /// Total HTTP requests, labeled by method, path, status.
    pub http_requests_total: IntCounterVec,

    /// HTTP request duration in seconds, labeled by method, path.
    pub http_request_duration_seconds: HistogramVec,

    /// HTTP requests currently in flight.
    pub http_requests_in_flight: IntGauge,

    // === Cache metrics ===
    /// Cache hits.
    pub cache_hits_total: IntCounterVec,

    /// Cache misses.
    pub cache_misses_total: IntCounterVec,

    /// Cache evictions.
    pub cache_evictions_total: IntCounterVec,

    /// Current cache entries.
    pub cache_entries: IntGauge,

    // === Rate limit metrics ===
    /// Requests rejected by rate limiter.
    pub rate_limited_total: IntCounterVec,

    // === Data metrics ===
    /// Number of Parquet files loaded.
    pub data_files_loaded: IntGauge,

    /// Total number of COTAHIST quotes.
    pub data_cotahist_quotes_total: IntGauge,

    /// Total number of IBOV data points.
    pub data_ibov_rows_total: IntGauge,

    /// Number of tickers in registry.
    pub data_tickers_total: IntGauge,

    // === Service metrics ===
    /// Service uptime in seconds.
    pub uptime_seconds: IntGauge,

    /// Service info (always 1, carries version label).
    pub service_info: IntGauge,
}

impl Default for AppMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl AppMetrics {
    /// Create and register all metrics.
    ///
    /// Called once at startup. Panics if metric registration fails
    /// (indicates a programming error, not a runtime condition).
    pub fn new() -> Self {
        let registry = Registry::new();

        // Helper closures to register without unwrap/expect
        let reg_counter_vec = |opts: Opts, labels: &[&str]| -> IntCounterVec {
            let metric = match IntCounterVec::new(opts, labels) {
                Ok(m) => m,
                Err(e) => panic!("failed to create IntCounterVec: {e}"),
            };
            if let Err(e) = registry.register(Box::new(metric.clone())) {
                panic!("failed to register IntCounterVec: {e}");
            }
            metric
        };

        let reg_histogram_vec = |opts: HistogramOpts, labels: &[&str]| -> HistogramVec {
            let metric = match HistogramVec::new(opts, labels) {
                Ok(m) => m,
                Err(e) => panic!("failed to create HistogramVec: {e}"),
            };
            if let Err(e) = registry.register(Box::new(metric.clone())) {
                panic!("failed to register HistogramVec: {e}");
            }
            metric
        };

        let reg_gauge = |name: &str, help: &str| -> IntGauge {
            let metric = match IntGauge::new(name, help) {
                Ok(m) => m,
                Err(e) => panic!("failed to create IntGauge: {e}"),
            };
            if let Err(e) = registry.register(Box::new(metric.clone())) {
                panic!("failed to register IntGauge: {e}");
            }
            metric
        };

        // HTTP request counter
        let http_requests_total = reg_counter_vec(
            Opts::new("tucano_http_requests_total", "Total HTTP requests"),
            &["method", "path", "status"],
        );
        // Pre-initialize a zero series so Prometheus exposition contains the metric family immediately
        http_requests_total.with_label_values(&["GET", "/api/v1/health", "200"]);

        // HTTP request duration histogram
        let http_request_duration_seconds = reg_histogram_vec(
            HistogramOpts::new(
                "tucano_http_request_duration_seconds",
                "HTTP request duration in seconds",
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0,
            ]),
            &["method", "path"],
        );

        // In-flight requests
        let http_requests_in_flight = reg_gauge(
            "tucano_http_requests_in_flight",
            "HTTP requests currently being processed",
        );

        // Cache metrics
        let cache_hits_total =
            reg_counter_vec(Opts::new("tucano_cache_hits_total", "Cache hits"), &[]);

        let cache_misses_total =
            reg_counter_vec(Opts::new("tucano_cache_misses_total", "Cache misses"), &[]);

        let cache_evictions_total = reg_counter_vec(
            Opts::new("tucano_cache_evictions_total", "Cache evictions"),
            &[],
        );

        let cache_entries = reg_gauge("tucano_cache_entries", "Current number of cache entries");

        // Rate limit metrics
        let rate_limited_total = reg_counter_vec(
            Opts::new(
                "tucano_rate_limited_total",
                "Requests rejected by rate limiter",
            ),
            &[],
        );

        // Data metrics
        let data_files_loaded =
            reg_gauge("tucano_data_files_loaded", "Number of Parquet files loaded");

        let data_cotahist_quotes_total = reg_gauge(
            "tucano_data_cotahist_quotes_total",
            "Total COTAHIST quotes loaded",
        );

        let data_ibov_rows_total = reg_gauge(
            "tucano_data_ibov_rows_total",
            "Total IBOV data points loaded",
        );

        let data_tickers_total =
            reg_gauge("tucano_data_tickers_total", "Number of tickers in registry");

        // Service metrics
        let uptime_seconds = reg_gauge("tucano_uptime_seconds", "Service uptime in seconds");

        let service_info = match IntGauge::with_opts(
            Opts::new("tucano_service_info", "Service information")
                .const_label("version", env!("CARGO_PKG_VERSION")),
        ) {
            Ok(g) => g,
            Err(e) => panic!("failed to create service_info gauge: {e}"),
        };
        service_info.set(1);
        if let Err(e) = registry.register(Box::new(service_info.clone())) {
            panic!("failed to register service_info: {e}");
        }

        Self {
            registry,
            http_requests_total,
            http_request_duration_seconds,
            http_requests_in_flight,
            cache_hits_total,
            cache_misses_total,
            cache_evictions_total,
            cache_entries,
            rate_limited_total,
            data_files_loaded,
            data_cotahist_quotes_total,
            data_ibov_rows_total,
            data_tickers_total,
            uptime_seconds,
            service_info,
        }
    }

    /// Render all metrics as Prometheus exposition format.
    pub fn render(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        if encoder.encode(&metric_families, &mut buffer).is_err() {
            return String::new();
        }
        String::from_utf8(buffer).unwrap_or_default()
    }

    /// Update data metrics from the DataLoader.
    pub fn update_data_metrics(
        &self,
        files: usize,
        cotahist_rows: usize,
        ibov_rows: usize,
        tickers: usize,
    ) {
        self.data_files_loaded.set(files as i64);
        self.data_cotahist_quotes_total.set(cotahist_rows as i64);
        self.data_ibov_rows_total.set(ibov_rows as i64);
        self.data_tickers_total.set(tickers as i64);
    }

    /// Update cache metrics from the CacheStore.
    pub fn update_cache_metrics(&self, entries: usize, _hits: u64, _misses: u64, _evictions: u64) {
        self.cache_entries.set(entries as i64);
        // Note: cache hits/misses are tracked via counters in middleware
    }

    /// Update uptime gauge.
    pub fn update_uptime(&self, seconds: u64) {
        self.uptime_seconds.set(seconds as i64);
    }
}
