use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Where an imported activity originated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Email,
    Pdf,
    Csv,
    Bank,
    Manual,
    Chat,
}

impl SourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceKind::Email => "email",
            SourceKind::Pdf => "pdf",
            SourceKind::Csv => "csv",
            SourceKind::Bank => "bank",
            SourceKind::Manual => "manual",
            SourceKind::Chat => "chat",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "email" => SourceKind::Email,
            "pdf" => SourceKind::Pdf,
            "csv" => SourceKind::Csv,
            "bank" => SourceKind::Bank,
            "chat" => SourceKind::Chat,
            _ => SourceKind::Manual,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivitySource {
    pub id: String,
    pub activity_id: String,
    pub source_kind: SourceKind,
    pub source_ref: Option<String>,
    /// Self-link: the activity whose cash funded this one (e.g. the SELL that
    /// funded a LOAN_ORIGINATION).
    pub funding_activity_id: Option<String>,
    pub thread_id: Option<String>,
    pub detail: Option<Value>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewActivitySource {
    pub activity_id: String,
    pub source_kind: SourceKind,
    #[serde(default)]
    pub source_ref: Option<String>,
    #[serde(default)]
    pub funding_activity_id: Option<String>,
    #[serde(default)]
    pub thread_id: Option<String>,
    #[serde(default)]
    pub detail: Option<Value>,
}

impl NewActivitySource {
    pub fn into_record(self) -> ActivitySource {
        ActivitySource {
            id: Uuid::new_v4().to_string(),
            activity_id: self.activity_id,
            source_kind: self.source_kind,
            source_ref: self.source_ref,
            funding_activity_id: self.funding_activity_id,
            thread_id: self.thread_id,
            detail: self.detail,
            created_at: now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSourceEmail {
    pub id: String,
    pub thread_id: Option<String>,
    pub message_id: String,
    pub subject: Option<String>,
    pub sender: Option<String>,
    pub sent_at: Option<String>,
    pub snapshot: Option<Value>,
    pub linked_activity_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewChatSourceEmail {
    #[serde(default)]
    pub thread_id: Option<String>,
    pub message_id: String,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub sender: Option<String>,
    #[serde(default)]
    pub sent_at: Option<String>,
    #[serde(default)]
    pub snapshot: Option<Value>,
    #[serde(default)]
    pub linked_activity_id: Option<String>,
}

impl NewChatSourceEmail {
    pub fn into_record(self) -> ChatSourceEmail {
        ChatSourceEmail {
            id: Uuid::new_v4().to_string(),
            thread_id: self.thread_id,
            message_id: self.message_id,
            subject: self.subject,
            sender: self.sender,
            sent_at: self.sent_at,
            snapshot: self.snapshot,
            linked_activity_id: self.linked_activity_id,
            created_at: now(),
        }
    }
}

fn now() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}
