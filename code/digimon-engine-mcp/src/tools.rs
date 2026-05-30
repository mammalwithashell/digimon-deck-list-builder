//! Tool registration and dispatch for the MCP server.
//!
//! Each tool has:
//! - a stable name (the contract surface — never rename without a spec
//!   delta per `engine-debug-mcp` "Tool Surface Stability")
//! - a JSONSchema description of its input shape (returned to the
//!   client via `tools/list`)
//! - a Rust handler that takes JSON args + a mutable `GameRegistry`
//!   reference and returns a JSON result value
//!
//! Tool-level errors (illegal action, missing card, unknown game_id)
//! are returned as `{ ok: false, error: "..." }` inside the result
//! envelope — they never surface as MCP-protocol errors, so agents can
//! read the rejection reason without having a tool call fail mid-flow.

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::dcgo_recording::parse_jsonl;
use digimon_engine::live_game::{LiveGame, LiveGameError};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::view::Perspective;
use serde_json::{json, Value};

use crate::registry::{GameRegistry, RegistryError};

/// Return the JSON spec the MCP client receives in response to
/// `tools/list`. Order is stable; tools are grouped by category.
pub fn list() -> Vec<Value> {
    let mut all = Vec::new();

    // ── Lifecycle ────────────────────────────────────────────────────────
    all.push(tool(
        "new_game_from_decks",
        "Construct a fresh game from two decklists. Shuffles via the engine RNG; pass seed for determinism.",
        json!({
            "type": "object",
            "required": ["deck1", "deck2"],
            "properties": {
                "deck1": { "type": "array", "items": { "type": "string" } },
                "deck2": { "type": "array", "items": { "type": "string" } },
                "seed": { "type": ["integer", "null"] }
            }
        }),
    ));
    all.push(tool(
        "new_game_debug",
        "Construct a fresh game from explicit hand + deck per player. No shuffling. Mirrors DebugRunnerBuilder. Optionally seed `security` and `digitama` zones and initial `memory`.",
        json!({
            "type": "object",
            "required": ["hands", "decks", "first_player"],
            "properties": {
                "hands": {
                    "type": "object",
                    "additionalProperties": { "type": "array", "items": { "type": "string" } }
                },
                "decks": {
                    "type": "object",
                    "additionalProperties": { "type": "array", "items": { "type": "string" } }
                },
                "security": {
                    "type": "object",
                    "description": "Per-player security stack (last element = top of stack). Optional.",
                    "additionalProperties": { "type": "array", "items": { "type": "string" } }
                },
                "digitama": {
                    "type": "object",
                    "description": "Per-player digi-egg deck (last element = top of deck). Optional.",
                    "additionalProperties": { "type": "array", "items": { "type": "string" } }
                },
                "memory": {
                    "type": "integer",
                    "description": "Optional initial memory for the active player (positive = active has memory).",
                    "minimum": -10,
                    "maximum": 10
                },
                "first_player": { "type": "integer", "minimum": 0, "maximum": 1 }
            }
        }),
    ));
    all.push(tool(
        "load_recording",
        "Load a recording paused at step 0, retaining its replay driver for stepping/scanning. Auto-detects the source: a native GameRecorder recording (JSON object with an `actions` array, replayed under Trust) vs a DCGO JSONL recording (one row per line starting with a `game_start` row, replayed under CheckThenApply — differential against the DCGO oracle). Pass recording_json (inline native object, or a string holding raw text) or recording_path. Returns { game_id, total_steps, source_format }.",
        json!({
            "type": "object",
            "properties": {
                "recording_json": { "description": "Inline native recording object, or a string holding raw native-JSON / DCGO-JSONL text." },
                "recording_path": { "type": "string", "description": "Path to a recording file (.json native or .jsonl DCGO). Auto-detected by content." },
                "source_format": {
                    "type": "string",
                    "enum": ["native", "dcgo"],
                    "description": "Optional override; skips auto-detection."
                }
            }
        }),
    ));
    all.push(tool(
        "seek",
        "Seek a recording-loaded game's cursor to step_n. Forward replays missing steps; backward uses snappy in-place reset-and-replay (native) / deterministic rebuild (DCGO). Returns a step view.",
        json!({
            "type": "object",
            "required": ["game_id", "step_n"],
            "properties": {
                "game_id": { "type": "string" },
                "step_n": { "type": "integer", "minimum": 0 }
            }
        }),
    ));
    all.push(tool(
        "list_games",
        "List every open game_id with summary metadata.",
        json!({ "type": "object", "properties": {} }),
    ));
    all.push(tool(
        "close_game",
        "Drop a game from the registry, freeing its slot.",
        json!({
            "type": "object",
            "required": ["game_id"],
            "properties": { "game_id": { "type": "string" } }
        }),
    ));

    // ── State inspection ─────────────────────────────────────────────────
    let view_param = json!({
        "type": "string",
        "enum": ["god", "player0", "player1"],
        "default": "god"
    });

    all.push(tool(
        "state",
        "Top-level game state view — phase, turn, memory, game_over, winner.",
        json!({
            "type": "object",
            "required": ["game_id"],
            "properties": {
                "game_id": { "type": "string" },
                "view": view_param.clone()
            }
        }),
    ));
    all.push(tool(
        "hand",
        "Hand contents for a player. Opponent perspective shows count only.",
        json!({
            "type": "object",
            "required": ["game_id", "player"],
            "properties": {
                "game_id": { "type": "string" },
                "player": { "type": "integer", "minimum": 0, "maximum": 1 },
                "view": view_param.clone()
            }
        }),
    ));
    all.push(tool(
        "field",
        "Battle area + breeding area for a player. Public to both players regardless of perspective.",
        json!({
            "type": "object",
            "required": ["game_id", "player"],
            "properties": {
                "game_id": { "type": "string" },
                "player": { "type": "integer", "minimum": 0, "maximum": 1 },
                "view": view_param.clone()
            }
        }),
    ));
    all.push(tool(
        "security",
        "Security stack for a player. Card IDs visible only in god view.",
        json!({
            "type": "object",
            "required": ["game_id", "player"],
            "properties": {
                "game_id": { "type": "string" },
                "player": { "type": "integer", "minimum": 0, "maximum": 1 },
                "view": view_param.clone()
            }
        }),
    ));
    all.push(tool(
        "pending_selection",
        "Current PendingSelection with decoded options, or null if none active.",
        json!({
            "type": "object",
            "required": ["game_id"],
            "properties": { "game_id": { "type": "string" } }
        }),
    ));
    all.push(tool(
        "effect_queue",
        "Queued triggered effects in resolution order.",
        json!({
            "type": "object",
            "required": ["game_id"],
            "properties": { "game_id": { "type": "string" } }
        }),
    ));
    all.push(tool(
        "events",
        "GameEvent log. Pass since_seq to filter to events newer than a sequence number.",
        json!({
            "type": "object",
            "required": ["game_id"],
            "properties": {
                "game_id": { "type": "string" },
                "since_seq": { "type": ["integer", "null"] }
            }
        }),
    ));
    all.push(tool(
        "modifiers",
        "Active modifiers on a specific permanent handle.",
        json!({
            "type": "object",
            "required": ["game_id", "handle"],
            "properties": {
                "game_id": { "type": "string" },
                "handle": {
                    "type": "object",
                    "required": ["player", "index"],
                    "properties": {
                        "player": { "type": "integer", "minimum": 0, "maximum": 1 },
                        "index": { "type": "integer", "minimum": 0 }
                    }
                }
            }
        }),
    ));
    all.push(tool(
        "inspect_card",
        "Card metadata + effect/inherited/security text. Returns null if the card is not in the loaded pool.",
        json!({
            "type": "object",
            "required": ["game_id", "card_id"],
            "properties": {
                "game_id": { "type": "string" },
                "card_id": { "type": "string" }
            }
        }),
    ));
    all.push(tool(
        "legal_actions",
        "Decoded list of every action_id legal for the given player right now.",
        json!({
            "type": "object",
            "required": ["game_id", "player"],
            "properties": {
                "game_id": { "type": "string" },
                "player": { "type": "integer", "minimum": 0, "maximum": 1 }
            }
        }),
    ));
    all.push(tool(
        "deck_cards",
        "Full card metadata for every card in both players' decks (cardCount per unique card_id). Use this once at session start to read a recording or smoke-test game in context.",
        json!({
            "type": "object",
            "required": ["game_id"],
            "properties": { "game_id": { "type": "string" } }
        }),
    ));
    all.push(tool(
        "recorded_actions",
        "Decoded action log for a game constructed from a recording. With decode_labels=true, each entry's label is computed at recording-time engine context via a temporary replay walk.",
        json!({
            "type": "object",
            "required": ["game_id"],
            "properties": {
                "game_id": { "type": "string" },
                "decode_labels": { "type": "boolean", "default": false }
            }
        }),
    ));

    // ── Replay stepping & scanning ───────────────────────────────────────
    // Only valid on games loaded via load_recording. A step view carries the
    // recorded action at the cursor, the engine's legal_now set, divergences,
    // emitted events, a zone/memory delta, and card_ids_in_play.
    all.push(tool(
        "step_forward",
        "Apply the next recorded step and return a step view (events + delta populated). No-op once complete or paused on a CheckThenApply divergence.",
        json!({
            "type": "object",
            "required": ["game_id"],
            "properties": { "game_id": { "type": "string" } }
        }),
    ));
    all.push(tool(
        "step_back",
        "Step the cursor back by one (reset-and-replay). Returns a read-only step view (no events/delta).",
        json!({
            "type": "object",
            "required": ["game_id"],
            "properties": { "game_id": { "type": "string" } }
        }),
    ));
    all.push(tool(
        "restore_checkpoint",
        "Restore the replay cursor to step_n (alias for seek). Returns a step view.",
        json!({
            "type": "object",
            "required": ["game_id", "step_n"],
            "properties": {
                "game_id": { "type": "string" },
                "step_n": { "type": "integer", "minimum": 0 }
            }
        }),
    ));
    all.push(tool(
        "replay_step_view",
        "Read-only step view at the current cursor — recorded next action, legal_now, divergences so far, and card_ids_in_play. No state change.",
        json!({
            "type": "object",
            "required": ["game_id"],
            "properties": { "game_id": { "type": "string" } }
        }),
    ));
    all.push(tool(
        "scan_divergences",
        "Walk the whole recording under CheckThenApply, collecting every differential divergence (recorded oracle action the engine's mask/actor disagrees with). With stop_at_first=true, halts at the first. Restores the caller's cursor afterward.",
        json!({
            "type": "object",
            "required": ["game_id"],
            "properties": {
                "game_id": { "type": "string" },
                "stop_at_first": { "type": "boolean", "default": false }
            }
        }),
    ));
    all.push(tool(
        "scan_fizzles",
        "Walk the recording flagging every step that emitted an EffectFizzled event — a lead for effects that silently did nothing. Restores the caller's cursor afterward.",
        json!({
            "type": "object",
            "required": ["game_id"],
            "properties": { "game_id": { "type": "string" } }
        }),
    ));
    all.push(tool(
        "scan_panics",
        "Walk the recording flagging every step the engine applied as a silent no-op (no events, no phase/memory change) — a lead for a recorded decision the engine couldn't reproduce. Restores the caller's cursor afterward.",
        json!({
            "type": "object",
            "required": ["game_id"],
            "properties": { "game_id": { "type": "string" } }
        }),
    ));

    // ── Actions ──────────────────────────────────────────────────────────
    all.push(tool(
        "play",
        "Play a card from a player's hand. Returns ok=false in ActionResult on illegal index.",
        json!({
            "type": "object",
            "required": ["game_id", "player", "hand_idx"],
            "properties": {
                "game_id": { "type": "string" },
                "player": { "type": "integer", "minimum": 0, "maximum": 1 },
                "hand_idx": { "type": "integer", "minimum": 0 }
            }
        }),
    ));
    all.push(tool(
        "resolve_selection",
        "Resolve the current pending selection with action_id (from legal_actions or the selection's options list).",
        json!({
            "type": "object",
            "required": ["game_id", "player", "action_id"],
            "properties": {
                "game_id": { "type": "string" },
                "player": { "type": "integer", "minimum": 0, "maximum": 1 },
                "action_id": { "type": "integer", "minimum": 0 }
            }
        }),
    ));
    all.push(tool(
        "end_turn",
        "End the current turn.",
        json!({
            "type": "object",
            "required": ["game_id"],
            "properties": { "game_id": { "type": "string" } }
        }),
    ));
    all.push(tool(
        "pass_turn",
        "Pass turn — roll memory to -3 for opponent.",
        json!({
            "type": "object",
            "required": ["game_id"],
            "properties": { "game_id": { "type": "string" } }
        }),
    ));
    all.push(tool(
        "move_from_breeding",
        "Move the breeding-area permanent to the battle area.",
        json!({
            "type": "object",
            "required": ["game_id", "player"],
            "properties": {
                "game_id": { "type": "string" },
                "player": { "type": "integer", "minimum": 0, "maximum": 1 }
            }
        }),
    ));
    all.push(tool(
        "step",
        "Universal action gate — submit a raw action_id through the engine's action decoder.",
        json!({
            "type": "object",
            "required": ["game_id", "action_id"],
            "properties": {
                "game_id": { "type": "string" },
                "action_id": { "type": "integer", "minimum": 0 }
            }
        }),
    ));
    all.push(tool(
        "digivolve",
        "Digivolve a card from hand onto a permanent. `host` must belong to the active player. Resolves the typed args to a legal digivolve action and dispatches via `step`.",
        json!({
            "type": "object",
            "required": ["game_id", "host", "source_hand_idx"],
            "properties": {
                "game_id": { "type": "string" },
                "host": {
                    "type": "object",
                    "required": ["player", "index"],
                    "properties": {
                        "player": { "type": "integer", "minimum": 0, "maximum": 1 },
                        "index": { "type": "integer", "minimum": 0 }
                    }
                },
                "source_hand_idx": { "type": "integer", "minimum": 0 }
            }
        }),
    ));
    all.push(tool(
        "attack",
        "Declare an attack. `target` is either a permanent handle (battle-attack) or the literal string \"security\" (security-attack).",
        json!({
            "type": "object",
            "required": ["game_id", "attacker", "target"],
            "properties": {
                "game_id": { "type": "string" },
                "attacker": {
                    "type": "object",
                    "required": ["player", "index"],
                    "properties": {
                        "player": { "type": "integer", "minimum": 0, "maximum": 1 },
                        "index": { "type": "integer", "minimum": 0 }
                    }
                },
                "target": {
                    "oneOf": [
                        { "type": "string", "enum": ["security"] },
                        {
                            "type": "object",
                            "required": ["player", "index"],
                            "properties": {
                                "player": { "type": "integer", "minimum": 0, "maximum": 1 },
                                "index": { "type": "integer", "minimum": 0 }
                            }
                        }
                    ]
                }
            }
        }),
    ));

    all
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
    })
}

