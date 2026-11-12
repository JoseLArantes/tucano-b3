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
