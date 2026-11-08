use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{process::Stdio, sync::Arc};
use tokio::io::AsyncWriteExt;
use tower::ServiceExt;
use tucano_b3::{
    api::{create_router, AppState},
    config::B3Settings,
    services::{
        query::{QueryPurpose, QueryService, QuoteFilters},
        sync_service::B3SyncService,
    },
};
use uuid::Uuid;

async fn fixture() -> (PgPool, PgPool, String, String, Uuid) {
    let url = std::env::var("DATABASE_URL").expect("PostgreSQL integration requires DATABASE_URL");
    let admin = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .unwrap();
    let name = format!("contract_test_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE {name}"))
        .execute(&admin)
        .await
        .unwrap();
    let mut url = url::Url::parse(&url).unwrap();
    url.set_path(&name);
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(url.as_str())
        .await
        .unwrap();
    tucano_b3::db::run_migrations(&pool).await.unwrap();
    let job = tucano_b3::services::jobs::JobService::new(pool.clone())
        .submit(
            "synthetic-fixture",
            tucano_b3::services::jobs::JobCommand::Ingest {
                source: tucano_b3::db::enums::SourceFileType::CotahistYearly,
                from: "2025-01-01".parse().unwrap(),
                to: "2025-12-31".parse().unwrap(),
            },
            tucano_b3::services::jobs::JobPriority::Backfill,
            1,
        )
        .await
        .unwrap();
    let file = Uuid::new_v4().to_string();
    let member = Uuid::new_v4().to_string();
    let revision = Uuid::new_v4();
    let fact = Uuid::new_v4();
    let version = Uuid::new_v4();
    sqlx::query("INSERT INTO b3_source_files(id,source_file_type,sha256,acquisition_channel,status) VALUES($1,'COTAHIST_YEARLY',$2,'synthetic-contract','ingested')").bind(&file).bind("a".repeat(64)).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO b3_source_file_members(id,source_file_id,member_name,sha256) VALUES($1,$2,'COTAHIST_TEST.TXT',$3)").bind(&member).bind(&file).bind("b".repeat(64)).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO b3_source_eligibility(source_file_type,channel,purpose,status,zero_cost,evidence_reference,verified_at) SELECT 'COTAHIST_YEARLY','synthetic-contract',p,'eligible',true,'SYNTHETIC TEST ONLY',NOW() FROM unnest(ARRAY['rest','mcp','third_party_delivery']) p").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO b3_history_revisions(id,job_id,source_file_id,channel,period_start,period_end,publication_key,status,expected_members) VALUES($1,$3,$2,'synthetic-contract','2025-01-01','2025-12-31','synthetic-2025','published',1)").bind(revision).bind(&file).bind(job).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO b3_cotahist_facts(id,trade_date,ticker_symbol,market_type,bdi_code,distribution_number) VALUES($1,'2025-01-02','EXMP3','010','02','001')").bind(fact).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO b3_cotahist_versions(id,fact_id,business_hash,currency,open_price,last_price,quotation_factor) VALUES($1,$2,$3,'BRL',30.10,30.20,1)").bind(version).bind(fact).bind("c".repeat(64)).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO b3_history_revision_quotes(revision_id,fact_id,version_id,source_member_id,record_index,selected_publication_key) VALUES($1,$2,$3,$4,2,'synthetic-2025')").bind(revision).bind(fact).bind(version).bind(&member).execute(&pool).await.unwrap();
    sqlx::query("UPDATE b3_history_active SET revision_id=$1 WHERE singleton")
        .bind(revision)
        .execute(&pool)
        .await
        .unwrap();
    (pool, admin, name, url.to_string(), revision)
}

#[tokio::test]
async fn quote_export_migration_upgrades_a_schema_at_version_0013() {
    let base_url =
        std::env::var("DATABASE_URL").expect("PostgreSQL integration requires DATABASE_URL");
    let admin = PgPoolOptions::new()
        .max_connections(2)
        .connect(&base_url)
        .await
        .unwrap();
    let name = format!("export_migration_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE {name}"))
        .execute(&admin)
        .await
        .unwrap();
    let mut database_url = url::Url::parse(&base_url).unwrap();
    database_url.set_path(&name);
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(database_url.as_str())
        .await
        .unwrap();

    let all_migrations = sqlx::migrate!("./migrations");
    let previous_migrations = all_migrations
        .iter()
        .filter(|migration| migration.version < 14)
        .cloned()
        .collect();
    let previous = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(previous_migrations),
        ignore_missing: false,
        locking: true,
        no_tx: false,
    };
    previous.run(&pool).await.unwrap();
    all_migrations.run(&pool).await.unwrap();

    let manifest_exists: bool =
        sqlx::query_scalar("SELECT to_regclass('public.b3_quote_export_artifacts') IS NOT NULL")
            .fetch_one(&pool)
            .await
            .unwrap();
    let cursor_index_exists: bool = sqlx::query_scalar(
        "SELECT to_regclass('public.idx_history_quotes_resolved_instrument') IS NOT NULL",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(manifest_exists);
    assert!(cursor_index_exists);
    cleanup(pool, admin, name).await;
}