/// Dispatch a tool call by name. Tool-level errors are returned as
/// `{ ok: false, error: "..." }` inside the result envelope — only
/// protocol violations (malformed args) return Err.
pub fn dispatch(
    name: &str,
    args: Value,
    registry: &mut GameRegistry,
    card_data: &HashMap<String, CardData>,
) -> Result<Value, String> {
    match name {
        "new_game_from_decks" => tool_new_game_from_decks(args, registry, card_data),
        "new_game_debug" => tool_new_game_debug(args, registry, card_data),
        "load_recording" => tool_load_recording(args, registry, card_data),
        "seek" => tool_seek(args, registry),
        "step_forward" => tool_step_forward(args, registry),
        "step_back" => tool_step_back(args, registry),
        "restore_checkpoint" => tool_seek(args, registry),
        "replay_step_view" => tool_replay_step_view(args, registry),
        "scan_divergences" => tool_scan_divergences(args, registry),
        "scan_fizzles" => tool_scan_fizzles(args, registry),
        "scan_panics" => tool_scan_panics(args, registry),
        "list_games" => Ok(tool_list_games(registry)),
        "close_game" => tool_close_game(args, registry),

        "state" => view_call(args, registry, |g, view| {
            Ok(json!(g.state(view)))
        }),
        "hand" => view_call_with_player(args, registry, |g, player, view| {
            Ok(json!(g.hand(player, view)))
        }),
        "field" => view_call_with_player(args, registry, |g, player, view| {
            Ok(json!(g.field(player, view)))
        }),
        "security" => view_call_with_player(args, registry, |g, player, view| {
            Ok(json!(g.security(player, view)))
        }),
        "pending_selection" => simple_view(args, registry, |g| {
            Ok(json!(g.pending_selection()))
        }),
        "effect_queue" => simple_view(args, registry, |g| Ok(json!(g.effect_queue()))),
        "events" => tool_events(args, registry),
        "modifiers" => tool_modifiers(args, registry),
        "inspect_card" => tool_inspect_card(args, registry),
        "legal_actions" => tool_legal_actions(args, registry),
        "deck_cards" => tool_deck_cards(args, registry),
        "recorded_actions" => tool_recorded_actions(args, registry),

        "play" => tool_play(args, registry),
        "resolve_selection" => tool_resolve_selection(args, registry),
        "end_turn" => tool_simple_mut(args, registry, |g| json!(g.end_turn())),
        "pass_turn" => tool_simple_mut(args, registry, |g| json!(g.pass_turn())),
        "move_from_breeding" => tool_move_from_breeding(args, registry),
        "step" => tool_step(args, registry),
        "digivolve" => tool_digivolve(args, registry),
        "attack" => tool_attack(args, registry),

        other => Err(format!("unknown tool: {}", other)),
    }
}

