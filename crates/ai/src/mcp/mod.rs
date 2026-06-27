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
//! STATUS: scaffold. The rig-0.30 MCP adapter call is marked TODO below; enabling
//! it requires the `mcp` feature on `rig-core` (see crates/ai/Cargo.toml) and a
//! transport client crate (`rmcp`). Until wired, this returns an empty Vec so the
//! build stays green and MCP is a strict no-op.

pub mod config;

use config::{McpConfig, McpTransport};
use rig::tool::ToolDyn;

/// Connect to all enabled MCP servers and return their tools as rig `ToolDyn`
/// boxes, ready to extend the agent's `allowed_tools`.
///
/// `tools_allowlist`: the same provider/UI allowlist used for native tools.
/// MCP tool names are namespaced `<server_id>__<tool>` so they can be allow/deny
/// listed without colliding with built-ins.
pub async fn load_mcp_tools(
    cfg: &McpConfig,
    tools_allowlist: &Option<Vec<String>>,
) -> Vec<Box<dyn ToolDyn>> {
    let mut out: Vec<Box<dyn ToolDyn>> = Vec::new();
    for server in cfg.enabled_servers() {
        match connect_and_list(server).await {
            Ok(tools) => {
                for (name, _tool) in tools {
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
                    if !(allowed && server_scoped) {
                        continue;
                    }
                    // TODO(rig-0.30 mcp): wrap the MCP tool def + client as a rig
                    // tool and push it:
                    //   out.push(Box::new(rig::tool::McpTool::from_mcp_server(_tool, client.clone())));
                    // The exact constructor/feature-gate must be confirmed against
                    // rig-core 0.30's `mcp` feature (or the `rmcp` adapter).
                    let _ = qualified; // silence unused until wired
                }
            }
            Err(e) => {
                tracing::warn!("MCP server '{}' unavailable: {e}. Skipping.", server.id);
            }
        }
    }
    out
}

/// Placeholder for the transport connect + `tools/list`. Returns (tool_name, def).
/// TODO: implement with `rmcp` (HTTP/SSE/stdio) per `server.transport`.
async fn connect_and_list(
    server: &config::McpServer,
) -> Result<Vec<(String, serde_json::Value)>, String> {
    match &server.transport {
        McpTransport::Sse { url, .. } | McpTransport::Http { url, .. } => {
            tracing::debug!("MCP connect (scaffold): {} -> {url}", server.id);
            Ok(Vec::new())
        }
        McpTransport::Stdio { command, .. } => {
            tracing::debug!("MCP connect (scaffold): {} -> stdio {command}", server.id);
            Ok(Vec::new())
        }
    }
}
