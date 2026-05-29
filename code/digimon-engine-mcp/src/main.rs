//! `digimon-engine-mcp` — Model Context Protocol stdio server.
//!
//! Phase 6 of the `add-engine-debug-mcp` change. Speaks line-delimited
//! JSON-RPC 2.0 over stdio per the MCP spec. Exposes 22 tools that wrap
//! `digimon_engine::live_game::LiveGame`:
//!
//! - **Lifecycle** (6): `new_game_from_decks`, `new_game_debug`,
//!   `load_recording`, `seek`, `list_games`, `close_game`.
//! - **State inspection** (10): `state`, `hand`, `field`, `security`,
//!   `pending_selection`, `effect_queue`, `events`, `modifiers`,
//!   `inspect_card`, `legal_actions`.
//! - **Actions** (6): `play`, `resolve_selection`, `end_turn`,
//!   `pass_turn`, `move_from_breeding`, `step`.
//!
//! The protocol is hand-rolled rather than going through an MCP SDK —
//! the surface needed here (initialize, tools/list, tools/call) is
//! small enough that the SDK dependency would add more friction than
//! value for v1.

use std::collections::HashMap;
use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use digimon_engine::card_data::CardData;
use digimon_engine::live_game::LiveGame;
use serde_json::{json, Value};

mod protocol;
mod registry;
mod tools;

use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use registry::GameRegistry;

/// CLI options for the server. Mirrors the `digimon-engine-cli` shape so
/// admins switching between the two get a consistent interface.
#[derive(Parser, Debug)]
#[command(
    name = "digimon-engine-mcp",
    version,
    about = "MCP stdio server for the Digimon TCG engine"
)]
struct ServerOpts {
    /// Card-pool source. Default `implemented` (intersection with
    /// `LiveGame::default_pool()` — the engine's registered cards, same
    /// filter used by `pilot_training` / `gauntlet`).
    #[arg(long, default_value = "implemented")]
    pool: String,

    /// Override for `cards.json` path. If absent, the server walks up
    /// from the working directory looking for `data/cards.json`.
    #[arg(long)]
    cards_json: Option<PathBuf>,

    /// Upper bound on the number of concurrent in-memory games. Lifecycle
    /// tools return a structured error when the cap would be exceeded.
    #[arg(long, default_value_t = 32)]
    max_games: usize,
}

fn main() -> ExitCode {
    let opts = ServerOpts::parse();

    let card_data = match load_card_data(&opts.cards_json, &opts.pool) {
        Ok(cd) => cd,
        Err(e) => {
            eprintln!("digimon-engine-mcp: failed to load card data: {}", e);
            return ExitCode::from(2);
        }
    };

    let mut registry = GameRegistry::new(opts.max_games);
    eprintln!(
        "digimon-engine-mcp: ready (pool={}, max_games={}, cards_loaded={})",
        opts.pool,
        opts.max_games,
        card_data.len()
    );

    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut buf = String::new();

    loop {
        buf.clear();
        match reader.read_line(&mut buf) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                eprintln!("digimon-engine-mcp: stdin error: {}", e);
                break;
            }
        }
        let line = buf.trim();
        if line.is_empty() {
            continue;
        }
        let req: JsonRpcRequest = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(e) => {
                emit_error(None, -32700, &format!("parse error: {}", e));
                continue;
            }
        };
        let response = handle_request(req, &mut registry, &card_data);
        emit_response(response);
    }

    ExitCode::SUCCESS
}

