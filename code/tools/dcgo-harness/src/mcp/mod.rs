//! The exam's agent surface: a JSON-RPC 2.0 MCP server over stdio.
//!
//! Every tool is a thin projection over machinery that already exists — the
//! clause binding, the differ, the ledger, the committed rules derivations — so
//! each behaviour stays unit-testable with no MCP client in the loop.
//!
//! **Payloads are small on purpose.** Orchestration was the largest single line
//! item of the first campaign ($704 of $4,210, driven by half a billion
//! cache-read tokens) because an agent shelled out to the CLI and re-read large
//! outputs. A tool that returns the whole verdict store has failed at its job:
//! return what the agent needs to act, and a count of what was elided.

pub mod handlers;
pub mod protocol;
pub mod tools;

use std::io::{BufRead, Write};
use std::path::PathBuf;

use protocol::{JsonRpcRequest, JsonRpcResponse, INTERNAL_ERROR, METHOD_NOT_FOUND};

/// Serve MCP over stdio until EOF.
pub fn serve(root: Option<PathBuf>) -> Result<(), String> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let line = line.map_err(|e| format!("reading stdin: {e}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                // Same code `digimon-engine-mcp` uses for an unparseable
                // request line -- see protocol::PARSE_ERROR.
                let resp =
                    JsonRpcResponse::error(None, protocol::PARSE_ERROR, &format!("{e}"));
                writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap())
                    .map_err(|e| format!("writing stdout: {e}"))?;
                stdout.flush().ok();
                continue;
            }
        };

        // A notification carries no id and must not be answered.
        let is_notification = req.id.is_none();
        let resp = handle(&req, root.as_deref());
        if is_notification {
            continue;
        }
        writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap())
            .map_err(|e| format!("writing stdout: {e}"))?;
        stdout.flush().ok();
    }
    Ok(())
}

fn handle(req: &JsonRpcRequest, root: Option<&std::path::Path>) -> JsonRpcResponse {
    match req.method.as_str() {
        "initialize" => JsonRpcResponse::result(
            req.id.clone(),
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "dcgo-exam", "version": env!("CARGO_PKG_VERSION")}
            }),
        ),
        "initialized" | "notifications/initialized" => {
            JsonRpcResponse::result(req.id.clone(), serde_json::json!({}))
        }
        "tools/list" => JsonRpcResponse::result(
            req.id.clone(),
            serde_json::json!({ "tools": tools::list() }),
        ),
        "tools/call" => {
            let params = req.params.clone().unwrap_or(serde_json::Value::Null);
            let name = params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            match handlers::dispatch(&name, &params, root) {
                Ok(value) => JsonRpcResponse::result(
                    req.id.clone(),
                    serde_json::json!({
                        "content": [{"type": "text", "text": value.to_string()}],
                        "isError": false
                    }),
                ),
                // A tool-level failure is reported as tool content, not as a
                // protocol error: the agent must be able to read WHY and fix
                // its input, which a transport-level error would hide.
                Err(message) => JsonRpcResponse::result(
                    req.id.clone(),
                    serde_json::json!({
                        "content": [{"type": "text", "text": message}],
                        "isError": true
                    }),
                ),
            }
        }
        other => JsonRpcResponse::error(
            req.id.clone(),
            METHOD_NOT_FOUND,
            &format!("unknown method {other:?}"),
        ),
    }
    .normalize(INTERNAL_ERROR)
}

impl JsonRpcResponse {
    /// Identity today; the hook exists so a future panic-catch has one place to
    /// convert into a protocol error rather than dropping the connection.
    fn normalize(self, _code: i64) -> JsonRpcResponse {
        self
    }
}
