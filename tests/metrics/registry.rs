use tucano_b3::metrics::AppMetrics;

#[test]
fn test_metrics_creation() {
    let metrics = AppMetrics::new();
    let output = metrics.render();
    assert!(output.contains("tucano_http_requests_total"));
    assert!(output.contains("tucano_uptime_seconds"));
    assert!(output.contains("tucano_service_info"));
}

#[test]
fn test_metrics_render_format() {
    let metrics = AppMetrics::new();
    let output = metrics.render();
    // Prometheus format: metric_name{labels} value
    assert!(output.contains("# HELP"));
    assert!(output.contains("# TYPE"));
}

#[test]
fn test_update_data_metrics() {
    let metrics = AppMetrics::new();
    metrics.update_data_metrics(3, 14_000_000, 7_500, 1_247);

    let output = metrics.render();
    assert!(output.contains("tucano_data_files_loaded 3"));
    assert!(output.contains("tucano_data_tickers_total 1247"));
}

#[test]
fn test_update_uptime() {
    let metrics = AppMetrics::new();
    metrics.update_uptime(3600);

    let output = metrics.render();
    assert!(output.contains("tucano_uptime_seconds 3600"));
}

#[test]
fn test_increment_request_counter() {
    let metrics = AppMetrics::new();
    metrics
        .http_requests_total
        .with_label_values(&["GET", "/api/v1/health", "200"])
        .inc();

    let output = metrics.render();
    assert!(output.contains("tucano_http_requests_total"));
}

#[test]
fn test_update_cache_metrics() {
    let metrics = AppMetrics::new();
    metrics.update_cache_metrics(42, 10, 5, 2);

    let output = metrics.render();
    assert!(output.contains("tucano_cache_entries 42"));
}