// ── argument helpers ─────────────────────────────────────────────────────

fn get_str<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    v.get(key)
        .and_then(|x| x.as_str())
        .ok_or_else(|| format!("missing or non-string arg: {}", key))
}

fn get_u8(v: &Value, key: &str) -> Result<u8, String> {
    v.get(key)
        .and_then(|x| x.as_u64())
        .map(|x| x as u8)
        .ok_or_else(|| format!("missing or non-integer arg: {}", key))
}

fn get_usize(v: &Value, key: &str) -> Result<usize, String> {
    v.get(key)
        .and_then(|x| x.as_u64())
        .map(|x| x as usize)
        .ok_or_else(|| format!("missing or non-integer arg: {}", key))
}

fn get_u16(v: &Value, key: &str) -> Result<u16, String> {
    v.get(key)
        .and_then(|x| x.as_u64())
        .map(|x| x as u16)
        .ok_or_else(|| format!("missing or non-integer arg: {}", key))
}

fn get_u32(v: &Value, key: &str) -> Result<u32, String> {
    v.get(key)
        .and_then(|x| x.as_u64())
        .map(|x| x as u32)
        .ok_or_else(|| format!("missing or non-integer arg: {}", key))
}

fn get_string_array(v: &Value, key: &str) -> Result<Vec<String>, String> {
    v.get(key)
        .and_then(|x| x.as_array())
        .ok_or_else(|| format!("missing or non-array arg: {}", key))?
        .iter()
        .map(|s| {
            s.as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| format!("non-string entry in {}", key))
        })
        .collect()
}