async fn cleanup(pool: PgPool, admin: PgPool, name: String) {
    pool.close().await;
    sqlx::query(&format!("DROP DATABASE {name} WITH (FORCE)"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

#[tokio::test]
async fn rest_and_shared_mcp_queries_keep_precision_revision_and_provenance() {
    let (pool, admin, name, _, revision) = fixture().await;
    let service = QueryService::new(pool.clone());
    let filters = QuoteFilters {
        ticker_symbol: Some("exmp3".into()),
        ..Default::default()
    };
    let rest = service
        .quotes(filters.clone(), QueryPurpose::Rest, false)
        .await
        .unwrap();
    let mcp = service
        .quotes(filters.clone(), QueryPurpose::Mcp, false)
        .await
        .unwrap();
    let rv = serde_json::to_value(rest).unwrap();
    let mv = serde_json::to_value(mcp).unwrap();
    assert_eq!(rv, mv);
    assert_eq!(rv["data_revision"], revision.to_string());
    assert_eq!(rv["items"][0]["identity"]["status"], "unresolved");
    assert_eq!(rv["items"][0]["provenance"]["record_index"], 2);
    assert!(rv["items"][0].get("internal").is_none());
    let price: rust_decimal::Decimal = rv["items"][0]["open_price"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(price, rust_decimal::Decimal::new(3010, 2));
    assert!(rv["disclaimer"].as_str().unwrap().contains("sem ajustes"));
    let newer = Uuid::new_v4();
    sqlx::query("INSERT INTO b3_history_revisions(id,job_id,base_revision_id,source_file_id,channel,period_start,period_end,publication_key,status,expected_members) SELECT $1,job_id,id,source_file_id,channel,period_start,period_end,publication_key,'published',expected_members FROM b3_history_revisions WHERE id=$2").bind(newer).bind(revision).execute(&pool).await.unwrap();
    sqlx::query("UPDATE b3_history_active SET revision_id=$1 WHERE singleton")
        .bind(newer)
        .execute(&pool)
        .await
        .unwrap();
    let pinned = service
        .quotes(
            QuoteFilters {
                data_revision: Some(revision),
                ..filters.clone()
            },
            QueryPurpose::Rest,
            false,
        )
        .await
        .unwrap();
    assert_eq!(pinned.total, 1);
    assert_eq!(
        service
            .quotes(filters, QueryPurpose::Rest, false)
            .await
            .unwrap()
            .total,
        0
    );
    sqlx::query("UPDATE b3_source_eligibility SET status='restricted' WHERE purpose='rest'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(service
        .quotes(
            QuoteFilters {
                data_revision: Some(revision),
                ..Default::default()
            },
            QueryPurpose::Rest,
            false
        )
        .await
        .is_err());
    cleanup(pool, admin, name).await;
}

#[tokio::test]
async fn ticker_export_streams_revision_pinned_ndjson_and_reuses_private_artifact() {
    let (pool, admin, name, _, revision) = fixture().await;
    let cache_dir = tempfile::tempdir().unwrap();
    let settings = Arc::new(B3Settings {
        tucano_b3_read_token: "fixture-read".into(),
        tucano_b3_admin_token: "fixture-admin".into(),
        b3_processed_data_dir: cache_dir.path().display().to_string(),
        ..Default::default()
    });
    let app = create_router(Arc::new(AppState {
        pool: pool.clone(),
        settings: settings.clone(),
        sync_service: Arc::new(B3SyncService::new(pool.clone(), (*settings).clone())),
    }));
    let request = || {
        Request::builder()
            .uri("/api/v1/b3/cotahist/quotes/export?ticker_symbol=exmp3")
            .header("Authorization", "Bearer fixture-read")
            .body(Body::empty())
            .unwrap()
    };
    let (response, concurrent_response) = tokio::join!(
        app.clone().oneshot(request()),
        app.clone().oneshot(request())
    );
    let response = response.unwrap();
    let concurrent_response = concurrent_response.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(concurrent_response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["content-type"],
        "application/x-ndjson; charset=utf-8"
    );
    assert!(response.headers()["cache-control"]
        .to_str()
        .unwrap()
        .contains("private"));
    let first_body = axum::body::to_bytes(response.into_body(), 1_048_576)
        .await
        .unwrap();
    let concurrent_body = axum::body::to_bytes(concurrent_response.into_body(), 1_048_576)
        .await
        .unwrap();
    assert_eq!(first_body, concurrent_body);
    let ready_artifact_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM b3_quote_export_artifacts WHERE revision_id=$1 AND ticker_symbol='EXMP3' AND status='ready'",
    )
    .bind(revision)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(ready_artifact_count, 1);
    let events: Vec<Value> = first_body
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect();
    assert_eq!(events[0]["type"], "meta");
    assert_eq!(events[0]["data_revision"], revision.to_string());
    assert!(events[0]["disclaimer"]
        .as_str()
        .unwrap()
        .contains("sem ajustes"));
    let quote = events
        .iter()
        .find(|event| event["type"] == "quote")
        .unwrap();
    assert_eq!(quote["data"]["ticker_symbol"], "EXMP3");
    assert_eq!(quote["data"]["identity"]["status"], "unresolved");
    assert_eq!(quote["data"]["provenance"]["record_index"], 2);
    assert!(quote["data"].get("internal").is_none());
    let checkpoint = events
        .iter()
        .find(|event| event["type"] == "checkpoint")
        .unwrap()["cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let complete = events
        .iter()
        .find(|event| event["type"] == "complete")
        .unwrap();
    assert_eq!(complete["record_count"], 1);
    assert_eq!(complete["content_sha256"].as_str().unwrap().len(), 64);

    let cached = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/b3/cotahist/quotes/export?ticker_symbol=EXMP3&data_revision={revision}"
                ))
                .header("Authorization", "Bearer fixture-read")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(cached.status(), StatusCode::OK);
    let etag = cached.headers()["etag"].to_str().unwrap().to_owned();
    assert!(!etag.is_empty());
    assert!(cached.headers().contains_key("content-length"));
    let _ = axum::body::to_bytes(cached.into_body(), 1_048_576)
        .await
        .unwrap();

    let unchanged = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/b3/cotahist/quotes/export?ticker_symbol=EXMP3&data_revision={revision}"
                ))
                .header("Authorization", "Bearer fixture-read")
                .header("If-None-Match", etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unchanged.status(), StatusCode::NOT_MODIFIED);

    let resumed = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/b3/cotahist/quotes/export?ticker_symbol=EXMP3&cursor={checkpoint}"
                ))
                .header("Authorization", "Bearer fixture-read")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resumed.status(), StatusCode::OK);
    let resumed_body = axum::body::to_bytes(resumed.into_body(), 1_048_576)
        .await
        .unwrap();
    let resumed_events: Vec<Value> = resumed_body
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect();
    assert_eq!(resumed_events[0]["resumed_after"], checkpoint);
    assert_eq!(
        resumed_events
            .iter()
            .find(|event| event["type"] == "complete")
            .unwrap()["record_count"],
        0
    );

    sqlx::query(
        "UPDATE b3_source_eligibility SET status='restricted' WHERE channel='synthetic-contract'",
    )
    .execute(&pool)
    .await
    .unwrap();
    let denied_cache_hit = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/b3/cotahist/quotes/export?ticker_symbol=EXMP3&data_revision={revision}"
                ))
                .header("Authorization", "Bearer fixture-read")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied_cache_hit.status(), StatusCode::FORBIDDEN);
    cleanup(pool, admin, name).await;
}

