# MCP servers in the Assistant — design / implementation plan

Branch: `feature/mcp-assistant`. Goal: let the Wealthfolio Assistant call tools
from external **MCP (Model Context Protocol)** servers in addition to the 19
built-in portfolio tools.

## Why a code change (not a setting)
The Assistant runs on `rig` (`rig-core 0.30`). Tools are a fixed `ToolSet`
(`crates/ai/src/tools/mod.rs`) compiled into the binary and registered in
`crates/ai/src/chat/streaming.rs` (the `build_with_tools_and_stream!` macro,
`allowed_tools: Vec<Box<dyn ToolDyn>>` → `.tools(allowed_tools)`). There is no
runtime tool registry today, so MCP needs wiring in `crates/ai`.

`rig` supports MCP, so this is a contained patch, not a rewrite.

## Architecture
```
chat request ─► streaming.rs (build allowed_tools)
                   ├─ 19 built-in tools (native)
                   └─ MCP tools  ◄── mcp::load_mcp_tools(cfg)
                          │
                          ├─ connect each enabled server (sse/http/stdio)
                          ├─ tools/list  → tool schemas
                          └─ wrap each as rig ToolDyn (rig `mcp` feature)
                   ▼
              rig agent .multi_turn(6)  ─► dispatches native + MCP tools the same way
```

## Files in this branch (scaffold — compiles, no-op)
- `src/mcp/config.rs` — config schema + loader. `McpConfig::load(db_dir)` reads
  `WF_MCP_CONFIG` or `<db_dir>/mcp_servers.json`; absent file ⇒ MCP off.
- `src/mcp/mod.rs` — `load_mcp_tools()` + `connect_and_list()` stub. Returns an
  empty `Vec<Box<dyn ToolDyn>>` until the rig adapter is wired (marked `TODO`).
- `src/lib.rs` — `pub mod mcp;`.
- `src/chat/streaming.rs` — commented integration point right before
  `.tools(allowed_tools)`.

## To make it live (the 4 remaining steps) — rig-0.30 API CONFIRMED

Verified against the cached `rig-core-0.30.0` source: feature is **`rmcp`** (not
`mcp`); wrapper is **`rig::tool::rmcp::McpTool`** which **impls `ToolDyn`**.

1. **Cargo** (`crates/ai/Cargo.toml:34`): enable rig's `rmcp` feature + the `rmcp`
   client crate (rig depends on `rmcp = "0.13"`):
   ```toml
   rig  = { package = "rig-core", version = "0.30",
            features = ["reqwest-rustls", "rmcp"] }
   rmcp = { version = "0.13", features = ["client",
            "transport-sse-client", "transport-streamable-http-client",
            "transport-child-process"] }   # pick transports you use
   ```
2. **`connect_and_list`** (`src/mcp/mod.rs`): per `McpTransport`, build an `rmcp`
   client → `serve_client(transport)` → `RunningService`. From it: `.list_tools()`
   → `Vec<rmcp::model::Tool>`, and the **`ServerSink`** via `running.peer().clone()`.
   Return `(Vec<rmcp::model::Tool>, ServerSink)`. Keep each `RunningService` alive
   for the chat duration (hold in a struct dropped at stream end).
3. **`load_mcp_tools`**: wrap + push directly (no builder change needed):
   ```rust
   out.push(Box::new(
       rig::tool::rmcp::McpTool::from_mcp_server(tool, server_sink.clone()),
   ));
   ```
   (Alternative: `builder.rmcp_tools(tools, client)` exists but returns a different
   builder type — `AgentBuilderSimple` — which would break the existing
   `.tools().tool_choice().temperature()...` chain. Prefer the `ToolDyn` push.)
4. **streaming.rs**: uncomment the hook; ensure `env.db_dir()` (or equivalent)
   exposes the DB dir for `McpConfig::load`. Add it to the `AiEnvironment` trait
   if missing.

`from_mcp_server(tool: rmcp::model::Tool, client: rmcp::service::ServerSink)` —
signature confirmed in `rig-core-0.30.0/src/agent/builder.rs`.

Then regen the nix cargo hash and rebuild (same as the `chore(port)` commits).

## Config example (`mcp_servers.json`)
Prefer **sse/http against the existing mcpproxy** (one endpoint, many servers) over
spawning stdio children inside the server:
```json
{
  "servers": [
    { "id": "vault", "transport": "sse",
      "url": "http://127.0.0.1:PORT/sse",
      "headers": { "Authorization": "Bearer ..." },
      "tools": ["search_messages", "get_message"] }
  ]
}
```

## Safety / ops notes
- MCP tool names are namespaced `<server_id>__<tool>` so they can be allow/deny
  listed via the existing `tools_allowlist` without colliding with built-ins.
- Network/secret-bearing tools widen the agent's reach — keep `tools` per-server
  allowlists tight; the agent already gates mutations via draft/confirm flows.
- A down server is logged and skipped (chat still works with native tools).
- stdio transport writes temp/pipes; on this deployment keep to sse/http and let
  mcpproxy own server lifecycles (also avoids the root-disk temp issue).