fn view_from_args(args: &Value) -> Perspective {
    match args.get("view").and_then(|v| v.as_str()) {
        Some("player0") => Perspective::Player(0),
        Some("player1") => Perspective::Player(1),
        _ => Perspective::God,
    }
}

fn err_value(msg: impl Into<String>) -> Value {
    json!({ "ok": false, "error": msg.into() })
}

fn registry_err_to_value(e: RegistryError) -> Value {
    err_value(e.to_string())
}

// ── lifecycle ────────────────────────────────────────────────────────────

fn tool_new_game_from_decks(
    args: Value,
    registry: &mut GameRegistry,
    card_data: &HashMap<String, CardData>,
) -> Result<Value, String> {
    let deck1 = get_string_array(&args, "deck1")?;
    let deck2 = get_string_array(&args, "deck2")?;
    let seed = args.get("seed").and_then(|v| v.as_u64());
    match LiveGame::from_decks(deck1, deck2, seed, card_data) {
        Ok(lg) => match registry.insert(lg) {
            Ok(id) => Ok(json!({ "ok": true, "game_id": id })),
            Err(e) => Ok(registry_err_to_value(e)),
        },
        Err(e) => Ok(err_value(e.to_string())),
    }
}

fn tool_new_game_debug(
    args: Value,
    registry: &mut GameRegistry,
    card_data: &HashMap<String, CardData>,
) -> Result<Value, String> {
    let first_player = get_u8(&args, "first_player")?;
    // Required zone map (errors if missing or wrong shape).
    let parse_zone_map = |key: &str| -> Result<HashMap<u8, Vec<String>>, String> {
        let obj = args
            .get(key)
            .and_then(|v| v.as_object())
            .ok_or_else(|| format!("missing or non-object arg: {}", key))?;
        let mut out = HashMap::new();
        for (k, v) in obj {
            let pid: u8 = k
                .parse()
                .map_err(|_| format!("{} key '{}' is not a u8", key, k))?;
            let cards = v
                .as_array()
                .ok_or_else(|| format!("{}.{} must be an array", key, k))?
                .iter()
                .map(|s| s.as_str().map(String::from).ok_or_else(|| format!("non-string in {}.{}", key, k)))
                .collect::<Result<Vec<_>, _>>()?;
            out.insert(pid, cards);
        }
        Ok(out)
    };
    // Optional zone map (returns empty HashMap if missing, errors only on bad shape).
    let parse_optional_zone_map = |key: &str| -> Result<HashMap<u8, Vec<String>>, String> {
        let Some(obj) = args.get(key) else { return Ok(HashMap::new()); };
        if obj.is_null() {
            return Ok(HashMap::new());
        }
        let obj = obj
            .as_object()
            .ok_or_else(|| format!("{} must be an object or omitted", key))?;
        let mut out = HashMap::new();
        for (k, v) in obj {
            let pid: u8 = k
                .parse()
                .map_err(|_| format!("{} key '{}' is not a u8", key, k))?;
            let cards = v
                .as_array()
                .ok_or_else(|| format!("{}.{} must be an array", key, k))?
                .iter()
                .map(|s| s.as_str().map(String::from).ok_or_else(|| format!("non-string in {}.{}", key, k)))
                .collect::<Result<Vec<_>, _>>()?;
            out.insert(pid, cards);
        }
        Ok(out)
    };
    let hands = parse_zone_map("hands")?;
    let decks = parse_zone_map("decks")?;
    let securities = parse_optional_zone_map("security")?;
    let digitamas = parse_optional_zone_map("digitama")?;
    let initial_memory: Option<i16> = match args.get("memory") {
        None => None,
        Some(v) if v.is_null() => None,
        Some(v) => Some(
            v.as_i64()
                .and_then(|n| i16::try_from(n).ok())
                .ok_or_else(|| "memory must be an integer in i16 range".to_string())?,
        ),
    };
    match LiveGame::from_debug_with_zones(
        hands,
        decks,
        securities,
        digitamas,
        first_player,
        initial_memory,
        card_data.clone(),
    ) {
        Ok(lg) => match registry.insert(lg) {
            Ok(id) => Ok(json!({ "ok": true, "game_id": id })),
            Err(e) => Ok(registry_err_to_value(e)),
        },
        Err(e) => Ok(err_value(e.to_string())),
    }
}

