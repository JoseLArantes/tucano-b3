use tucano_b3::api::openapi::ApiDoc;
use utoipa::OpenApi;

#[test]
fn test_openapi_schema_has_no_unresolved_refs() {
    let doc = ApiDoc::openapi();
    let json_val = serde_json::to_value(&doc).unwrap();
    let schemas = json_val
        .pointer("/components/schemas")
        .and_then(|s| s.as_object())
        .expect("components.schemas should be an object");

    // Check all $ref in the entire document
    fn check_refs(val: &serde_json::Value, schemas: &serde_json::Map<String, serde_json::Value>) {
        match val {
            serde_json::Value::Object(map) => {
                if let Some(r) = map.get("$ref").and_then(|r| r.as_str()) {
                    if let Some(schema_name) = r.strip_prefix("#/components/schemas/") {
                        assert!(
                            schemas.contains_key(schema_name),
                            "Broken $ref: '{}' not found in components.schemas! Available: {:?}",
                            schema_name,
                            schemas.keys().collect::<Vec<_>>()
                        );
                    }
                }
                for v in map.values() {
                    check_refs(v, schemas);
                }
            }
            serde_json::Value::Array(arr) => {
                for v in arr {
                    check_refs(v, schemas);
                }
            }
            _ => {}
        }
    }

    check_refs(&json_val, schemas);
}

#[test]
fn market_calendar_and_negotiation_hours_are_documented() {
    let document = serde_json::to_value(ApiDoc::openapi()).unwrap();
    let paths = document.pointer("/paths").unwrap();
    for (path, method) in [
        ("/calendar", "get"),
        ("/calendar", "post"),
        ("/calendar/{year}", "get"),
        ("/calendar/{year}", "put"),
        ("/calendar/{year}", "delete"),
        ("/negotiation-hours", "get"),
    ] {
        assert!(paths
            .get(path)
            .and_then(|entry| entry.get(method))
            .is_some());
    }

    let schemas = document.pointer("/components/schemas").unwrap();
    for schema in [
        "MarketCalendarCreateRequest",
        "MarketCalendarUpdateRequest",
        "MarketCalendarEvent",
        "MarketCalendarResponse",
        "NegotiationHoursResponse",
    ] {
        assert!(schemas.get(schema).is_some(), "missing schema {schema}");
    }
    assert_eq!(
        schemas
            .pointer("/MarketCalendarCreateRequest/additionalProperties")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(
        schemas
            .pointer("/MarketCalendarCreateRequest/example/ano")
            .and_then(serde_json::Value::as_i64),
        Some(2027)
    );
    assert!(paths
        .get("/calendar")
        .and_then(|path| path.get("post"))
        .and_then(|operation| operation.get("description"))
        .and_then(serde_json::Value::as_str)
        .is_some());
}
