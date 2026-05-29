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
use digimon_engine::live_game::LiveGame;
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
        "Construct a fresh game from explicit hand + deck per player. No shuffling. Mirrors DebugRunnerBuilder.",
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
                "first_player": { "type": "integer", "minimum": 0, "maximum": 1 }
            }
        }),
    ));
    all.push(tool(
        "load_recording",
        "Load a GameRecorder recording, paused at step 0. Pass recording_json (inline) or recording_path.",
        json!({
            "type": "object",
            "properties": {
                "recording_json": {},
                "recording_path": { "type": "string" }
            }
        }),
    ));
    all.push(tool(
        "seek",
        "Fast-forward a recording-loaded game to step_n. Backward seek rebuilds and re-walks.",
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
        "list_games" => Ok(tool_list_games(registry)),
        "close_game" => tool_close_game(args, registry),

        "state" => view_call(args, registry, |g, view| Ok(json!(g.state(view)))),
        "hand" => view_call_with_player(args, registry, |g, player, view| {
            Ok(json!(g.hand(player, view)))
        }),
        "field" => view_call_with_player(args, registry, |g, player, view| {
            Ok(json!(g.field(player, view)))
        }),
        "security" => view_call_with_player(args, registry, |g, player, view| {
            Ok(json!(g.security(player, view)))
        }),
        "pending_selection" => simple_view(args, registry, |g| Ok(json!(g.pending_selection()))),
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
                .map(|s| {
                    s.as_str()
                        .map(String::from)
                        .ok_or_else(|| format!("non-string in {}.{}", key, k))
                })
                .collect::<Result<Vec<_>, _>>()?;
            out.insert(pid, cards);
        }
        Ok(out)
    };
    let hands = parse_zone_map("hands")?;
    let decks = parse_zone_map("decks")?;
    match LiveGame::from_debug(hands, decks, first_player, card_data.clone()) {
        Ok(lg) => match registry.insert(lg) {
            Ok(id) => Ok(json!({ "ok": true, "game_id": id })),
            Err(e) => Ok(registry_err_to_value(e)),
        },
        Err(e) => Ok(err_value(e.to_string())),
    }
}

fn tool_load_recording(
    args: Value,
    registry: &mut GameRegistry,
    card_data: &HashMap<String, CardData>,
) -> Result<Value, String> {
    let recording = if let Some(inline) = args.get("recording_json").cloned() {
        if inline.is_null() {
            return Err("recording_json is null".into());
        }
        inline
    } else if let Some(path) = args.get("recording_path").and_then(|v| v.as_str()) {
        let bytes = std::fs::read(path).map_err(|e| format!("reading {}: {}", path, e))?;
        serde_json::from_slice(&bytes).map_err(|e| format!("parsing {}: {}", path, e))?
    } else {
        return Err("provide recording_json or recording_path".into());
    };
    match LiveGame::from_recording(recording, card_data) {
        Ok(lg) => match registry.insert(lg) {
            Ok(id) => Ok(json!({ "ok": true, "game_id": id })),
            Err(e) => Ok(registry_err_to_value(e)),
        },
        Err(e) => Ok(err_value(e.to_string())),
    }
}

fn tool_seek(args: Value, registry: &mut GameRegistry) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let step_n = get_u32(&args, "step_n")?;
    // LiveGame post-construction doesn't keep its ReplayRunner — for v1
    // we surface a not-supported error explaining the limitation.
    let _ = (registry.get(&game_id), step_n);
    Ok(err_value(
        "seek is not supported in v1 — reconstruct via load_recording with a higher step_n",
    ))
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

fn parse_handle(
    v: &Value,
    key: &str,
) -> Result<digimon_engine::permanent::PermanentHandle, String> {
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
    fn list_includes_26_tools() {
        let tools = list();
        // Expected count: 6 lifecycle + 12 state (10 originals +
        // deck_cards + recorded_actions) + 8 action (6 originals +
        // digivolve + attack per enforce-live-game-action-contracts) = 26.
        assert_eq!(tools.len(), 26, "tool count drifted");
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
}