/// A native recording object or raw recording text awaiting format detection.
enum RecordingInput {
    /// An inline native `GameRecorder` recording object.
    NativeValue(Value),
    /// Raw text — either native JSON or DCGO JSONL, decided by content.
    Text(String),
}

fn tool_load_recording(
    args: Value,
    registry: &mut GameRegistry,
    card_data: &HashMap<String, CardData>,
) -> Result<Value, String> {
    let explicit_format = args.get("source_format").and_then(|v| v.as_str());

    let input = if let Some(inline) = args.get("recording_json") {
        if inline.is_null() {
            return Err("recording_json is null".into());
        }
        match inline {
            Value::String(s) => RecordingInput::Text(s.clone()),
            other => RecordingInput::NativeValue(other.clone()),
        }
    } else if let Some(path) = args.get("recording_path").and_then(|v| v.as_str()) {
        let text =
            std::fs::read_to_string(path).map_err(|e| format!("reading {}: {}", path, e))?;
        RecordingInput::Text(text)
    } else {
        return Err("provide recording_json or recording_path".into());
    };

    // Resolve (format, constructed LiveGame). Detection: a native recording is
    // a single JSON object carrying `actions`/`initial_state`; a DCGO recording
    // is JSONL (one object per line, leading `game_start` row).
    let (lg_result, source_format): (Result<LiveGame, LiveGameError>, &'static str) =
        match (explicit_format, input) {
            (Some("native"), RecordingInput::NativeValue(v)) => {
                (LiveGame::from_recording(v, card_data), "native")
            }
            (Some("native"), RecordingInput::Text(t)) => {
                let v: Value = serde_json::from_str(&t)
                    .map_err(|e| format!("parsing native recording: {}", e))?;
                (LiveGame::from_recording(v, card_data), "native")
            }
            (Some("dcgo"), RecordingInput::Text(t)) => match parse_jsonl(&t) {
                Ok(rec) => (LiveGame::from_dcgo_recording(rec, card_data), "dcgo"),
                Err(e) => return Ok(err_value(format!("parsing DCGO recording: {}", e))),
            },
            (Some("dcgo"), RecordingInput::NativeValue(_)) => {
                return Err(
                    "source_format=dcgo requires JSONL text (recording_json string or recording_path)"
                        .into(),
                );
            }
            (Some(other), _) => {
                return Err(format!("unknown source_format: {}", other));
            }
            // Auto-detect.
            (None, RecordingInput::NativeValue(v)) => {
                (LiveGame::from_recording(v, card_data), "native")
            }
            (None, RecordingInput::Text(t)) => {
                let as_native: Option<Value> = serde_json::from_str(&t)
                    .ok()
                    .filter(|v: &Value| v.get("actions").is_some() || v.get("initial_state").is_some());
                if let Some(v) = as_native {
                    (LiveGame::from_recording(v, card_data), "native")
                } else {
                    match parse_jsonl(&t) {
                        Ok(rec) => (LiveGame::from_dcgo_recording(rec, card_data), "dcgo"),
                        Err(e) => {
                            return Ok(err_value(format!(
                                "unrecognized recording: not a native JSON recording, and DCGO JSONL parse failed: {}",
                                e
                            )))
                        }
                    }
                }
            }
        };

    match lg_result {
        Ok(lg) => {
            let total_steps = lg.replay_total_steps();
            match registry.insert(lg) {
                Ok(id) => Ok(json!({
                    "ok": true,
                    "game_id": id,
                    "total_steps": total_steps,
                    "source_format": source_format,
                })),
                Err(e) => Ok(registry_err_to_value(e)),
            }
        }
        Err(e) => Ok(err_value(e.to_string())),
    }
}

/// Shared driver for the replay-stepping tools that mutate the cursor and
/// return a serializable payload under `key`.
fn replay_mut_call<T: serde::Serialize>(
    args: Value,
    registry: &mut GameRegistry,
    key: &str,
    f: impl FnOnce(&mut LiveGame) -> Result<T, LiveGameError>,
) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    match registry.get_mut(&game_id) {
        Ok(g) => match f(g) {
            Ok(out) => {
                let mut obj = serde_json::Map::new();
                obj.insert("ok".into(), json!(true));
                obj.insert(
                    key.to_string(),
                    serde_json::to_value(out).map_err(|e| e.to_string())?,
                );
                Ok(Value::Object(obj))
            }
            Err(e) => Ok(err_value(e.to_string())),
        },
        Err(e) => Ok(registry_err_to_value(e)),
    }
}

fn tool_seek(args: Value, registry: &mut GameRegistry) -> Result<Value, String> {
    let step_n = get_u32(&args, "step_n")?;
    replay_mut_call(args, registry, "view", move |g| g.replay_seek(step_n))
}

fn tool_step_forward(args: Value, registry: &mut GameRegistry) -> Result<Value, String> {
    replay_mut_call(args, registry, "view", |g| g.step_forward())
}

fn tool_step_back(args: Value, registry: &mut GameRegistry) -> Result<Value, String> {
    replay_mut_call(args, registry, "view", |g| g.step_back())
}

fn tool_replay_step_view(args: Value, registry: &GameRegistry) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    match registry.get(&game_id) {
        Ok(g) => match g.replay_step_view() {
            Ok(view) => Ok(json!({ "ok": true, "view": view })),
            Err(e) => Ok(err_value(e.to_string())),
        },
        Err(e) => Ok(registry_err_to_value(e)),
    }
}

fn tool_scan_divergences(args: Value, registry: &mut GameRegistry) -> Result<Value, String> {
    let stop_at_first = args
        .get("stop_at_first")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    replay_mut_call(args, registry, "scan", move |g| {
        g.scan_divergences(stop_at_first)
    })
}

fn tool_scan_fizzles(args: Value, registry: &mut GameRegistry) -> Result<Value, String> {
    replay_mut_call(args, registry, "scan", |g| g.scan_fizzles())
}

fn tool_scan_panics(args: Value, registry: &mut GameRegistry) -> Result<Value, String> {
    replay_mut_call(args, registry, "scan", |g| g.scan_panics())
}

