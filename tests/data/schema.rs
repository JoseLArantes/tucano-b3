use arrow::datatypes::{DataType, Field, Schema};
use tucano_b3::data::schema::{cotahist_schema, ibov_schema, validate_schema};

#[test]
fn test_cotahist_schema_has_required_fields() {
    let schema = cotahist_schema();
    let field_names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();

    assert!(field_names.contains(&"ticker"));
    assert!(field_names.contains(&"date"));
    assert!(field_names.contains(&"pre_ult"));
    assert!(field_names.contains(&"vol_tot"));
    assert!(field_names.contains(&"year"));
}

#[test]
fn test_ibov_schema_has_required_fields() {
    let schema = ibov_schema();
    let field_names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();

    assert!(field_names.contains(&"date"));
    assert!(field_names.contains(&"codigo"));
    assert!(field_names.contains(&"pre_ult"));
    assert!(field_names.contains(&"vol_tot"));
    assert!(field_names.contains(&"pre_ofe_compra")); // nullable future field
}

#[test]
fn test_validate_schema_matching() {
    let expected = cotahist_schema();
    let file_schema = expected.as_ref().clone();
    let result = validate_schema(&file_schema, &expected, "test.parquet");
    assert!(result.is_ok());
}

#[test]
fn test_validate_schema_missing_field() {
    let expected = cotahist_schema();
    // Create a schema missing the "ticker" field
    let fields: Vec<Field> = expected
        .fields()
        .iter()
        .filter(|f| f.name() != "ticker")
        .map(|f| f.as_ref().clone())
        .collect();
    let file_schema = Schema::new(fields);

    let result = validate_schema(&file_schema, &expected, "test.parquet");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("ticker"));
}

#[test]
fn test_validate_schema_type_mismatch() {
    let expected = cotahist_schema();
    // Create a schema with wrong type for "ticker"
    let fields: Vec<Field> = expected
        .fields()
        .iter()
        .map(|f| {
            if f.name() == "ticker" {
                Field::new("ticker", DataType::Int32, false) // Wrong type
            } else {
                f.as_ref().clone()
            }
        })
        .collect();
    let file_schema = Schema::new(fields);

    let result = validate_schema(&file_schema, &expected, "test.parquet");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("TypeMismatch") || err.to_string().contains("type"));
}

#[test]
fn test_validate_schema_extra_field_allowed() {
    let expected = cotahist_schema();
    // Add an extra field (forward compatibility)
    let mut fields: Vec<Field> = expected
        .fields()
        .iter()
        .map(|f| f.as_ref().clone())
        .collect();
    fields.push(Field::new("extra_field", DataType::Utf8, true));
    let file_schema = Schema::new(fields);

    let result = validate_schema(&file_schema, &expected, "test.parquet");
    assert!(result.is_ok()); // Extra fields are allowed
}
