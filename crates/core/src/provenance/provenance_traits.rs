use async_trait::async_trait;

use crate::provenance::provenance_model::{
    ActivitySource, ChatSourceEmail, NewActivitySource, NewChatSourceEmail,
};
use crate::Result;

/// Storage for activity provenance + chosen source emails.
#[async_trait]
pub trait ProvenanceRepositoryTrait: Send + Sync {
    async fn add_source(&self, new: NewActivitySource) -> Result<ActivitySource>;
    async fn sources_for_activity(&self, activity_id: &str) -> Result<Vec<ActivitySource>>;
    async fn add_email(&self, new: NewChatSourceEmail) -> Result<ChatSourceEmail>;
    async fn emails_for_thread(&self, thread_id: &str) -> Result<Vec<ChatSourceEmail>>;
    async fn list_emails(&self, limit: i64) -> Result<Vec<ChatSourceEmail>>;
}

#[async_trait]
pub trait ProvenanceServiceTrait: Send + Sync {
    async fn record_source(&self, new: NewActivitySource) -> Result<ActivitySource>;
    async fn activity_sources(&self, activity_id: &str) -> Result<Vec<ActivitySource>>;
    async fn save_email(&self, new: NewChatSourceEmail) -> Result<ChatSourceEmail>;
    async fn thread_emails(&self, thread_id: &str) -> Result<Vec<ChatSourceEmail>>;
    async fn recent_emails(&self, limit: i64) -> Result<Vec<ChatSourceEmail>>;
}
