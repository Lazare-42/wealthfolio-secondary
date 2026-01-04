use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

/// Domain model representing an import session
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSession {
    pub id: String,
    pub account_id: String,
    pub file_name: Option<String>,
    pub imported_at: NaiveDateTime,
    pub activity_count: i32,
    pub success_count: i32,
    pub failed_count: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Database model for import sessions
#[derive(
    Queryable,
    Identifiable,
    Insertable,
    AsChangeset,
    Selectable,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Clone,
)]
#[diesel(table_name = crate::schema::import_sessions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ImportSessionDB {
    pub id: String,
    pub account_id: String,
    pub file_name: Option<String>,
    pub imported_at: NaiveDateTime,
    pub activity_count: i32,
    pub success_count: i32,
    pub failed_count: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Input model for creating a new import session
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewImportSession {
    pub account_id: String,
    pub file_name: Option<String>,
    pub activity_count: i32,
    pub success_count: i32,
    pub failed_count: i32,
}

impl NewImportSession {
    pub fn new(account_id: String, file_name: Option<String>) -> Self {
        Self {
            account_id,
            file_name,
            activity_count: 0,
            success_count: 0,
            failed_count: 0,
        }
    }
}

// Conversion implementations
impl From<ImportSessionDB> for ImportSession {
    fn from(db: ImportSessionDB) -> Self {
        Self {
            id: db.id,
            account_id: db.account_id,
            file_name: db.file_name,
            imported_at: db.imported_at,
            activity_count: db.activity_count,
            success_count: db.success_count,
            failed_count: db.failed_count,
            created_at: db.created_at,
            updated_at: db.updated_at,
        }
    }
}

impl From<NewImportSession> for ImportSessionDB {
    fn from(new: NewImportSession) -> Self {
        let now = chrono::Utc::now().naive_utc();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            account_id: new.account_id,
            file_name: new.file_name,
            imported_at: now,
            activity_count: new.activity_count,
            success_count: new.success_count,
            failed_count: new.failed_count,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Summary view of an import session for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSessionSummary {
    pub id: String,
    pub account_id: String,
    pub account_name: String,
    pub file_name: Option<String>,
    pub imported_at: String,
    pub activity_count: i32,
    pub success_count: i32,
    pub failed_count: i32,
}