fn tool_list_games(registry: &GameRegistry) -> Value {
    let games: Vec<Value> = registry
        .iter()
        .map(|(id, g)| {
            json!({
                "game_id": id,
                "turn_count": g.game.turn_count,
                "phase": format!("{:?}", g.game.current_phase),
                "game_over": g.game.game_over,
                "winner": g.game.winner,
            })
        })
        .collect();
    json!({ "ok": true, "games": games, "limit": registry.limit() })
}

fn tool_close_game(args: Value, registry: &mut GameRegistry) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    match registry.remove(&game_id) {
        Ok(()) => Ok(json!({ "ok": true })),
        Err(e) => Ok(registry_err_to_value(e)),
    }
}

// ── view dispatchers ────────────────────────────────────────────────────

fn view_call(
    args: Value,
    registry: &GameRegistry,
    f: impl FnOnce(&LiveGame, Perspective) -> Result<Value, String>,
) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let view = view_from_args(&args);
    match registry.get(&game_id) {
        Ok(g) => f(g, view),
        Err(e) => Ok(registry_err_to_value(e)),
    }
}

fn view_call_with_player(
    args: Value,
    registry: &GameRegistry,
    f: impl FnOnce(&LiveGame, u8, Perspective) -> Result<Value, String>,
) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let player = get_u8(&args, "player")?;
    let view = view_from_args(&args);
    match registry.get(&game_id) {
        Ok(g) => f(g, player, view),
        Err(e) => Ok(registry_err_to_value(e)),
    }
}

fn simple_view(
    args: Value,
    registry: &GameRegistry,
    f: impl FnOnce(&LiveGame) -> Result<Value, String>,
) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    match registry.get(&game_id) {
        Ok(g) => f(g),
        Err(e) => Ok(registry_err_to_value(e)),
    }
}

fn tool_events(args: Value, registry: &GameRegistry) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let since_seq = args.get("since_seq").and_then(|v| v.as_u64());
    match registry.get(&game_id) {
        Ok(g) => Ok(json!(g.events(since_seq))),
        Err(e) => Ok(registry_err_to_value(e)),
    }
}

fn tool_modifiers(args: Value, registry: &GameRegistry) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let h = args
        .get("handle")
        .ok_or_else(|| "missing 'handle'".to_string())?;
    let player = get_u8(h, "player")?;
    let index = get_usize(h, "index")? as u8;
    let handle = PermanentHandle { player, index };
    match registry.get(&game_id) {
        Ok(g) => Ok(json!(g.modifiers(handle))),
        Err(e) => Ok(registry_err_to_value(e)),
    }
}

fn tool_inspect_card(args: Value, registry: &GameRegistry) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let card_id = get_str(&args, "card_id")?.to_string();
    match registry.get(&game_id) {
        Ok(g) => Ok(match g.inspect_card(&card_id) {
            Some(ins) => json!(ins),
            None => json!(null),
        }),
        Err(e) => Ok(registry_err_to_value(e)),
    }
}

fn tool_legal_actions(args: Value, registry: &GameRegistry) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let player = get_u8(&args, "player")?;
    match registry.get(&game_id) {
        Ok(g) => Ok(json!(g.legal_actions(player))),
        Err(e) => Ok(registry_err_to_value(e)),
    }
}

fn tool_deck_cards(args: Value, registry: &GameRegistry) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    match registry.get(&game_id) {
        Ok(g) => Ok(json!(g.deck_cards())),
        Err(e) => Ok(registry_err_to_value(e)),
    }
}

fn tool_recorded_actions(args: Value, registry: &GameRegistry) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let decode_labels = args
        .get("decode_labels")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    match registry.get(&game_id) {
        Ok(g) => match g.recorded_actions(decode_labels) {
            Some(actions) => Ok(json!({ "ok": true, "actions": actions })),
            None => Ok(err_value(
                "this game was not constructed from a recording — use load_recording first",
            )),
        },
        Err(e) => Ok(registry_err_to_value(e)),
    }
}

// ── actions ──────────────────────────────────────────────────────────────

fn tool_simple_mut(
    args: Value,
    registry: &mut GameRegistry,
    f: impl FnOnce(&mut LiveGame) -> Value,
) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    match registry.get_mut(&game_id) {
        Ok(g) => Ok(f(g)),
        Err(e) => Ok(registry_err_to_value(e)),
    }
}

fn tool_play(args: Value, registry: &mut GameRegistry) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let player = get_u8(&args, "player")?;
    let hand_idx = get_usize(&args, "hand_idx")?;
    match registry.get_mut(&game_id) {
        Ok(g) => Ok(json!(g.play(player, hand_idx))),
        Err(e) => Ok(registry_err_to_value(e)),
    }
}

fn tool_resolve_selection(args: Value, registry: &mut GameRegistry) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let player = get_u8(&args, "player")?;
    let action_id = get_u16(&args, "action_id")?;
    match registry.get_mut(&game_id) {
        Ok(g) => Ok(json!(g.resolve_selection(player, action_id))),
        Err(e) => Ok(registry_err_to_value(e)),
    }
}

fn tool_move_from_breeding(args: Value, registry: &mut GameRegistry) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let player = get_u8(&args, "player")?;
    match registry.get_mut(&game_id) {
        Ok(g) => Ok(json!(g.move_from_breeding(player))),
        Err(e) => Ok(registry_err_to_value(e)),
    }
}

fn tool_step(args: Value, registry: &mut GameRegistry) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let action_id = get_u16(&args, "action_id")?;
    match registry.get_mut(&game_id) {
        Ok(g) => Ok(json!(g.step(action_id))),
        Err(e) => Ok(registry_err_to_value(e)),
    }
}

