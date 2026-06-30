use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::errors::StorageError;
use wealthfolio_core::provenance::{
    ActivitySource, ChatSourceEmail, NewActivitySource, NewChatSourceEmail, SourceKind,
};
use wealthfolio_core::{Error, Result};

#[derive(Queryable, Insertable, Selectable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = crate::schema::activity_sources)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ActivitySourceDB {
    pub id: String,
    pub activity_id: String,
    pub source_kind: String,
    pub source_ref: Option<String>,
    pub funding_activity_id: Option<String>,
    pub thread_id: Option<String>,
    pub detail_json: Option<String>,
    pub created_at: String,
}

impl ActivitySourceDB {
    pub fn from_record(record: ActivitySource) -> Result<Self> {
        Ok(Self {
            id: record.id,
            activity_id: record.activity_id,
            source_kind: record.source_kind.as_str().to_string(),
            source_ref: record.source_ref,
            funding_activity_id: record.funding_activity_id,
            thread_id: record.thread_id,
            detail_json: match record.detail {
                Some(v) => {
                    Some(serde_json::to_string(&v).map_err(|e| {
                        Error::from(StorageError::SerializationError(e.to_string()))
                    })?)
                }
                None => None,
            },
            created_at: record.created_at,
        })
    }

    pub fn into_domain(self) -> Result<ActivitySource> {
        Ok(ActivitySource {
            id: self.id,
            activity_id: self.activity_id,
            source_kind: SourceKind::parse(&self.source_kind),
            source_ref: self.source_ref,
            funding_activity_id: self.funding_activity_id,
            thread_id: self.thread_id,
            detail: match self.detail_json {
                Some(s) => {
                    Some(serde_json::from_str(&s).map_err(|e| {
                        Error::from(StorageError::SerializationError(e.to_string()))
                    })?)
                }
                None => None,
            },
            created_at: self.created_at,
        })
    }

    pub fn for_new(new: NewActivitySource) -> Result<Self> {
        Self::from_record(new.into_record())
    }
}

#[derive(Queryable, Insertable, Selectable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = crate::schema::chat_source_emails)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ChatSourceEmailDB {
    pub id: String,
    pub thread_id: Option<String>,
    pub message_id: String,
    pub subject: Option<String>,
    pub sender: Option<String>,
    pub sent_at: Option<String>,
    pub snapshot_json: Option<String>,
    pub linked_activity_id: Option<String>,
    pub created_at: String,
}

impl ChatSourceEmailDB {
    pub fn from_record(record: ChatSourceEmail) -> Result<Self> {
        Ok(Self {
            id: record.id,
            thread_id: record.thread_id,
            message_id: record.message_id,
            subject: record.subject,
            sender: record.sender,
            sent_at: record.sent_at,
            snapshot_json: match record.snapshot {
                Some(v) => {
                    Some(serde_json::to_string(&v).map_err(|e| {
                        Error::from(StorageError::SerializationError(e.to_string()))
                    })?)
                }
                None => None,
            },
            linked_activity_id: record.linked_activity_id,
            created_at: record.created_at,
        })
    }

    pub fn into_domain(self) -> Result<ChatSourceEmail> {
        Ok(ChatSourceEmail {
            id: self.id,
            thread_id: self.thread_id,
            message_id: self.message_id,
            subject: self.subject,
            sender: self.sender,
            sent_at: self.sent_at,
            snapshot: match self.snapshot_json {
                Some(s) => {
                    Some(serde_json::from_str(&s).map_err(|e| {
                        Error::from(StorageError::SerializationError(e.to_string()))
                    })?)
                }
                None => None,
            },
            linked_activity_id: self.linked_activity_id,
            created_at: self.created_at,
        })
    }

    pub fn for_new(new: NewChatSourceEmail) -> Result<Self> {
        Self::from_record(new.into_record())
    }
}