#[tokio::test]
async fn admin_commands_return_202_and_keep_execution_out_of_request() {
    let (pool, admin, name, _, _) = fixture().await;
    let settings = Arc::new(B3Settings {
        tucano_b3_read_token: "fixture-read".into(),
        tucano_b3_admin_token: "fixture-admin".into(),
        ..Default::default()
    });
    let app = create_router(Arc::new(AppState {
        pool: pool.clone(),
        settings: settings.clone(),
        sync_service: Arc::new(B3SyncService::new(pool.clone(), (*settings).clone())),
    }));
    let request = json!({"idempotency_key":"fixture-command","command":{"operation":"rebuild_identity","from":"2025-01-01","to":"2025-12-31"},"priority":"rebuild"});
    let mut ids = Vec::new();
    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/b3/jobs")
                    .header("Authorization", "Bearer fixture-admin")
                    .header("content-type", "application/json")
                    .body(Body::from(request.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        assert!(response.headers().contains_key("location"));
        let value: Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), 65536)
                .await
                .unwrap(),
        )
        .unwrap();
        ids.push(value["job_id"].clone());
    }
    assert_eq!(ids[0], ids[1]);
    let status: String = sqlx::query_scalar("SELECT status FROM b3_jobs WHERE id=$1")
        .bind(ids[0].as_str().unwrap().parse::<Uuid>().unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "queued");
    cleanup(pool, admin, name).await;
}