fn handle_request(
    req: JsonRpcRequest,
    registry: &mut GameRegistry,
    card_data: &HashMap<String, CardData>,
) -> JsonRpcResponse {
    let id = req.id.clone();
    match req.method.as_str() {
        "initialize" => JsonRpcResponse::result(
            id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name": "digimon-engine-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        ),
        "initialized" | "notifications/initialized" => {
            // Notification — no response. Caller is expected to not send
            // an `id` field, but if they do we ack with empty result.
            JsonRpcResponse::result(id, Value::Null)
        }
        "tools/list" => JsonRpcResponse::result(id, json!({ "tools": tools::list() })),
        "tools/call" => match dispatch_tool_call(req.params, registry, card_data) {
            Ok(result_value) => {
                // Per MCP spec, tool responses are wrapped as `{ content: [...] }`.
                let text = serde_json::to_string(&result_value).unwrap_or_else(|_| "{}".into());
                JsonRpcResponse::result(
                    id,
                    json!({
                        "content": [
                            { "type": "text", "text": text }
                        ]
                    }),
                )
            }
            Err(e) => JsonRpcResponse::error(
                id,
                JsonRpcError {
                    code: -32602,
                    message: e,
                    data: None,
                },
            ),
        },
        "ping" => JsonRpcResponse::result(id, Value::Null),
        other => JsonRpcResponse::error(
            id,
            JsonRpcError {
                code: -32601,
                message: format!("method not found: {}", other),
                data: None,
            },
        ),
    }
}

/// Dispatch a `tools/call` invocation. Returns Err for protocol-level
/// errors (malformed args); tool-level failures (illegal action,
/// missing card) come back as `{ ok: false, error: "..." }` inside the
/// Ok result envelope per the spec's "Action Tools" requirement.
fn dispatch_tool_call(
    params: Option<Value>,
    registry: &mut GameRegistry,
    card_data: &HashMap<String, CardData>,
) -> Result<Value, String> {
    let p = params.ok_or_else(|| "missing params".to_string())?;
    let name = p
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "tools/call: missing 'name'".to_string())?;
    let args = p.get("arguments").cloned().unwrap_or_else(|| json!({}));
    tools::dispatch(name, args, registry, card_data)
}

fn emit_response(resp: JsonRpcResponse) {
    if resp.is_notification_ack() {
        return; // Notifications: no response on the wire.
    }
    let line = serde_json::to_string(&resp).unwrap_or_else(|_| "{}".into());
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let _ = writeln!(handle, "{}", line);
    let _ = handle.flush();
}

fn emit_error(id: Option<Value>, code: i64, message: &str) {
    emit_response(JsonRpcResponse::error(
        id,
        JsonRpcError {
            code,
            message: message.to_string(),
            data: None,
        },
    ));
}

// ── card-data loader (shared shape with the CLI) ─────────────────────────

fn load_card_data(
    cards_json: &Option<PathBuf>,
    pool_spec: &str,
) -> Result<HashMap<String, CardData>, String> {
    let path = cards_json
        .clone()
        .or_else(default_cards_json_path)
        .ok_or_else(|| {
            "no --cards-json path provided and no data/cards.json found in current dir or ancestors"
                .to_string()
        })?;
    let bytes = std::fs::read(&path).map_err(|e| format!("reading {}: {}", path.display(), e))?;
    let text =
        std::str::from_utf8(&bytes).map_err(|e| format!("cards.json is not valid UTF-8: {}", e))?;
    let all = CardData::load_from_str(text).map_err(|e| format!("parsing cards.json: {}", e))?;

    match pool_spec {
        "all" => Ok(all),
        "implemented" => {
            let pool = LiveGame::default_pool();
            Ok(all
                .into_iter()
                .filter(|(id, _)| pool.contains(id))
                .collect())
        }
        other => {
            let p = PathBuf::from(other);
            let bytes = std::fs::read(&p)
                .map_err(|e| format!("reading pool file {}: {}", p.display(), e))?;
            let pool: Vec<String> = serde_json::from_slice(&bytes)
                .map_err(|e| format!("parsing pool file as [card_id...]: {}", e))?;
            let pool: HashSet<String> = pool.into_iter().collect();
            Ok(all
                .into_iter()
                .filter(|(id, _)| pool.contains(id))
                .collect())
        }
    }
}

fn default_cards_json_path() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    for _ in 0..6 {
        let candidate = dir.join("data").join("cards.json");
        if candidate.exists() {
            return Some(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}
