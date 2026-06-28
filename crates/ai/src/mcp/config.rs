//! Configuration for external MCP (Model Context Protocol) servers whose tools
//! are exposed to the Assistant alongside the built-in portfolio tools.
//!
//! Loaded from `WF_ASSISTANT_MCP_CONFIG` (path to a JSON file) or, if unset, from
//! `<db_dir>/assistant_mcp_servers.json`. Absent file => no MCP servers (feature off).

use serde::{Deserialize, Serialize};

/// Top-level MCP configuration: a list of servers to connect on chat start.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpConfig {
    #[serde(default)]
    pub servers: Vec<McpServer>,
}

/// A single MCP server definition. Either an HTTP/SSE endpoint (preferred — e.g.
/// pointing at an mcpproxy that already aggregates many servers) or a stdio
/// subprocess spawned by the Wealthfolio server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    /// Stable id, used in logs and as a tool-name prefix (`<id>__<tool>`).
    pub id: String,
    /// Disable without deleting the entry.
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(flatten)]
    pub transport: McpTransport,
    /// Optional per-server allowlist of tool names. None => all tools the
    /// server advertises.
    #[serde(default)]
    pub tools: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "transport", rename_all = "snake_case")]
pub enum McpTransport {
    /// HTTP/SSE endpoint (recommended for the existing mcpproxy at a URL).
    Sse {
        url: String,
        #[serde(default)]
        headers: std::collections::BTreeMap<String, String>,
    },
    /// Streamable-HTTP endpoint.
    Http {
        url: String,
        #[serde(default)]
        headers: std::collections::BTreeMap<String, String>,
    },
    /// stdio subprocess. NOTE: spawns a child process inside the server; prefer
    /// `sse`/`http` against mcpproxy in the hosted deployment.
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: std::collections::BTreeMap<String, String>,
    },
}

fn default_true() -> bool {
    true
}

impl McpConfig {
    /// Load from `WF_ASSISTANT_MCP_CONFIG` or `<db_dir>/assistant_mcp_servers.json`. Returns an empty
    /// config (no error) when no file is present, so MCP is strictly opt-in.
    pub fn load(db_dir: &std::path::Path) -> Self {
        let path = std::env::var_os("WF_ASSISTANT_MCP_CONFIG")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| db_dir.join("assistant_mcp_servers.json"));
        match std::fs::read_to_string(&path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
                log::warn!("Invalid MCP config at {}: {e}. Ignoring.", path.display());
                McpConfig::default()
            }),
            Err(_) => McpConfig::default(),
        }
    }

    pub fn enabled_servers(&self) -> impl Iterator<Item = &McpServer> {
        self.servers.iter().filter(|s| s.enabled)
    }
}