#[tokio::test]
async fn instrument_detail_keeps_multiple_snapshots_and_identifier_lineage() {
    use tucano_b3::db::enums::SourceFileType;

    let (pool, admin, name, _, _) = fixture().await;
    let job: Uuid = sqlx::query_scalar("SELECT id FROM b3_jobs LIMIT 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    let instrument = Uuid::new_v4();
    let families = [
        SourceFileType::Bvbg028InstrumentsFile,
        SourceFileType::TradableSecurityList,
    ];
    let mut snapshots = Vec::new();
    for (index, family) in families.into_iter().enumerate() {
        let file = Uuid::new_v4().to_string();
        let member = Uuid::new_v4().to_string();
        let snapshot = Uuid::new_v4();
        let channel = format!("registry-{index}");
        let reference_date = if index == 0 {
            "2025-01-02"
        } else {
            "2026-01-02"
        };
        sqlx::query("INSERT INTO b3_source_files(id,source_file_type,sha256,acquisition_channel,status) VALUES($1,$2,$3,$4,'ingested')")
            .bind(&file)
            .bind(family.as_str())
            .bind(char::from(b'a' + index as u8).to_string().repeat(64))
            .bind(&channel)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO b3_source_file_members(id,source_file_id,member_name,sha256,member_index) VALUES($1,$2,'registry.csv',$3,0)")
            .bind(&member)
            .bind(&file)
            .bind(char::from(b'c' + index as u8).to_string().repeat(64))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO b3_source_eligibility(source_file_type,channel,purpose,status,zero_cost,evidence_reference,verified_at) SELECT $1,$2,p,'eligible',true,'SYNTHETIC TEST ONLY',NOW() FROM unnest(ARRAY['rest','third_party_delivery']) p")
            .bind(family.as_str())
            .bind(&channel)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO b3_registry_snapshots(id,job_id,source_file_id,source_file_type,channel,reference_date,expected_members,status,published_at) VALUES($1,$2,$3,$4,$5,$6::date,1,'published',NOW())")
            .bind(snapshot)
            .bind(job)
            .bind(&file)
            .bind(family.as_str())
            .bind(&channel)
            .bind(reference_date)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO b3_registry_active(source_file_type,snapshot_id) VALUES($1,$2)")
            .bind(family.as_str())
            .bind(snapshot)
            .execute(&pool)
            .await
            .unwrap();
        if index == 0 {
            sqlx::query("INSERT INTO b3_instruments(id,security_id,security_source,market_identifier_code,source_file_id,source_row_hash) VALUES($1,'12345','8','BVMF',$2,$3)")
                .bind(instrument.to_string())
                .bind(&file)
                .bind("d".repeat(64))
                .execute(&pool)
                .await
                .unwrap();
        }
        sqlx::query("INSERT INTO b3_registry_records(snapshot_id,source_member_id,record_index,security_id,security_source,market_identifier_code,ticker_symbol,isin,asset_description,business_hash,instrument_id) VALUES($1,$2,1,'12345','8','BVMF','EXMP3','BREXMPACNOR0',$3,$4,$5)")
            .bind(snapshot)
            .bind(&member)
            .bind(format!("Empresa de teste {index}"))
            .bind(char::from(b'e' + index as u8).to_string().repeat(64))
            .bind(instrument.to_string())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO b3_instrument_identifiers(id,instrument_id,identifier_type,identifier_value,market_code,valid_from,valid_to,source_file_id,source_member_name,source_line_number,source_row_hash) VALUES($1,$2,'ticker','EXMP3','010',$3::date,$3::date,$4,'registry.csv',1,$5)")
            .bind(Uuid::new_v4().to_string())
            .bind(instrument.to_string())
            .bind(reference_date)
            .bind(&file)
            .bind(char::from(b'f' + index as u8).to_string().repeat(64))
            .execute(&pool)
            .await
            .unwrap();
        snapshots.push(snapshot);
    }

    let settings = Arc::new(B3Settings {
        tucano_b3_read_token: "fixture-read".into(),
        tucano_b3_admin_token: "fixture-admin".into(),
        ..Default::default()
    });
    let app = create_router(Arc::new(AppState {
        pool: pool.clone(),
        settings: settings.clone(),
        sync_service: Arc::new(B3SyncService::new(pool.clone(), (*settings).clone())),
    }));
    let get = |uri: String| {
        app.clone().oneshot(
            Request::builder()
                .uri(uri)
                .header("Authorization", "Bearer fixture-read")
                .body(Body::empty())
                .unwrap(),
        )
    };
    let response = get(format!("/api/v1/b3/instruments/{instrument}?limit=10"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let details: Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 65536)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(details["total"], 2);
    assert_eq!(details["snapshots"].as_array().unwrap().len(), 2);
    assert_ne!(
        details["items"][0]["snapshot_id"],
        details["items"][1]["snapshot_id"]
    );

    let response = get(format!(
        "/api/v1/b3/instruments/{instrument}/identifiers?limit=10"
    ))
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let identifiers: Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 65536)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(identifiers["total"], 2);
    assert_eq!(identifiers["items"][0]["provenance"]["record_index"], 1);
    assert!(identifiers["items"][0]["provenance"]["source_member_id"].is_string());

    let response = get("/api/v1/b3/tradable-securities?ticker_symbol=EXMP3".into())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let securities: Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 65536)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(securities["total"], 1);
    assert_eq!(
        securities["items"][0]["provenance"]["source_file_type"],
        "TRADABLE_SECURITY_LIST"
    );
    assert_ne!(snapshots[0], snapshots[1]);
    cleanup(pool, admin, name).await;
}

