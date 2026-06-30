use async_trait::async_trait;
use diesel::prelude::*;
use diesel::r2d2::{self, Pool};
use diesel::sqlite::SqliteConnection;
use std::sync::Arc;

use crate::db::{get_connection, WriteHandle};
use crate::errors::StorageError;
use crate::schema::{activity_sources, chat_source_emails};
use wealthfolio_core::provenance::{
    ActivitySource, ChatSourceEmail, NewActivitySource, NewChatSourceEmail,
    ProvenanceRepositoryTrait,
};
use wealthfolio_core::Result;

use super::model::{ActivitySourceDB, ChatSourceEmailDB};

pub struct ProvenanceRepository {
    pool: Arc<Pool<r2d2::ConnectionManager<SqliteConnection>>>,
    writer: WriteHandle,
}

impl ProvenanceRepository {
    pub fn new(
        pool: Arc<Pool<r2d2::ConnectionManager<SqliteConnection>>>,
        writer: WriteHandle,
    ) -> Self {
        Self { pool, writer }
    }
}

#[async_trait]
impl ProvenanceRepositoryTrait for ProvenanceRepository {
    async fn add_source(&self, new: NewActivitySource) -> Result<ActivitySource> {
        let row = ActivitySourceDB::for_new(new)?;
        self.writer
            .exec_tx(move |tx| {
                diesel::insert_into(activity_sources::table)
                    .values(&row)
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;
                row.into_domain()
            })
            .await
    }

    async fn sources_for_activity(&self, activity_id: &str) -> Result<Vec<ActivitySource>> {
        let mut conn = get_connection(&self.pool)?;
        let rows = activity_sources::table
            .filter(activity_sources::activity_id.eq(activity_id))
            .select(ActivitySourceDB::as_select())
            .order(activity_sources::created_at.desc())
            .load::<ActivitySourceDB>(&mut conn)
            .map_err(StorageError::from)?;
        rows.into_iter()
            .map(ActivitySourceDB::into_domain)
            .collect()
    }

    async fn add_email(&self, new: NewChatSourceEmail) -> Result<ChatSourceEmail> {
        let row = ChatSourceEmailDB::for_new(new)?;
        self.writer
            .exec_tx(move |tx| {
                diesel::insert_into(chat_source_emails::table)
                    .values(&row)
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;
                row.into_domain()
            })
            .await
    }

    async fn emails_for_thread(&self, thread_id: &str) -> Result<Vec<ChatSourceEmail>> {
        let mut conn = get_connection(&self.pool)?;
        let rows = chat_source_emails::table
            .filter(chat_source_emails::thread_id.eq(thread_id))
            .select(ChatSourceEmailDB::as_select())
            .order(chat_source_emails::created_at.desc())
            .load::<ChatSourceEmailDB>(&mut conn)
            .map_err(StorageError::from)?;
        rows.into_iter()
            .map(ChatSourceEmailDB::into_domain)
            .collect()
    }

    async fn list_emails(&self, limit: i64) -> Result<Vec<ChatSourceEmail>> {
        let mut conn = get_connection(&self.pool)?;
        let rows = chat_source_emails::table
            .select(ChatSourceEmailDB::as_select())
            .order(chat_source_emails::created_at.desc())
            .limit(limit)
            .load::<ChatSourceEmailDB>(&mut conn)
            .map_err(StorageError::from)?;
        rows.into_iter()
            .map(ChatSourceEmailDB::into_domain)
            .collect()
    }
}
