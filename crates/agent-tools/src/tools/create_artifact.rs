//! Create Artifact tool.
//!
//! Lets the assistant author a standalone financial document — a written
//! report or a structured table — that the frontend renders in a side panel
//! next to the chat instead of inline in the message stream. The document
//! content travels in the tool-call ARGS (the frontend reads it from there),
//! so this tool's job is only to validate the request and hand back a stable
//! artifact id the model can reuse to revise the same document later.
//!
//! It touches no services: the model has already gathered the underlying data
//! via the read tools and composes the document itself.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::env::AgentEnvironment;
use crate::scope::AgentScope;
use crate::tool::{AgentTool, AgentToolAccess, AgentToolError, AgentToolResult};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateArtifactArgs {
    /// Document title shown in the panel header.
    pub title: String,
    /// "report" (markdown prose + GFM tables) or "table" (structured grid).
    pub kind: String,
    /// Stable slug identifying this document (e.g. "allocation-review"). Reuse
    /// the same value to replace/revise an existing artifact; omit to create a
    /// fresh one.
    #[serde(default)]
    pub artifact_id: Option<String>,
    /// One-line summary shown on the inline chat chip.
    #[serde(default)]
    pub summary: Option<String>,
    /// Markdown body for `kind: "report"`. Supports GitHub-flavored tables.
    #[serde(default)]
    pub markdown: Option<String>,
    /// Structured grid for `kind: "table"`.
    #[serde(default)]
    pub table: Option<ArtifactTable>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactTable {
    pub columns: Vec<ArtifactColumn>,
    /// Rows as objects keyed by column `key`. Values may be string or number.
    pub rows: Vec<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactColumn {
    /// Field key used in each row object.
    pub key: String,
    /// Human-readable column header.
    pub label: String,
    /// Cell alignment: "left" (default), "right", or "center".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
    /// Display format hint: "currency", "percent", "number", or "text".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

/// Minimal confirmation handed back to the model. The document content is NOT
/// echoed here — it lives in the tool-call args the frontend renders — so the
/// model's history stays lean.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateArtifactOutput {
    pub artifact_id: String,
    pub kind: String,
    pub title: String,
    pub status: String,
    pub message: String,
}

const CREATE_ARTIFACT_DESCRIPTION: &str =
    "Author a standalone financial document that is shown to the user in a side panel \
     next to the chat, instead of inline in your message. Use this for substantial, \
     self-contained outputs the user will want to read, scroll, or copy as a unit — \
     e.g. a portfolio review, an allocation breakdown, a performance report, a \
     rebalancing plan, or a comparison table. Do NOT use it for short conversational \
     answers; reply normally for those. \
     \n\nKINDS: use `kind: \"report\"` with a `markdown` body for written analysis \
     (headings, prose, bullet lists, and GitHub-flavored tables are supported). Use \
     `kind: \"table\"` with `table.columns` + `table.rows` for a single structured \
     grid of figures. \
     \n\nWORKFLOW: gather the data with the read tools first, then call this once with \
     the finished document. To revise a document you already created in this \
     conversation, call again with the SAME `artifactId` slug. Base every figure on \
     real tool output — never invent numbers.";

pub struct CreateArtifact;

impl CreateArtifact {
    fn build_output(args: CreateArtifactArgs) -> Result<CreateArtifactOutput, AgentToolError> {
        let title = args.title.trim().to_string();
        if title.is_empty() {
            return Err(AgentToolError::ExecutionFailed(
                "title is required and cannot be empty".to_string(),
            ));
        }

        let kind = args.kind.trim().to_lowercase();
        match kind.as_str() {
            "report" => {
                let has_body = args
                    .markdown
                    .as_deref()
                    .map(str::trim)
                    .is_some_and(|s| !s.is_empty());
                if !has_body {
                    return Err(AgentToolError::ExecutionFailed(
                        "kind \"report\" requires a non-empty `markdown` body".to_string(),
                    ));
                }
            }
            "table" => {
                let has_columns = args.table.as_ref().is_some_and(|t| !t.columns.is_empty());
                if !has_columns {
                    return Err(AgentToolError::ExecutionFailed(
                        "kind \"table\" requires `table.columns` with at least one column"
                            .to_string(),
                    ));
                }
            }
            other => {
                return Err(AgentToolError::ExecutionFailed(format!(
                    "unsupported kind \"{other}\": use \"report\" or \"table\""
                )));
            }
        }

        let artifact_id = args
            .artifact_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| Uuid::now_v7().to_string());

        Ok(CreateArtifactOutput {
            artifact_id,
            kind,
            title: title.clone(),
            status: "ready".to_string(),
            message: format!("Opened \"{title}\" in the document panel."),
        })
    }
}

#[async_trait::async_trait]
impl AgentTool for CreateArtifact {
    fn name(&self) -> &'static str {
        "create_artifact"
    }

    fn description(&self) -> &'static str {
        CREATE_ARTIFACT_DESCRIPTION
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "Document title shown in the panel header."
                },
                "kind": {
                    "type": "string",
                    "enum": ["report", "table"],
                    "description": "\"report\" for markdown prose/analysis, \"table\" for a structured grid."
                },
                "artifactId": {
                    "type": "string",
                    "description": "Stable slug (e.g. \"allocation-review\"). Reuse to revise an existing document; omit to create a new one."
                },
                "summary": {
                    "type": "string",
                    "description": "One-line summary shown on the inline chat chip."
                },
                "markdown": {
                    "type": "string",
                    "description": "Markdown body for kind \"report\". GitHub-flavored tables supported."
                },
                "table": {
                    "type": "object",
                    "description": "Structured grid for kind \"table\".",
                    "properties": {
                        "columns": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "key": { "type": "string", "description": "Field key used in each row object." },
                                    "label": { "type": "string", "description": "Column header." },
                                    "align": { "type": "string", "enum": ["left", "right", "center"], "description": "Cell alignment. Default left; use right for numbers." },
                                    "format": { "type": "string", "enum": ["currency", "percent", "number", "text"], "description": "Display format hint." }
                                },
                                "required": ["key", "label"]
                            }
                        },
                        "rows": {
                            "type": "array",
                            "items": { "type": "object", "description": "Row keyed by column key; values string or number." }
                        }
                    },
                    "required": ["columns", "rows"]
                }
            },
            "required": ["title", "kind"]
        })
    }

    fn required_scopes(&self) -> &'static [AgentScope] {
        // Composes a document from data the model already gathered; it reads no
        // services itself, so it needs no scopes. (Assistant-only — never
        // exposed through the scope-gated MCP catalog.)
        &[]
    }

    fn access_level(&self) -> AgentToolAccess {
        AgentToolAccess::Suggest
    }

    async fn call(
        &self,
        _env: Arc<dyn AgentEnvironment>,
        args: serde_json::Value,
    ) -> Result<AgentToolResult, AgentToolError> {
        let args: CreateArtifactArgs = serde_json::from_value(args)?;
        let output = CreateArtifact::build_output(args)?;
        Ok(AgentToolResult {
            content: serde_json::to_value(output)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call_with(value: serde_json::Value) -> Result<CreateArtifactOutput, AgentToolError> {
        let args: CreateArtifactArgs = serde_json::from_value(value).unwrap();
        CreateArtifact::build_output(args)
    }

    #[test]
    fn report_requires_markdown_body() {
        let err = call_with(serde_json::json!({ "title": "X", "kind": "report" })).unwrap_err();
        assert!(matches!(err, AgentToolError::ExecutionFailed(_)));
    }

    #[test]
    fn table_requires_columns() {
        let err = call_with(serde_json::json!({
            "title": "X",
            "kind": "table",
            "table": { "columns": [], "rows": [] }
        }))
        .unwrap_err();
        assert!(matches!(err, AgentToolError::ExecutionFailed(_)));
    }

    #[test]
    fn unknown_kind_is_rejected() {
        let err = call_with(serde_json::json!({ "title": "X", "kind": "chart" })).unwrap_err();
        assert!(matches!(err, AgentToolError::ExecutionFailed(_)));
    }

    #[test]
    fn report_generates_id_when_omitted() {
        let out = call_with(serde_json::json!({
            "title": "Allocation",
            "kind": "report",
            "markdown": "# Hello"
        }))
        .unwrap();
        assert!(!out.artifact_id.is_empty());
        assert_eq!(out.kind, "report");
        assert_eq!(out.status, "ready");
    }

    #[test]
    fn explicit_slug_is_preserved() {
        let out = call_with(serde_json::json!({
            "title": "Allocation",
            "kind": "report",
            "artifactId": "allocation-review",
            "markdown": "# Hello"
        }))
        .unwrap();
        assert_eq!(out.artifact_id, "allocation-review");
    }

    #[test]
    fn table_round_trips() {
        let out = call_with(serde_json::json!({
            "title": "Holdings",
            "kind": "TABLE",
            "table": {
                "columns": [{ "key": "sym", "label": "Symbol" }],
                "rows": [{ "sym": "AAPL" }]
            }
        }))
        .unwrap();
        assert_eq!(out.kind, "table");
    }

    #[test]
    fn schema_required_fields_are_title_and_kind() {
        let schema = CreateArtifact.input_schema();
        let required: Vec<String> = schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        assert_eq!(required, vec!["title", "kind"]);
    }
}