#[tokio::test]
async fn mcp_real_process_uses_readonly_role_and_rejects_mutations() {
    let (pool, admin, name, url, _) = fixture().await;
    let role = format!("mcp_read_{}", Uuid::new_v4().simple());
    let password = Uuid::new_v4().simple().to_string();
    sqlx::query(&format!("CREATE ROLE {role} LOGIN PASSWORD '{password}'"))
        .execute(&admin)
        .await
        .unwrap();
    sqlx::query(&format!("GRANT CONNECT ON DATABASE {name} TO {role}"))
        .execute(&admin)
        .await
        .unwrap();
    sqlx::query(&format!("GRANT USAGE ON SCHEMA public TO {role}"))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(&format!(
        "GRANT SELECT ON ALL TABLES IN SCHEMA public TO {role}"
    ))
    .execute(&pool)
    .await
    .unwrap();
    let mut readonly = url::Url::parse(&url).unwrap();
    readonly.set_username(&role).unwrap();
    readonly.set_password(Some(&password)).unwrap();
    let read_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(readonly.as_str())
        .await
        .unwrap();
    assert!(sqlx::query("CREATE TABLE must_not_exist(id int)")
        .execute(&read_pool)
        .await
        .is_err());
    read_pool.close().await;
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_tucano-b3"))
        .arg("mcp")
        .env("DATABASE_URL", readonly.as_str())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"integration","version":"1"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_ohlcv_series","arguments":{"ticker_symbol":"EXMP3","market_type":"010"}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"sync","arguments":{}}}),
    ];
    let mut stdin = child.stdin.take().unwrap();
    for message in messages {
        stdin
            .write_all(format!("{message}\n").as_bytes())
            .await
            .unwrap();
    }
    drop(stdin);
    let output = tokio::time::timeout(std::time::Duration::from_secs(30), child.wait_with_output())
        .await
        .unwrap()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let responses: Vec<Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).expect("stdout must contain JSON-RPC only"))
        .collect();
    assert_eq!(responses.len(), 4);
    assert!(responses[1]["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .all(|t| t["name"] != "sync" && t["name"] != "get_index_portfolio"));
    assert_eq!(responses[2]["result"]["isError"], false);
    assert_eq!(responses[3]["result"]["isError"], true);
    let quote: Value = serde_json::from_str(
        responses[2]["result"]["content"][0]["text"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(quote["items"][0]["ticker_symbol"], "EXMP3");
    pool.close().await;
    sqlx::query(&format!("DROP DATABASE {name} WITH (FORCE)"))
        .execute(&admin)
        .await
        .unwrap();
    sqlx::query(&format!("DROP ROLE {role}"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}
