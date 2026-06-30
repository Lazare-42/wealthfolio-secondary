use async_trait::async_trait;
use std::sync::Arc;

use crate::provenance::provenance_model::{
    ActivitySource, ChatSourceEmail, NewActivitySource, NewChatSourceEmail,
};
use crate::provenance::provenance_traits::{ProvenanceRepositoryTrait, ProvenanceServiceTrait};
use crate::Result;

pub struct ProvenanceService {
    repo: Arc<dyn ProvenanceRepositoryTrait>,
}

impl ProvenanceService {
    pub fn new(repo: Arc<dyn ProvenanceRepositoryTrait>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ProvenanceServiceTrait for ProvenanceService {
    async fn record_source(&self, new: NewActivitySource) -> Result<ActivitySource> {
        self.repo.add_source(new).await
    }
    async fn activity_sources(&self, activity_id: &str) -> Result<Vec<ActivitySource>> {
        self.repo.sources_for_activity(activity_id).await
    }
    async fn save_email(&self, new: NewChatSourceEmail) -> Result<ChatSourceEmail> {
        self.repo.add_email(new).await
    }
    async fn thread_emails(&self, thread_id: &str) -> Result<Vec<ChatSourceEmail>> {
        self.repo.emails_for_thread(thread_id).await
    }
    async fn recent_emails(&self, limit: i64) -> Result<Vec<ChatSourceEmail>> {
        self.repo.list_emails(limit).await
    }
}