fn parse_handle(v: &Value, key: &str) -> Result<digimon_engine::permanent::PermanentHandle, String> {
    let obj = v
        .get(key)
        .and_then(|x| x.as_object())
        .ok_or_else(|| format!("missing or non-object arg: {}", key))?;
    let player = obj
        .get("player")
        .and_then(|x| x.as_u64())
        .ok_or_else(|| format!("{}.player missing or not an integer", key))? as u8;
    let index = obj
        .get("index")
        .and_then(|x| x.as_u64())
        .ok_or_else(|| format!("{}.index missing or not an integer", key))? as u8;
    Ok(digimon_engine::permanent::PermanentHandle { player, index })
}

fn tool_digivolve(args: Value, registry: &mut GameRegistry) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let host = parse_handle(&args, "host")?;
    let source_hand_idx = get_usize(&args, "source_hand_idx")?;
    match registry.get_mut(&game_id) {
        Ok(g) => Ok(json!(g.digivolve(host, source_hand_idx))),
        Err(e) => Ok(registry_err_to_value(e)),
    }
}

fn tool_attack(args: Value, registry: &mut GameRegistry) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let attacker = parse_handle(&args, "attacker")?;
    // target = "security" | { player, index }
    let target_val = args
        .get("target")
        .ok_or_else(|| "missing arg: target".to_string())?;
    let target = if target_val.as_str() == Some("security") {
        digimon_engine::live_game::AttackTarget::Security
    } else if target_val.is_object() {
        digimon_engine::live_game::AttackTarget::Permanent(parse_handle(&args, "target")?)
    } else {
        return Err("target must be either \"security\" or {player, index}".into());
    };
    match registry.get_mut(&game_id) {
        Ok(g) => Ok(json!(g.attack(attacker, target))),
        Err(e) => Ok(registry_err_to_value(e)),
    }
}

// ── tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_includes_33_tools() {
        let tools = list();
        // Expected count: 6 lifecycle + 12 state (10 originals +
        // deck_cards + recorded_actions) + 7 replay stepping/scanning
        // (step_forward/step_back/restore_checkpoint/replay_step_view +
        // scan_divergences/scan_fizzles/scan_panics) + 8 action (6 originals
        // + digivolve + attack) = 33.
        assert_eq!(tools.len(), 33, "tool count drifted");
    }

    #[test]
    fn deck_cards_and_recorded_actions_are_advertised() {
        let names: Vec<String> = list()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect();
        assert!(names.iter().any(|n| n == "deck_cards"));
        assert!(names.iter().any(|n| n == "recorded_actions"));
    }

    #[test]
    fn replay_stepping_and_scanner_tools_are_advertised() {
        let names: Vec<String> = list()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect();
        for t in [
            "step_forward",
            "step_back",
            "restore_checkpoint",
            "replay_step_view",
            "scan_divergences",
            "scan_fizzles",
            "scan_panics",
        ] {
            assert!(names.iter().any(|n| n == t), "tool {t} not advertised");
        }
    }

    #[test]
    fn each_tool_has_required_fields() {
        for t in list() {
            assert!(t.get("name").and_then(|v| v.as_str()).is_some());
            assert!(t.get("description").and_then(|v| v.as_str()).is_some());
            assert!(t.get("inputSchema").is_some());
        }
    }

    #[test]
    fn dispatch_unknown_tool_returns_err() {
        let mut reg = GameRegistry::new(8);
        let cd = HashMap::new();
        assert!(dispatch("nonexistent", json!({}), &mut reg, &cd).is_err());
    }

    #[test]
    fn list_games_empty_registry() {
        let reg = GameRegistry::new(8);
        let v = tool_list_games(&reg);
        assert_eq!(v["games"].as_array().unwrap().len(), 0);
        assert_eq!(v["limit"], 8);
    }

    // ── replay-stepping dispatch tests (Group 7.6) ───────────────────────

    fn minimal_db() -> HashMap<String, CardData> {
        let json = r#"{
            "ST1-01": {
                "card_id": "ST1-01", "card_name_eng": "Botamon",
                "card_effect_class_name": "ST1_01", "play_cost": 0, "dp": -1,
                "level": 2, "card_kind": 3, "rarity": 0, "card_colors": [0],
                "type_eng": ["Lesser"], "form_eng": ["In-Training"], "attribute_eng": [],
                "effect_description_eng": "", "inherited_effect_description_eng": "",
                "security_effect_description_eng": "", "evo_costs": []
            },
            "ST1-03": {
                "card_id": "ST1-03", "card_name_eng": "Koromon",
                "card_effect_class_name": "ST1_03", "play_cost": 3, "dp": 2000,
                "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [0],
                "type_eng": ["Lesser"], "form_eng": ["Rookie"], "attribute_eng": ["Free"],
                "effect_description_eng": "", "inherited_effect_description_eng": "",
                "security_effect_description_eng": "", "evo_costs": []
            }
        }"#;
        CardData::load_from_str(json).unwrap()
    }

    fn small_deck() -> Vec<String> {
        std::iter::repeat("ST1-01".to_string())
            .take(5)
            .chain(std::iter::repeat("ST1-03".to_string()).take(45))
            .collect()
    }

    fn native_recording(seed: u64) -> Value {
        let db = minimal_db();
        let mut hr = digimon_engine::HeadlessRunner::new(
            small_deck(),
            small_deck(),
            &db,
            false,
            true,
            false,
            Some(seed),
        )
        .unwrap();
        hr.step(0); // mulligan keep p1
        hr.step(0); // mulligan keep p2
        for _ in 0..4 {
            hr.step(digimon_engine::action::space::PASS);
        }
        hr.get_recording().unwrap()
    }

    /// Native recording whose first non-mulligan action_id is corrupted to an
    /// always-masked value (999), so a CheckThenApply scan flags one MaskMiss.
    fn native_recording_with_illegal_action() -> Value {
        let mut rec = native_recording(11);
        if let Some(actions) = rec["actions"].as_array_mut() {
            for a in actions.iter_mut() {
                if a["phase"].as_str() != Some("Mulligan") {
                    a["action_id"] = json!(999);
                    break;
                }
            }
        }
        rec
    }

    /// Minimal bot-vs-bot DCGO recording as JSONL text (game_start +
    /// two mulligan keeps + game_end).
    fn dcgo_bot_jsonl() -> String {
        let deck = small_deck();
        let start = json!({
            "v": 1, "type": "game_start", "game_id": "bot", "timestamp": "t",
            "my_player_id": 0, "is_ai": true,
            "my_deck_post_shuffle": deck, "opp_deck_post_shuffle": deck
        });
        let a0 = json!({"type":"action","step":0,"actor":0,"action_id":0,"phase":"Mulligan","source":"mulligan"});
        let a1 = json!({"type":"action","step":1,"actor":1,"action_id":0,"phase":"Mulligan","source":"mulligan"});
        let end = json!({"type":"game_end","winner":0,"reason":"win","total_steps":2});
        format!("{start}\n{a0}\n{a1}\n{end}\n")
    }

    fn load_game(recording: Value) -> (GameRegistry, String, Value) {
        let db = minimal_db();
        let mut reg = GameRegistry::new(8);
        let resp = dispatch(
            "load_recording",
            json!({ "recording_json": recording }),
            &mut reg,
            &db,
        )
        .unwrap();
        assert_eq!(resp["ok"], true, "load_recording failed: {resp}");
        let id = resp["game_id"].as_str().unwrap().to_string();
        (reg, id, resp)
    }

    #[test]
    fn load_recording_native_reports_total_and_format() {
        let (_reg, _id, resp) = load_game(native_recording(11));
        assert_eq!(resp["source_format"], "native");
        // 4 passes replay (mulligan baked into initial_state).
        assert_eq!(resp["total_steps"], 4);
    }

    #[test]
    fn load_recording_autodetects_dcgo_jsonl() {
        let db = minimal_db();
        let mut reg = GameRegistry::new(8);
        // DCGO JSONL is passed as a string in recording_json.
        let resp = dispatch(
            "load_recording",
            json!({ "recording_json": dcgo_bot_jsonl() }),
            &mut reg,
            &db,
        )
        .unwrap();
        assert_eq!(resp["ok"], true, "dcgo load failed: {resp}");
        assert_eq!(resp["source_format"], "dcgo");
        assert_eq!(resp["total_steps"], 2);
    }

    #[test]
    fn step_forward_view_carries_action_legal_set_and_delta() {
        let db = minimal_db();
        let (mut reg, id, _) = load_game(native_recording(11));
        let resp = dispatch("step_forward", json!({ "game_id": id }), &mut reg, &db).unwrap();
        assert_eq!(resp["ok"], true, "{resp}");
        let view = &resp["view"];
        assert_eq!(view["cursor"], 1);
        assert!(view["recorded"].is_object(), "recorded action missing");
        assert!(
            !view["legal_now"].as_array().unwrap().is_empty(),
            "legal_now should list the engine's offered actions"
        );
        assert!(view["delta"].is_object(), "forward step should carry a delta");
    }

    #[test]
    fn step_forward_then_back_round_trips_cursor() {
        let db = minimal_db();
        let (mut reg, id, _) = load_game(native_recording(11));
        dispatch("step_forward", json!({ "game_id": id }), &mut reg, &db).unwrap();
        let fwd = dispatch("step_forward", json!({ "game_id": id }), &mut reg, &db).unwrap();
        assert_eq!(fwd["view"]["cursor"], 2);
        let back = dispatch("step_back", json!({ "game_id": id }), &mut reg, &db).unwrap();
        assert_eq!(back["ok"], true, "{back}");
        assert_eq!(back["view"]["cursor"], 1);
    }

    #[test]
    fn seek_and_restore_checkpoint_move_the_cursor() {
        let db = minimal_db();
        let (mut reg, id, _) = load_game(native_recording(11));
        let seek = dispatch(
            "seek",
            json!({ "game_id": id, "step_n": 3 }),
            &mut reg,
            &db,
        )
        .unwrap();
        assert_eq!(seek["view"]["cursor"], 3);
        let restore = dispatch(
            "restore_checkpoint",
            json!({ "game_id": id, "step_n": 1 }),
            &mut reg,
            &db,
        )
        .unwrap();
        assert_eq!(restore["view"]["cursor"], 1);
    }

    #[test]
    fn scan_divergences_finds_masked_out_action() {
        let db = minimal_db();
        let (mut reg, id, _) = load_game(native_recording_with_illegal_action());
        // Advance the cursor first; the scan must not perturb it.
        dispatch("step_forward", json!({ "game_id": id }), &mut reg, &db).unwrap();

        let resp = dispatch(
            "scan_divergences",
            json!({ "game_id": id, "stop_at_first": true }),
            &mut reg,
            &db,
        )
        .unwrap();
        assert_eq!(resp["ok"], true, "{resp}");
        let findings = resp["scan"]["findings"].as_array().unwrap();
        assert!(!findings.is_empty(), "expected a masked-action divergence");
        assert_eq!(findings[0]["action_id"], 999);
        assert!(
            findings[0]["divergence"].is_object(),
            "divergence finding should carry the structured Divergence"
        );

        // The scan restored the cursor (still at 1 from the step_forward).
        let view = dispatch("replay_step_view", json!({ "game_id": id }), &mut reg, &db).unwrap();
        assert_eq!(view["view"]["cursor"], 1, "scan perturbed the cursor");
    }

    #[test]
    fn stepping_a_non_recording_game_returns_ok_false() {
        let db = minimal_db();
        let mut reg = GameRegistry::new(8);
        let made = dispatch(
            "new_game_from_decks",
            json!({ "deck1": small_deck(), "deck2": small_deck(), "seed": 1 }),
            &mut reg,
            &db,
        )
        .unwrap();
        let id = made["game_id"].as_str().unwrap().to_string();
        let resp = dispatch("step_forward", json!({ "game_id": id }), &mut reg, &db).unwrap();
        assert_eq!(resp["ok"], false);
        assert!(resp["error"].as_str().unwrap().contains("not constructed from a recording"));
    }
}
