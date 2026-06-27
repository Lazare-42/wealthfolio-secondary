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

## To make it live (the 4 remaining steps)
1. **Cargo** (`crates/ai/Cargo.toml:34`): enable rig's MCP feature + a transport
   client. Confirm the exact names against rig-core 0.30:
   ```toml
   rig = { package = "rig-core", version = "0.30",
           features = ["reqwest-rustls", "mcp"] }
   rmcp = { version = "*", features = ["client", "transport-sse-client"] }
   ```
   (rig may already re-export an MCP client; if so, drop the separate `rmcp` dep.)
2. **`connect_and_list`** (`src/mcp/mod.rs`): implement per `McpTransport`
   (sse/http/stdio) — connect, `initialize`, `tools/list`; return `(name, def, client)`.
3. **`load_mcp_tools`**: wrap each MCP tool as a rig tool and push it:
   ```rust
   out.push(Box::new(rig::tool::McpTool::from_mcp_server(def, client.clone())));
   ```
   (verify the constructor/path in rig 0.30 — could be `rig_core::tool::mcp`).
4. **streaming.rs**: uncomment the hook; ensure `env.db_dir()` (or equivalent)
   exposes the DB dir for `McpConfig::load`. Add it to the `AiEnvironment` trait
   if missing.

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
