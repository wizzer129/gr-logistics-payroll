//! Business audit trail (`audit_log`) — separate from operational logging. Always called
//! with the caller's transaction, so the audit row commits or rolls back with the change.

use serde_json::Value;
use sqlx::{PgConnection, types::Json};
use uuid::Uuid;

#[derive(Clone, Copy, Debug)]
pub enum AuditAction {
    Create,
    Update,
    Delete,
}

impl AuditAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Delete => "delete",
        }
    }
}

pub struct AuditEntry {
    pub entity: &'static str,
    pub entity_id: Uuid,
    pub action: AuditAction,
    pub changed_by: Uuid,
    /// `{}` when there's nothing to record (e.g. deletes), as the Bun backend stored.
    pub diff: Value,
}

pub async fn record_audit(conn: &mut PgConnection, entry: AuditEntry) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO audit_log (entity, entity_id, action, changed_by, diff)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(entry.entity)
    .bind(entry.entity_id)
    .bind(entry.action.as_str())
    .bind(entry.changed_by)
    .bind(Json(entry.diff))
    .execute(conn)
    .await?;
    Ok(())
}
