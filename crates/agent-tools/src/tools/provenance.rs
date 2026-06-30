//! Assistant-only provenance tools: let a chat session save the transaction
//! email it chose as the source for an import, and link an activity to its
//! source (and the activity that funded it). These touch the provenance store
//! only — no portfolio mutations — so they are assistant-only (not scope-gated
//! MCP tools).

use serde::Deserialize;
use std::sync::Arc;

use crate::env::AgentEnvironment;
use crate::scope::AgentScope;
use crate::tool::{AgentTool, AgentToolAccess, AgentToolError, AgentToolResult};
use wealthfolio_core::provenance::{NewActivitySource, NewChatSourceEmail, SourceKind};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSourceEmailArgs {
    pub message_id: String,
    #[serde(default)]
    pub thread_id: Option<String>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub sender: Option<String>,
    #[serde(default)]
    pub sent_at: Option<String>,
    #[serde(default)]
    pub linked_activity_id: Option<String>,
    #[serde(default)]
    pub snapshot: Option<serde_json::Value>,
}

pub struct SaveSourceEmail;

#[async_trait::async_trait]
impl AgentTool for SaveSourceEmail {
    fn name(&self) -> &'static str {
        "save_source_email"
    }

    fn description(&self) -> &'static str {
        "Save a transaction email this session chose as the source for an import, so the link is traceable later. Provide the archive/message id and any subject/sender/date you have; optionally link it to the activity it produced."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "messageId": { "type": "string", "description": "Archive/Gmail message id of the source email." },
                "threadId": { "type": "string", "description": "Chat thread id, when known." },
                "subject": { "type": "string" },
                "sender": { "type": "string" },
                "sentAt": { "type": "string", "description": "ISO date the email was sent." },
                "linkedActivityId": { "type": "string", "description": "Activity this email is the source for, if already created." },
                "snapshot": { "type": "object", "description": "Optional extra captured fields (headers, extracted amounts)." }
            },
            "required": ["messageId"]
        })
    }

    fn required_scopes(&self) -> &'static [AgentScope] {
        &[]
    }

    fn access_level(&self) -> AgentToolAccess {
        AgentToolAccess::Write
    }

    async fn call(
        &self,
        env: Arc<dyn AgentEnvironment>,
        args: serde_json::Value,
    ) -> Result<AgentToolResult, AgentToolError> {
        let args: SaveSourceEmailArgs = serde_json::from_value(args)?;
        let saved = env
            .provenance_service()
            .save_email(NewChatSourceEmail {
                thread_id: args.thread_id,
                message_id: args.message_id,
                subject: args.subject,
                sender: args.sender,
                sent_at: args.sent_at,
                snapshot: args.snapshot,
                linked_activity_id: args.linked_activity_id,
            })
            .await
            .map_err(|e| AgentToolError::ExecutionFailed(e.to_string()))?;
        Ok(AgentToolResult {
            content: serde_json::to_value(saved)?,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkActivitySourceArgs {
    pub activity_id: String,
    /// email | pdf | csv | bank | manual | chat
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub source_ref: Option<String>,
    /// The activity whose cash funded this one (e.g. the SELL that funded a loan).
    #[serde(default)]
    pub funding_activity_id: Option<String>,
    #[serde(default)]
    pub thread_id: Option<String>,
    #[serde(default)]
    pub detail: Option<serde_json::Value>,
}

pub struct LinkActivitySource;

#[async_trait::async_trait]
impl AgentTool for LinkActivitySource {
    fn name(&self) -> &'static str {
        "link_activity_source"
    }

    fn description(&self) -> &'static str {
        "Attach provenance to an activity: where it came from (email/pdf/csv/bank/chat) and, optionally, the activity that funded it (e.g. the SELL whose proceeds funded a loan origination). Makes imports traceable."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "activityId": { "type": "string", "description": "Activity to attach provenance to." },
                "sourceKind": { "type": "string", "enum": ["email", "pdf", "csv", "bank", "manual", "chat"], "description": "Where it came from. Defaults to chat." },
                "sourceRef": { "type": "string", "description": "Email/message id, file name, url, or run id." },
                "fundingActivityId": { "type": "string", "description": "The activity whose cash funded this one." },
                "threadId": { "type": "string" },
                "detail": { "type": "object", "description": "Optional note/extra fields." }
            },
            "required": ["activityId"]
        })
    }

    fn required_scopes(&self) -> &'static [AgentScope] {
        &[]
    }

    fn access_level(&self) -> AgentToolAccess {
        AgentToolAccess::Write
    }

    async fn call(
        &self,
        env: Arc<dyn AgentEnvironment>,
        args: serde_json::Value,
    ) -> Result<AgentToolResult, AgentToolError> {
        let args: LinkActivitySourceArgs = serde_json::from_value(args)?;
        let source_kind = args
            .source_kind
            .map(|s| SourceKind::parse(&s))
            .unwrap_or(SourceKind::Chat);
        let created = env
            .provenance_service()
            .record_source(NewActivitySource {
                activity_id: args.activity_id,
                source_kind,
                source_ref: args.source_ref,
                funding_activity_id: args.funding_activity_id,
                thread_id: args.thread_id,
                detail: args.detail,
            })
            .await
            .map_err(|e| AgentToolError::ExecutionFailed(e.to_string()))?;
        Ok(AgentToolResult {
            content: serde_json::to_value(created)?,
        })
    }
}
