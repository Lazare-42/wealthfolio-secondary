//! MCP (Model Context Protocol) integration for the Assistant.
//!
//! Connects to external MCP servers (see [`config::McpConfig`]) and exposes their
//! tools to the rig agent alongside the built-in portfolio tools. The agent's
//! existing multi-turn loop then dispatches MCP tools exactly like native ones.
//!
//! Integration point: `crates/ai/src/chat/streaming.rs`, where `allowed_tools`
//! is built — call [`load_mcp_tools`] and `allowed_tools.extend(..)` before
//! `.tools(allowed_tools)`.
//!
//! Gated behind the crate `mcp` feature. Default build = no-op stub (returns an
//! empty tool list), so MCP is strictly opt-in and adds no deps unless enabled.

pub mod config;

use config::McpConfig;
use rig::tool::ToolDyn;

// ───────────────────────── default build: no-op stub ─────────────────────────
#[cfg(not(feature = "mcp"))]
pub async fn load_mcp_tools(
    _cfg: &McpConfig,
    _tools_allowlist: &Option<Vec<String>>,
) -> Vec<Box<dyn ToolDyn>> {
    Vec::new()
}

// ─────────────────────── `mcp` feature: live (rmcp 0.13) ──────────────────────
#[cfg(feature = "mcp")]
mod live {
    use super::*;
    use config::McpTransport;
    use once_cell::sync::Lazy;
    use rmcp::{model::Tool, service::ServerSink, ServiceExt};
    use std::collections::HashMap;
    use tokio::sync::Mutex;

    /// A connected server kept alive for the process lifetime so its `ServerSink`
    /// stays valid. Reconnecting per-chat would be wasteful for a fixed local
    /// server like msgvault.
    struct Conn {
        // Held only to keep the connection (and thus `sink`) alive. The concrete
        // type is `rmcp::service::RunningService<rmcp::RoleClient, ()>`.
        _running: Box<dyn std::any::Any + Send + Sync>,
        sink: ServerSink,
        tools: Vec<Tool>,
    }

    static CONNS: Lazy<Mutex<HashMap<String, Conn>>> = Lazy::new(|| Mutex::new(HashMap::new()));

    /// Connect (once) to every enabled server and wrap its tools as rig `ToolDyn`.
    pub async fn load_mcp_tools(
        cfg: &McpConfig,
        tools_allowlist: &Option<Vec<String>>,
    ) -> Vec<Box<dyn ToolDyn>> {
        let mut out: Vec<Box<dyn ToolDyn>> = Vec::new();
        let mut conns = CONNS.lock().await;
        for server in cfg.enabled_servers() {
            if !conns.contains_key(&server.id) {
                match connect(server).await {
                    Ok(conn) => {
                        conns.insert(server.id.clone(), conn);
                    }
                    Err(e) => {
                        tracing::warn!("MCP server '{}' unavailable: {e}. Skipping.", server.id);
                        continue;
                    }
                }
            }
            let conn = &conns[&server.id];
            for tool in &conn.tools {
                let name = tool.name.to_string();
                let qualified = format!("{}__{}", server.id, name);
                let allowed = match tools_allowlist {
                    None => true,
                    Some(list) => list.iter().any(|t| t == &qualified || t == &name),
                };
                let server_scoped = server
                    .tools
                    .as_ref()
                    .map(|l| l.iter().any(|t| t == &name))
                    .unwrap_or(true);
                if allowed && server_scoped {
                    out.push(Box::new(rig::tool::rmcp::McpTool::from_mcp_server(
                        tool.clone(),
                        conn.sink.clone(),
                    )));
                }
            }
        }
        out
    }

    async fn connect(server: &config::McpServer) -> Result<Conn, String> {
        match &server.transport {
            McpTransport::Http { url, .. } | McpTransport::Sse { url, .. } => {
                use rmcp::transport::StreamableHttpClientTransport;
                let transport = StreamableHttpClientTransport::from_uri(url.as_str());
                let running = ().serve(transport).await.map_err(|e| format!("serve {url}: {e}"))?;
                let tools = running
                    .list_all_tools()
                    .await
                    .map_err(|e| format!("list_tools {url}: {e}"))?;
                let sink = running.peer().clone();
                tracing::info!("MCP '{}' connected: {} tools", server.id, tools.len());
                Ok(Conn {
                    sink,
                    tools,
                    _running: Box::new(running),
                })
            }
            McpTransport::Stdio { command, .. } => {
                // Not needed for the local msgvault (HTTP) wiring. Add a
                // TokioChildProcess transport here if a stdio server is required.
                Err(format!(
                    "stdio transport not implemented (server '{command}')"
                ))
            }
        }
    }
}

#[cfg(feature = "mcp")]
pub use live::load_mcp_tools;
