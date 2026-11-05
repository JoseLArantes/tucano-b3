use sqlx::PgPool;
use uuid::Uuid;

use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpsertAction {
    Inserted,
    Changed,
    Unchanged,
}

#[derive(Debug, Clone)]
pub struct UpsertResult {
    pub action: UpsertAction,
    pub record_id: String,
    pub changed_fields: Vec<String>,
}

pub struct FieldChangeAudit {
    pub table_name: String,
    pub record_pk: String,
    pub field_name: String,
    pub old_value_text: Option<String>,
    pub new_value_text: Option<String>,
    pub old_value_hash: Option<String>,
    pub new_value_hash: Option<String>,
}

pub async fn insert_field_changes(
    pool: &PgPool,
    sync_run_id: Option<&str>,
    source_file_id: &str,
    audits: &[FieldChangeAudit],
) -> Result<()> {
    for audit in audits {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO b3_field_changes (
                id, sync_run_id, source_file_id, table_name, record_pk,
                field_name, old_value_text, new_value_text, old_value_hash,
                new_value_hash, changed_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW(), NOW(), NOW())
            "#,
        )
        .bind(&id)
        .bind(sync_run_id)
        .bind(source_file_id)
        .bind(&audit.table_name)
        .bind(&audit.record_pk)
        .bind(&audit.field_name)
        .bind(&audit.old_value_text)
        .bind(&audit.new_value_text)
        .bind(&audit.old_value_hash)
        .bind(&audit.new_value_hash)
        .execute(pool)
        .await?;
    }
    Ok(())
}
