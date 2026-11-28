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
use std::path::{Component, Path, PathBuf};

use digimon_engine::action::build_action_mask;
use digimon_engine::card_data::CardData;
use digimon_engine::card_registry::CardRegistry;
use digimon_engine::dcgo_recording::parse_jsonl;
use digimon_engine::inference::{
    inspect_position_with_evaluator, score_line_with_evaluator, InspectPositionParams,
    RefutationTrigger, ScoreLineParams,
};
use digimon_engine::live_game::{ActionResult, LiveGame, LiveGameError};
use digimon_engine::observation::{
    build_observation_tensor_with_mask, parse_observation_profile, ObservationProfileId,
};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::policies::greedy_action;
use digimon_engine::search::{
    decision_player, determinized_search_batched, DeterminizedConfig, EvalRequest, PolicyValueFn,
    UniformEvaluator,
};
use digimon_engine::view::{OPAQUE_PLACEHOLDER_CARD_ID, Perspective};
use serde_json::{json, Value};

use crate::registry::{GameEntry, GameRegistry, ModelRegistry, OpponentDriver, RegistryError, SessionMode};

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

    // ── Models, Valuation & Tactical Puzzle Lab (Phase 1A & 1B) ─────────
    all.push(tool(
        "load_model",
        "Load and cache an ONNX policy+value checkpoint (stateless MLP or recurrent LSTM + .value.onnx sidecar) into ModelRegistry.",
        json!({
            "type": "object",
            "required": ["model_id", "onnx_path"],
            "properties": {
                "model_id": { "type": "string" },
                "onnx_path": { "type": "string" },
                "profile": { "type": ["string", "null"] }
            }
        }),
    ));
    all.push(tool(
        "unload_model",
        "Remove a cached ONNX model from ModelRegistry.",
        json!({
            "type": "object",
            "required": ["model_id"],
            "properties": {
                "model_id": { "type": "string" }
            }
        }),
    ));
    all.push(tool(
        "list_models",
        "List every ONNX model currently cached in ModelRegistry.",
        json!({ "type": "object", "properties": {} }),
    ));
    all.push(tool(
        "inspect_position",
        "Evaluate the current decision position of a live game with a cached ONNX model (or uniform evaluator if model_id is omitted): policy head + value head + optional determinized MCTS search.",
        json!({
            "type": "object",
            "required": ["game_id"],
            "properties": {
                "game_id": { "type": "string" },
                "model_id": { "type": ["string", "null"] },
                "simulations": { "type": "integer", "minimum": 0, "default": 0 },
                "worlds": { "type": "integer", "minimum": 1, "default": 1 },
                "regime": { "type": "string", "default": "open_list" },
                "seed": { "type": "integer", "minimum": 0, "default": 0 },
                "top_k": { "type": "integer", "minimum": 1, "default": 12 },
                "expectations": { "type": ["array", "null"], "items": { "type": "object" } }
            }
        }),
    ));
    all.push(tool(
        "score_line",
        "Roll out a candidate action sequence on a cloned game without mutating the live session, scoring each step with a cached ONNX model (or uniform evaluator if model_id is omitted) and resolving opponent interrupts via refutation_triggers or bounded adversary MCTS.",
        json!({
            "type": "object",
            "required": ["game_id", "actions"],
            "properties": {
                "game_id": { "type": "string" },
                "model_id": { "type": ["string", "null"] },
                "actions": { "type": "array", "items": { "type": "integer", "minimum": 0 } },
                "adversary_sims": { "type": "integer", "minimum": 0, "default": 0 },
                "refutation_triggers": { "type": ["array", "null"], "items": { "type": "object" } }
            }
        }),
    ));
    all.push(tool(
        "inspect_stack",
        "Stack-aware permanent inspection: top card's main effect_text + printed/effective stats, under-cards' inherited_text ONLY (respecting face-down opacity), linked cards' link_text ONLY + dp_bonus, suspend/turn state, and active modifiers.",
        json!({
            "type": "object",
            "required": ["game_id", "player"],
            "properties": {
                "game_id": { "type": "string" },
                "player": { "type": "integer", "minimum": 0, "maximum": 1 },
                "zone": { "type": "string", "enum": ["battle", "breeding"], "default": "battle" },
                "slot": { "type": "integer", "minimum": 0, "default": 0 },
                "view": view_param
            }
        }),
    ));
    all.push(tool(
        "load_puzzle",
        "Load a studio-puzzle-v1 or search-exam-v1 fixture into a live game session in either 'author' (God view) or 'play' (seat-locked Perspective + optional auto-opponent) mode.",
        json!({
            "type": "object",
            "properties": {
                "puzzle_path": { "type": ["string", "null"] },
                "puzzle_json": { "description": "Inline studio-puzzle-v1 / search-exam-v1 object or JSON string." },
                "mode": { "type": "string", "enum": ["author", "play"], "default": "author" },
                "opponent_driver": {
                    "description": "Optional auto-opponent in play mode: 'greedy', 'mcts', 'onnx:<model_id>', or { kind: 'greedy'|'onnx'|'mcts', model_id?, simulations?, worlds?, seed? }."
                }
            }
        }),
    ));
    all.push(tool(
        "save_puzzle",
        "Save an author-mode game session as canonical studio-puzzle-v1 JSON under qa/puzzles/ (and optionally export search-exam-v1 JSON).",
        json!({
            "type": "object",
            "required": ["game_id", "puzzle_path", "id", "title"],
            "properties": {
                "game_id": { "type": "string" },
                "puzzle_path": { "type": "string" },
                "id": { "type": "string" },
                "title": { "type": "string" },
                "description": { "type": ["string", "null"] },
                "folder": { "type": ["string", "null"] },
                "tags": { "type": ["array", "null"], "items": { "type": "string" } },
                "seat": { "type": ["integer", "null"], "minimum": 0, "maximum": 2 },
                "lines": { "type": ["array", "null"], "items": { "type": "object" } },
                "expectations": { "type": ["array", "null"], "items": { "type": "object" } },
                "assertions": { "type": ["object", "null"] },
                "rl_goal": { "type": ["object", "null"] },
                "refutation_triggers": { "type": ["array", "null"], "items": { "type": "object" } },
                "export_search_exam_path": { "type": ["string", "null"] }
            }
        }),
    ));
    all.push(tool(
        "verify_puzzle",
        "Verify a studio-puzzle-v1 or search-exam-v1 puzzle across multiple seeds: target lines must achieve win/win_assertions even against adversary interrupts, and avoid lines must fail/be refuted.",
        json!({
            "type": "object",
            "properties": {
                "puzzle_path": { "type": ["string", "null"] },
                "puzzle_json": { "description": "Inline studio-puzzle-v1 / search-exam-v1 object or JSON string." },
                "model_id": { "type": ["string", "null"] },
                "adversary_sims": { "type": "integer", "minimum": 0, "default": 64 },
                "seeds": { "type": "integer", "minimum": 1, "default": 4 }
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

        "state" => view_call(args, registry, |g, view| Ok(json!(g.state(view)))),
        "hand" => view_call_with_player(args, registry, |g, player, view| {
            Ok(json!(g.hand(player, view)))
        }),
        "field" => tool_field(args, registry),
        "security" => view_call_with_player(args, registry, |g, player, view| {
            Ok(json!(g.security(player, view)))
        }),
        "pending_selection" => tool_pending_selection(args, registry),
        "effect_queue" => simple_view(args, registry, |g| Ok(json!(g.effect_queue()))),
        "events" => tool_events(args, registry),
        "modifiers" => tool_modifiers(args, registry),
        "inspect_card" => tool_inspect_card(args, registry),
        "legal_actions" => tool_legal_actions(args, registry),
        "deck_cards" => tool_deck_cards(args, registry),
        "recorded_actions" => tool_recorded_actions(args, registry),

        "play" => tool_play(args, registry, card_data),
        "resolve_selection" => tool_resolve_selection(args, registry, card_data),
        "end_turn" => tool_end_turn(args, registry, card_data),
        "pass_turn" => tool_pass_turn(args, registry, card_data),
        "move_from_breeding" => tool_move_from_breeding(args, registry, card_data),
        "step" => tool_step(args, registry, card_data),
        "digivolve" => tool_digivolve(args, registry, card_data),
        "attack" => tool_attack(args, registry, card_data),

        "load_model" => tool_load_model(args, registry),
        "unload_model" => tool_unload_model(args, registry),
        "list_models" => Ok(tool_list_models(registry)),
        "inspect_position" => tool_inspect_position(args, registry, card_data),
        "score_line" => tool_score_line(args, registry, card_data),
        "inspect_stack" => tool_inspect_stack(args, registry),
        "load_puzzle" => tool_load_puzzle(args, registry, card_data),
        "save_puzzle" => tool_save_puzzle(args, registry),
        "verify_puzzle" => tool_verify_puzzle(args, registry, card_data),

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
    // Optional zone map (returns empty HashMap if missing, errors only on bad shape).
    let parse_optional_zone_map = |key: &str| -> Result<HashMap<u8, Vec<String>>, String> {
        let Some(obj) = args.get(key) else {
            return Ok(HashMap::new());
        };
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
        let text = std::fs::read_to_string(path).map_err(|e| format!("reading {}: {}", path, e))?;
        RecordingInput::Text(text)
    } else {
        return Err("provide recording_json or recording_path".into());
    };

    // Resolve (format, constructed LiveGame). Detection: a native recording is
    // a single JSON object carrying `actions`/`initial_state`; a DCGO recording
    // is JSONL (one object per line, leading `game_start` row).
    let (lg_result, source_format): (Result<LiveGame, LiveGameError>, &'static str) = match (
        explicit_format,
        input,
    ) {
        (Some("native"), RecordingInput::NativeValue(v)) => {
            (LiveGame::from_recording(v, card_data), "native")
        }
        (Some("native"), RecordingInput::Text(t)) => {
            let v: Value =
                serde_json::from_str(&t).map_err(|e| format!("parsing native recording: {}", e))?;
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

const MAX_AUTO_OPPONENT_STEPS: usize = 64;

/// Resolve the effective `Perspective` for a view call, enforcing `SessionMode::Play`
/// seat locks (rejecting `view="god"` or opponent seat views with `{ ok: false, error }`).
fn resolve_perspective(args: &Value, mode: &SessionMode) -> Result<Perspective, Value> {
    match mode {
        SessionMode::Author => Ok(view_from_args(args)),
        SessionMode::Play { seat, .. } => match args.get("view").and_then(Value::as_str) {
            None => Ok(Perspective::Player(*seat)),
            Some("god") => Err(err_value("view=\"god\" is blocked in play mode")),
            Some("player0") if *seat == 0 => Ok(Perspective::Player(0)),
            Some("player1") if *seat == 1 => Ok(Perspective::Player(1)),
            Some(other) => Err(err_value(format!(
                "view={other:?} is blocked in play mode (locked to player{seat})"
            ))),
        },
    }
}

fn ensure_author_mode(entry: &GameEntry, tool_name: &str) -> Result<(), Value> {
    if matches!(entry.mode, SessionMode::Play { .. }) {
        return Err(err_value(format!(
            "{tool_name} is not permitted in play mode"
        )));
    }
    Ok(())
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
    match registry.get_entry_mut(&game_id) {
        Ok(entry) => {
            if let Err(err) = ensure_author_mode(entry, "replay stepping/scanning") {
                return Ok(err);
            }
            match f(&mut entry.game) {
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
            }
        }
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
    match registry.get_entry(&game_id) {
        Ok(entry) => {
            if let Err(err) = ensure_author_mode(entry, "replay_step_view") {
                return Ok(err);
            }
            match entry.game.replay_step_view() {
                Ok(view) => Ok(json!({ "ok": true, "view": view })),
                Err(e) => Ok(err_value(e.to_string())),
            }
        }
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
        .iter_entries()
        .map(|(id, entry)| {
            let g = &entry.game;
            json!({
                "game_id": id,
                "mode": entry.mode.as_str(),
                "puzzle_id": entry.puzzle_id,
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
    match registry.get_entry(&game_id) {
        Ok(entry) => match resolve_perspective(&args, &entry.mode) {
            Ok(view) => f(&entry.game, view),
            Err(err) => Ok(err),
        },
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
    match registry.get_entry(&game_id) {
        Ok(entry) => match resolve_perspective(&args, &entry.mode) {
            Ok(view) => f(&entry.game, player, view),
            Err(err) => Ok(err),
        },
        Err(e) => Ok(registry_err_to_value(e)),
    }
}

fn tool_field(args: Value, registry: &GameRegistry) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let player = get_u8(&args, "player")?;
    match registry.get_entry(&game_id) {
        Ok(entry) => match resolve_perspective(&args, &entry.mode) {
            Ok(view) => {
                let mut val = json!(entry.game.field(player, view));
                if view != Perspective::God {
                    redact_face_down_field_sources(&entry.game, player, &mut val);
                }
                Ok(val)
            }
            Err(err) => Ok(err),
        },
        Err(e) => Ok(registry_err_to_value(e)),
    }
}

fn redact_face_down_field_sources(lg: &LiveGame, player: u8, field_json: &mut Value) {
    if (player as usize) >= lg.game.players.len() {
        return;
    }
    let p = lg.game.player(player);
    if let Some(ba) = field_json.get_mut("battle_area").and_then(Value::as_array_mut) {
        for (slot, pv) in ba.iter_mut().enumerate() {
            if let (Some(perm), Some(stack)) = (
                p.battle_area.get(slot),
                pv.get_mut("stack_card_ids").and_then(Value::as_array_mut),
            ) {
                for (s_idx, cs) in perm.card_sources.iter().enumerate() {
                    if cs.face_down {
                        if let Some(cell) = stack.get_mut(s_idx) {
                            *cell = json!(OPAQUE_PLACEHOLDER_CARD_ID);
                        }
                    }
                }
            }
        }
    }
    if let Some(br) = field_json.get_mut("breeding").and_then(Value::as_array_mut) {
        if let (Some(perm), Some(pv)) = (p.breeding_area.as_ref(), br.get_mut(0)) {
            if let Some(stack) = pv.get_mut("stack_card_ids").and_then(Value::as_array_mut) {
                for (s_idx, cs) in perm.card_sources.iter().enumerate() {
                    if cs.face_down {
                        if let Some(cell) = stack.get_mut(s_idx) {
                            *cell = json!(OPAQUE_PLACEHOLDER_CARD_ID);
                        }
                    }
                }
            }
        }
    }
}

fn redact_pending_for_mode(pending: Option<Value>, mode: &SessionMode) -> Value {
    let Some(mut pval) = pending else {
        return Value::Null;
    };
    if let SessionMode::Play { seat, .. } = mode {
        let selecting = pval
            .get("selecting_player")
            .and_then(Value::as_u64)
            .map(|n| n as u8);
        if selecting != Some(*seat) {
            pval["options"] = json!([]);
            pval["effect_choices"] = Value::Null;
        }
    }
    pval
}

fn tool_pending_selection(args: Value, registry: &GameRegistry) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    match registry.get_entry(&game_id) {
        Ok(entry) => {
            if let Err(err) = resolve_perspective(&args, &entry.mode) {
                return Ok(err);
            }
            let raw = entry.game.pending_selection().map(|p| json!(p));
            Ok(redact_pending_for_mode(raw, &entry.mode))
        }
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
    match registry.get_entry(&game_id) {
        Ok(entry) => {
            if let Err(err) = ensure_author_mode(entry, "events") {
                return Ok(err);
            }
            Ok(json!(entry.game.events(since_seq)))
        }
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
    match registry.get_entry(&game_id) {
        Ok(entry) => {
            if let SessionMode::Play { seat, .. } = entry.mode {
                if player != seat {
                    return Ok(err_value(format!(
                        "legal_actions for opponent player {player} is blocked in play mode (locked to player {seat})"
                    )));
                }
            }
            Ok(json!(entry.game.legal_actions(player)))
        }
        Err(e) => Ok(registry_err_to_value(e)),
    }
}

fn tool_deck_cards(args: Value, registry: &GameRegistry) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    match registry.get_entry(&game_id) {
        Ok(entry) => {
            if let Err(err) = ensure_author_mode(entry, "deck_cards") {
                return Ok(err);
            }
            Ok(json!(entry.game.deck_cards()))
        }
        Err(e) => Ok(registry_err_to_value(e)),
    }
}

fn tool_recorded_actions(args: Value, registry: &GameRegistry) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let decode_labels = args
        .get("decode_labels")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    match registry.get_entry(&game_id) {
        Ok(entry) => {
            if let Err(err) = ensure_author_mode(entry, "recorded_actions") {
                return Ok(err);
            }
            match entry.game.recorded_actions(decode_labels) {
                Some(actions) => Ok(json!({ "ok": true, "actions": actions })),
                None => Ok(err_value(
                    "this game was not constructed from a recording — use load_recording first",
                )),
            }
        }
        Err(e) => Ok(registry_err_to_value(e)),
    }
}

// ── actions & Play-mode opponent auto-stepper ────────────────────────────

fn redact_event_for_seat(mut ev: Value, seat: u8) -> Value {
    let ev_type = ev.get("type").and_then(Value::as_str).unwrap_or("");
    let ev_player = ev
        .get("player")
        .and_then(Value::as_u64)
        .map(|n| n as u8);
    if ev_type == "CardDrawn" && ev_player.is_some_and(|p| p != seat) {
        if ev.get("card_id").is_some() {
            ev["card_id"] = json!(OPAQUE_PLACEHOLDER_CARD_ID);
        }
    }
    ev
}

fn choose_driver_action(
    lg: &LiveGame,
    opp_pid: u8,
    driver: &OpponentDriver,
    models: &mut ModelRegistry,
    card_registry: &CardRegistry,
    step_idx: usize,
) -> Result<u16, String> {
    let mask = build_action_mask(&lg.game, opp_pid);
    let legal_ids: Vec<u16> = mask
        .iter()
        .enumerate()
        .filter_map(|(i, &m)| (m > 0.5).then_some(i as u16))
        .collect();
    if legal_ids.is_empty() {
        return Err("opponent has no legal actions".to_string());
    }
    if legal_ids.len() == 1 {
        return Ok(legal_ids[0]);
    }
    match driver {
        OpponentDriver::Greedy => Ok(greedy_action(&lg.game, &mask)),
        OpponentDriver::Onnx { model_id } => {
            let model = models
                .get_mut(model_id)
                .ok_or_else(|| format!("opponent_driver references unloaded model_id {model_id:?}"))?;
            let profile = model.info.profile;
            let obs = build_observation_tensor_with_mask(
                &lg.game,
                opp_pid,
                card_registry,
                profile,
                &mask,
            );
            let st_in = model.evaluator.root_recurrent_state(opp_pid);
            let (evals, _) = model.evaluator.evaluate_batch_recurrent(
                &[EvalRequest {
                    obs: obs.clone(),
                    mask: mask.clone(),
                }],
                &[st_in],
            );
            model
                .evaluator
                .observe_committed_step(opp_pid, &obs, &mask);
            let policy = &evals
                .first()
                .ok_or_else(|| "empty policy evaluation".to_string())?
                .policy;
            let best = legal_ids
                .iter()
                .copied()
                .max_by(|&a, &b| {
                    let pa = policy.get(a as usize).copied().unwrap_or(0.0);
                    let pb = policy.get(b as usize).copied().unwrap_or(0.0);
                    pa.partial_cmp(&pb).unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap_or(legal_ids[0]);
            Ok(best)
        }
        OpponentDriver::Mcts {
            model_id,
            simulations,
            worlds,
            seed,
        } => {
            let mut cfg = DeterminizedConfig::default();
            cfg.worlds = (*worlds).max(1);
            cfg.search.simulations = (*simulations).max(1);
            cfg.search.dirichlet_epsilon = 0.0;
            cfg.search.seed = seed.wrapping_add((step_idx as u64).wrapping_mul(0x9E3779B97F4A7C15));
            let res = if let Some(mid) = model_id {
                let model = models
                    .get_mut(mid)
                    .ok_or_else(|| format!("opponent_driver references unloaded model_id {mid:?}"))?;
                cfg.search.observation_profile = model.info.profile;
                determinized_search_batched(
                    card_registry,
                    &lg.game,
                    opp_pid,
                    model.evaluator.as_mut(),
                    &cfg,
                    4,
                )
            } else {
                cfg.search.observation_profile = ObservationProfileId::StandardLiteV2;
                let mut uni = UniformEvaluator;
                determinized_search_batched(card_registry, &lg.game, opp_pid, &mut uni, &cfg, 4)
            };
            Ok(res
                .best_plan()
                .and_then(|p| p.first().copied())
                .unwrap_or_else(|| greedy_action(&lg.game, &mask)))
        }
    }
}

fn auto_step_opponent_if_needed(
    entry: &mut GameEntry,
    models: &mut ModelRegistry,
    card_registry: &CardRegistry,
) -> Result<Option<Vec<Value>>, String> {
    let SessionMode::Play {
        seat,
        opponent_driver: Some(ref driver),
    } = entry.mode
    else {
        return Ok(None);
    };
    let driver = driver.clone();
    let mut opponent_events = Vec::new();
    for step_idx in 0..MAX_AUTO_OPPONENT_STEPS {
        if entry.game.game.game_over {
            break;
        }
        let decider = entry.game.current_decision_player();
        if decider == seat {
            break;
        }
        let action_id =
            choose_driver_action(&entry.game, decider, &driver, models, card_registry, step_idx)?;
        let step_res = entry.game.step(action_id);
        for ev in step_res.events_emitted {
            opponent_events.push(redact_event_for_seat(ev, seat));
        }
        if !step_res.ok {
            return Err(step_res
                .error
                .unwrap_or_else(|| "auto-opponent step failed".to_string()));
        }
    }
    Ok(Some(opponent_events))
}

fn run_player_action(
    game_id: &str,
    explicit_actor: Option<u8>,
    registry: &mut GameRegistry,
    card_data: &HashMap<String, CardData>,
    action_fn: impl FnOnce(&mut LiveGame) -> ActionResult,
) -> Value {
    let (entry, models, creg) = match registry.split_mut_with_cards(game_id, card_data) {
        Ok(parts) => parts,
        Err(e) => return registry_err_to_value(e),
    };

    if let SessionMode::Play { seat, .. } = entry.mode {
        if let Some(actor) = explicit_actor {
            if actor != seat {
                return err_value(format!(
                    "action rejected: play mode is locked to player {seat} (got player {actor})"
                ));
            }
        }
        let current = entry.game.current_decision_player();
        if !entry.game.game.game_over && current != seat {
            return err_value(format!(
                "action rejected: waiting on opponent player {current} (play mode locked to player {seat})"
            ));
        }
    }

    let res = action_fn(&mut entry.game);
    let mut val = json!(res);

    if let SessionMode::Play { seat, .. } = entry.mode {
        if let Some(arr) = val.get_mut("events_emitted").and_then(Value::as_array_mut) {
            for item in arr.iter_mut() {
                let taken = std::mem::take(item);
                *item = redact_event_for_seat(taken, seat);
            }
        }
        if res.ok {
            match auto_step_opponent_if_needed(entry, models, creg) {
                Ok(Some(opp_events)) => {
                    val["new_phase"] = json!(format!("{:?}", entry.game.game.current_phase));
                    let post_pending = entry.game.pending_selection().map(|p| json!(p));
                    val["pending_selection_after"] =
                        redact_pending_for_mode(post_pending, &entry.mode);
                    val["opponent_events"] = json!(opp_events);
                }
                Ok(None) => {
                    let cur_pending = val.get("pending_selection_after").cloned();
                    val["pending_selection_after"] =
                        redact_pending_for_mode(cur_pending, &entry.mode);
                }
                Err(driver_err) => {
                    val["opponent_driver_error"] = json!(driver_err);
                }
            }
        } else {
            let cur_pending = val.get("pending_selection_after").cloned();
            val["pending_selection_after"] = redact_pending_for_mode(cur_pending, &entry.mode);
        }
    }

    val
}

fn tool_end_turn(
    args: Value,
    registry: &mut GameRegistry,
    card_data: &HashMap<String, CardData>,
) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    Ok(run_player_action(&game_id, None, registry, card_data, |g| {
        g.end_turn()
    }))
}

fn tool_pass_turn(
    args: Value,
    registry: &mut GameRegistry,
    card_data: &HashMap<String, CardData>,
) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    Ok(run_player_action(&game_id, None, registry, card_data, |g| {
        g.pass_turn()
    }))
}

fn tool_play(
    args: Value,
    registry: &mut GameRegistry,
    card_data: &HashMap<String, CardData>,
) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let player = get_u8(&args, "player")?;
    let hand_idx = get_usize(&args, "hand_idx")?;
    Ok(run_player_action(
        &game_id,
        Some(player),
        registry,
        card_data,
        move |g| g.play(player, hand_idx),
    ))
}

fn tool_resolve_selection(
    args: Value,
    registry: &mut GameRegistry,
    card_data: &HashMap<String, CardData>,
) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let player = get_u8(&args, "player")?;
    let action_id = get_u16(&args, "action_id")?;
    Ok(run_player_action(
        &game_id,
        Some(player),
        registry,
        card_data,
        move |g| g.resolve_selection(player, action_id),
    ))
}

fn tool_move_from_breeding(
    args: Value,
    registry: &mut GameRegistry,
    card_data: &HashMap<String, CardData>,
) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let player = get_u8(&args, "player")?;
    Ok(run_player_action(
        &game_id,
        Some(player),
        registry,
        card_data,
        move |g| g.move_from_breeding(player),
    ))
}

fn tool_step(
    args: Value,
    registry: &mut GameRegistry,
    card_data: &HashMap<String, CardData>,
) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let action_id = get_u16(&args, "action_id")?;
    Ok(run_player_action(&game_id, None, registry, card_data, move |g| {
        g.step(action_id)
    }))
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

fn tool_digivolve(
    args: Value,
    registry: &mut GameRegistry,
    card_data: &HashMap<String, CardData>,
) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let host = parse_handle(&args, "host")?;
    let source_hand_idx = get_usize(&args, "source_hand_idx")?;
    Ok(run_player_action(
        &game_id,
        Some(host.player),
        registry,
        card_data,
        move |g| g.digivolve(host, source_hand_idx),
    ))
}

fn tool_attack(
    args: Value,
    registry: &mut GameRegistry,
    card_data: &HashMap<String, CardData>,
) -> Result<Value, String> {
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
    Ok(run_player_action(
        &game_id,
        Some(attacker.player),
        registry,
        card_data,
        move |g| g.attack(attacker, target),
    ))
}

// ── Phase 1A & 1B: ONNX models, inspection, stack, and puzzles ───────────

fn tool_load_model(args: Value, registry: &mut GameRegistry) -> Result<Value, String> {
    let model_id = get_str(&args, "model_id")?.to_string();
    let onnx_path = get_str(&args, "onnx_path")?.to_string();
    let profile_override = match args.get("profile").and_then(Value::as_str) {
        None | Some("") => None,
        Some(raw) => match parse_observation_profile(raw) {
            Ok(p) => Some(p),
            Err(e) => return Ok(err_value(e)),
        },
    };
    match registry
        .models
        .load(model_id.clone(), onnx_path, profile_override)
    {
        Ok(info) => Ok(json!({
            "ok": true,
            "model_id": model_id,
            "kind": info.kind,
            "profile": info.profile.as_str(),
            "value_semantics": info.value_semantics,
        })),
        Err(e) => Ok(err_value(e.to_string())),
    }
}

fn tool_unload_model(args: Value, registry: &mut GameRegistry) -> Result<Value, String> {
    let model_id = get_str(&args, "model_id")?;
    let removed = registry.models.unload(model_id);
    Ok(json!({
        "ok": true,
        "removed": removed,
    }))
}

fn tool_list_models(registry: &GameRegistry) -> Value {
    json!({
        "ok": true,
        "models": registry.models.list_json(),
    })
}

fn tool_inspect_position(
    args: Value,
    registry: &mut GameRegistry,
    card_data: &HashMap<String, CardData>,
) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let model_id = args
        .get("model_id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(String::from);
    let simulations = args
        .get("simulations")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let worlds = args.get("worlds").and_then(Value::as_u64).unwrap_or(1) as u32;
    let regime = args
        .get("regime")
        .and_then(Value::as_str)
        .unwrap_or("open_list")
        .to_string();
    let seed = args.get("seed").and_then(Value::as_u64).unwrap_or(0);
    let top_k = args.get("top_k").and_then(Value::as_u64).unwrap_or(12) as usize;
    let expectations = match args.get("expectations") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(arr)) => arr.clone(),
        Some(_) => return Err("'expectations' must be an array or null".into()),
    };

    let (entry, models, creg) = match registry.split_mut_with_cards(&game_id, card_data) {
        Ok(parts) => parts,
        Err(e) => return Ok(registry_err_to_value(e)),
    };

    if let SessionMode::Play { seat, .. } = entry.mode {
        if regime == "perfect_info" {
            return Ok(err_value(
                "regime=\"perfect_info\" is blocked in play mode",
            ));
        }
        let current = entry.game.current_decision_player();
        if current != seat {
            return Ok(err_value(format!(
                "inspect_position blocked in play mode while waiting on opponent player {current}"
            )));
        }
    }

    let mut uniform_eval = UniformEvaluator;
    let (eval_ref, profile): (&mut dyn PolicyValueFn, ObservationProfileId) =
        match model_id.as_deref() {
            Some(mid) => {
                let Some(cached) = models.get_mut(mid) else {
                    return Ok(err_value(format!("unknown model_id: {mid}")));
                };
                (cached.evaluator.as_mut(), cached.info.profile)
            }
            None => (&mut uniform_eval, ObservationProfileId::StandardLiteV2),
        };

    let params = InspectPositionParams {
        profile,
        simulations,
        worlds,
        seed,
        regime,
        top_k,
        expectations,
    };

    match inspect_position_with_evaluator(&entry.game.game, creg, eval_ref, &params) {
        Ok(mut val) => {
            if let Some(obj) = val.as_object_mut() {
                obj.insert("ok".to_string(), json!(true));
            }
            Ok(val)
        }
        Err(e) => Ok(err_value(e)),
    }
}

fn parse_u16_array(val: Option<&Value>, label: &str) -> Result<Vec<u16>, String> {
    let Some(v) = val else {
        return Ok(Vec::new());
    };
    if v.is_null() {
        return Ok(Vec::new());
    }
    let arr = v
        .as_array()
        .ok_or_else(|| format!("{label} must be an array of integers"))?;
    arr.iter()
        .map(|item| {
            item.as_u64()
                .and_then(|n| u16::try_from(n).ok())
                .ok_or_else(|| format!("non-u16 entry in {label}"))
        })
        .collect()
}

fn parse_refutation_triggers(val: Option<&Value>) -> Result<Vec<RefutationTrigger>, String> {
    let Some(v) = val else {
        return Ok(Vec::new());
    };
    if v.is_null() {
        return Ok(Vec::new());
    }
    serde_json::from_value(v.clone())
        .map_err(|e| format!("invalid refutation_triggers: {e}"))
}

fn tool_score_line(
    args: Value,
    registry: &mut GameRegistry,
    card_data: &HashMap<String, CardData>,
) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let model_id = args
        .get("model_id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(String::from);
    let actions = parse_u16_array(Some(args.get("actions").ok_or("missing arg: actions")?), "actions")?;
    let adversary_sims = args
        .get("adversary_sims")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let refutation_triggers = parse_refutation_triggers(args.get("refutation_triggers"))?;

    let (entry, models, creg) = match registry.split_mut_with_cards(&game_id, card_data) {
        Ok(parts) => parts,
        Err(e) => return Ok(registry_err_to_value(e)),
    };

    if let Err(err) = ensure_author_mode(entry, "score_line") {
        return Ok(err);
    }

    let mut uniform_eval = UniformEvaluator;
    let (eval_ref, profile): (&mut dyn PolicyValueFn, ObservationProfileId) =
        match model_id.as_deref() {
            Some(mid) => {
                let Some(cached) = models.get_mut(mid) else {
                    return Ok(err_value(format!("unknown model_id: {mid}")));
                };
                (cached.evaluator.as_mut(), cached.info.profile)
            }
            None => (&mut uniform_eval, ObservationProfileId::StandardLiteV2),
        };
    let params = ScoreLineParams {
        actions,
        auto_adversary: adversary_sims > 0 || !refutation_triggers.is_empty(),
        adversary_sims,
        adversary_seed: 0,
        refutation_triggers,
        max_adversary_steps: 64,
    };

    match score_line_with_evaluator(&entry.game.game, creg, eval_ref, profile, &params) {
        Ok(res) => Ok(json!({
            "ok": true,
            "root_mover": res.root_mover,
            "root_policy_value": res.root_policy_value,
            "steps": res.steps,
            "final_winner": res.final_winner,
        })),
        Err(e) => Ok(err_value(e)),
    }
}

fn tool_inspect_stack(args: Value, registry: &GameRegistry) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let player = get_u8(&args, "player")?;
    let zone = args
        .get("zone")
        .and_then(Value::as_str)
        .unwrap_or("battle");
    let slot = args.get("slot").and_then(Value::as_u64).unwrap_or(0) as usize;

    let entry = match registry.get_entry(&game_id) {
        Ok(e) => e,
        Err(e) => return Ok(registry_err_to_value(e)),
    };
    let view = match resolve_perspective(&args, &entry.mode) {
        Ok(v) => v,
        Err(err) => return Ok(err),
    };
    match entry.game.inspect_stack(player, zone, slot, view) {
        Ok(stack) => Ok(json!({
            "ok": true,
            "top_card": {
                "card_id": stack.top_card.card_id,
                "card_name": stack.top_card.card_name,
                "level": stack.top_card.level,
                "level_printed": stack.top_card.level_printed,
                "dp_printed": stack.top_card.dp_printed,
                "dp_effective": stack.top_card.dp_effective,
                "colors": stack.top_card.colors,
                "traits": stack.top_card.traits,
                "effect_text": stack.top_card.effect_text,
            },
            "inherited_sources": stack.inherited_sources,
            "linked_cards": stack.linked_cards,
            "is_suspended": stack.is_suspended,
            "turn_played": stack.turn_played,
            "modifiers": stack.modifiers,
        })),
        Err(e) => Ok(err_value(e)),
    }
}

fn read_puzzle_document(args: &Value) -> Result<Value, String> {
    if let Some(inline) = args.get("puzzle_json") {
        if !inline.is_null() {
            return match inline {
                Value::String(s) => {
                    serde_json::from_str(s).map_err(|e| format!("parsing puzzle_json string: {e}"))
                }
                Value::Object(_) => Ok(inline.clone()),
                _ => Err("puzzle_json must be a JSON object or JSON string".into()),
            };
        }
    }
    if let Some(path) = args.get("puzzle_path").and_then(Value::as_str) {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| format!("reading puzzle_path {path}: {e}"))?;
        return serde_json::from_str(&raw).map_err(|e| format!("parsing {path}: {e}"));
    }
    Err("provide puzzle_path or puzzle_json".into())
}

struct LoadedPuzzleSpec {
    puzzle_id: String,
    seat: u8,
    live_game: LiveGame,
}

fn parse_deck_vec(v: &Value, label: &str) -> Result<Vec<String>, String> {
    v.as_array()
        .ok_or_else(|| format!("{label} must be an array of card_id strings"))?
        .iter()
        .map(|s| {
            s.as_str()
                .map(String::from)
                .ok_or_else(|| format!("non-string card_id in {label}"))
        })
        .collect()
}

fn build_puzzle_live_game(
    doc: &Value,
    card_data: &HashMap<String, CardData>,
) -> Result<LoadedPuzzleSpec, String> {
    let schema = doc.get("schema").and_then(Value::as_str).unwrap_or("");
    let puzzle_id = doc
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| "puzzle document missing string 'id'".to_string())?
        .to_string();

    match schema {
        "studio-puzzle-v1" => {
            let decks_obj = doc
                .get("decks")
                .and_then(Value::as_object)
                .ok_or_else(|| "studio-puzzle-v1 missing 'decks' object".to_string())?;
            let deck1 = parse_deck_vec(
                decks_obj
                    .get("1")
                    .ok_or_else(|| "studio-puzzle-v1 decks missing key '1'".to_string())?,
                "decks.1",
            )?;
            let deck2 = parse_deck_vec(
                decks_obj
                    .get("2")
                    .ok_or_else(|| "studio-puzzle-v1 decks missing key '2'".to_string())?,
                "decks.2",
            )?;
            let seed = doc.get("seed").and_then(Value::as_u64).unwrap_or(0);
            let seat_1based = doc.get("seat").and_then(Value::as_u64).unwrap_or(1);
            let seat = (seat_1based.max(1) - 1).min(1) as u8;

            let mut lg = LiveGame::from_decks(deck1, deck2, Some(seed), card_data)
                .map_err(|e| e.to_string())?;

            let setup = doc.get("setup").unwrap_or(&Value::Null);
            let kind = setup
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("staged");
            let prefix_actions =
                parse_u16_array(setup.get("prefix_actions"), "setup.prefix_actions")?;

            if kind == "staged" || setup.get("zones").is_some() || setup.get("state").is_some() {
                while let Some(p) = lg.game.mulligan_current_player() {
                    let _ = lg.game.accept_mulligan(p, true);
                }
                let scenario = json!({
                    "state": setup.get("state").cloned().unwrap_or_else(|| json!({
                        "memory": 0,
                        "phase": "Main",
                        "turn": 1,
                        "first_player": seat_1based.max(1),
                    })),
                    "zones": setup.get("zones").cloned().unwrap_or_else(|| json!({})),
                });
                lg.game.apply_scenario(&scenario)?;
                lg.game.tick_declarative_effects();
                let _ = lg.game.enable_belief_knowledge();
            } else {
                let _ = lg.game.enable_belief_knowledge();
                if prefix_actions.is_empty() {
                    while let Some(p) = lg.game.mulligan_current_player() {
                        let _ = lg.game.accept_mulligan(p, true);
                    }
                }
            }

            for (idx, aid) in prefix_actions.iter().copied().enumerate() {
                if lg.game.game_over {
                    break;
                }
                let pid = decision_player(&lg.game);
                let mask = build_action_mask(&lg.game, pid);
                if (aid as usize) >= mask.len() || mask[aid as usize] <= 0.5 {
                    return Err(format!(
                        "prefix_actions[{idx}] action {aid} is illegal for player {pid} in phase {:?}",
                        lg.game.current_phase
                    ));
                }
                lg.game.decode_action(aid, pid);
            }

            Ok(LoadedPuzzleSpec {
                puzzle_id,
                seat,
                live_game: lg,
            })
        }
        "search-exam-v1" => {
            let setup = doc
                .get("setup")
                .ok_or_else(|| "search-exam-v1 missing 'setup'".to_string())?;
            let decks_val = setup
                .get("decks")
                .ok_or_else(|| "search-exam-v1 setup missing 'decks'".to_string())?;
            let (deck1, deck2) = if let Some(arr) = decks_val.as_array() {
                if arr.len() < 2 {
                    return Err("search-exam-v1 setup.decks must have 2 decks".into());
                }
                (
                    parse_deck_vec(&arr[0], "setup.decks[0]")?,
                    parse_deck_vec(&arr[1], "setup.decks[1]")?,
                )
            } else if let Some(obj) = decks_val.as_object() {
                (
                    parse_deck_vec(
                        obj.get("1")
                            .ok_or_else(|| "setup.decks missing '1'".to_string())?,
                        "setup.decks.1",
                    )?,
                    parse_deck_vec(
                        obj.get("2")
                            .ok_or_else(|| "setup.decks missing '2'".to_string())?,
                        "setup.decks.2",
                    )?,
                )
            } else {
                return Err("search-exam-v1 setup.decks must be an array or object".into());
            };

            let seed = setup.get("game_seed").and_then(Value::as_u64).unwrap_or(0);
            let seat = doc
                .get("search")
                .and_then(|s| s.get("viewer"))
                .and_then(Value::as_u64)
                .unwrap_or(0)
                .min(1) as u8;

            let mut lg = LiveGame::from_decks(deck1, deck2, Some(seed), card_data)
                .map_err(|e| e.to_string())?;

            if let Some(staged) = setup.get("staged").filter(|v| !v.is_null()) {
                while let Some(p) = lg.game.mulligan_current_player() {
                    let _ = lg.game.accept_mulligan(p, true);
                }
                lg.game.apply_scenario(staged)?;
                lg.game.tick_declarative_effects();
                let _ = lg.game.enable_belief_knowledge();
            } else {
                let _ = lg.game.enable_belief_knowledge();
                lg.game.turn_count =
                    setup.get("turn_count_reset").and_then(Value::as_u64).unwrap_or(1) as u16;
            }

            let prefix_actions = parse_u16_array(setup.get("actions"), "setup.actions")?;
            for (idx, aid) in prefix_actions.iter().copied().enumerate() {
                if lg.game.game_over {
                    break;
                }
                let pid = decision_player(&lg.game);
                let mask = build_action_mask(&lg.game, pid);
                if (aid as usize) >= mask.len() || mask[aid as usize] <= 0.5 {
                    return Err(format!(
                        "setup.actions[{idx}] action {aid} is illegal for player {pid} in phase {:?}",
                        lg.game.current_phase
                    ));
                }
                lg.game.decode_action(aid, pid);
            }

            Ok(LoadedPuzzleSpec {
                puzzle_id,
                seat,
                live_game: lg,
            })
        }
        other => Err(format!(
            "unsupported puzzle schema {:?} — expected \"studio-puzzle-v1\" or \"search-exam-v1\"",
            other
        )),
    }
}

fn tool_load_puzzle(
    args: Value,
    registry: &mut GameRegistry,
    card_data: &HashMap<String, CardData>,
) -> Result<Value, String> {
    let doc = match read_puzzle_document(&args) {
        Ok(d) => d,
        Err(e) => return Ok(err_value(e)),
    };
    let mode_str = args
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("author");
    let opponent_driver = match OpponentDriver::from_json(args.get("opponent_driver")) {
        Ok(d) => d,
        Err(e) => return Ok(err_value(e)),
    };

    let spec = match build_puzzle_live_game(&doc, card_data) {
        Ok(s) => s,
        Err(e) => return Ok(err_value(e)),
    };

    let mode = match mode_str {
        "author" => SessionMode::Author,
        "play" => SessionMode::Play {
            seat: spec.seat,
            opponent_driver,
        },
        other => {
            return Ok(err_value(format!(
                "unknown mode {other:?} — expected \"author\" or \"play\""
            )))
        }
    };

    let puzzle_id = spec.puzzle_id.clone();
    let seat = spec.seat;
    let entry = GameEntry {
        game: spec.live_game,
        mode,
        puzzle_id: Some(puzzle_id.clone()),
    };

    let game_id = match registry.insert_entry(entry) {
        Ok(id) => id,
        Err(e) => return Ok(registry_err_to_value(e)),
    };

    // In play mode with a bound opponent_driver, if the initial decision
    // belongs to the opponent, advance until it's the solver seat's turn.
    if mode_str == "play" {
        if let Ok((entry, models, creg)) = registry.split_mut_with_cards(&game_id, card_data) {
            if let Err(e) = auto_step_opponent_if_needed(entry, models, creg) {
                return Ok(err_value(format!(
                    "initial auto-opponent step failed on load_puzzle: {e}"
                )));
            }
        }
    }

    Ok(json!({
        "ok": true,
        "game_id": game_id,
        "puzzle_id": puzzle_id,
        "seat": seat,
        "mode": mode_str,
    }))
}

fn find_repo_root() -> PathBuf {
    if let Ok(mut dir) = std::env::current_dir() {
        for _ in 0..6 {
            if dir.join("data").join("cards.json").exists() {
                return dir;
            }
            if !dir.pop() {
                break;
            }
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn resolve_safe_output_path(raw: &str, default_subdir: &str) -> Result<PathBuf, String> {
    let p = Path::new(raw);
    for comp in p.components() {
        if matches!(comp, Component::ParentDir) {
            return Err(format!("path traversal '..' is not allowed in {raw:?}"));
        }
    }
    if p.is_absolute() {
        return Ok(p.to_path_buf());
    }
    let root = find_repo_root();
    let normalized = raw.replace('\\', "/");
    if normalized.starts_with(default_subdir) {
        Ok(root.join(p))
    } else {
        Ok(root.join(default_subdir).join(p))
    }
}

fn tool_save_puzzle(args: Value, registry: &GameRegistry) -> Result<Value, String> {
    let game_id = get_str(&args, "game_id")?.to_string();
    let raw_puzzle_path = get_str(&args, "puzzle_path")?;
    let id = get_str(&args, "id")?.trim().to_string();
    let title = get_str(&args, "title")?.trim().to_string();
    if id.is_empty() {
        return Ok(err_value("puzzle 'id' must not be empty"));
    }

    let entry = match registry.get_entry(&game_id) {
        Ok(e) => e,
        Err(e) => return Ok(registry_err_to_value(e)),
    };
    if let Err(err) = ensure_author_mode(entry, "save_puzzle") {
        return Ok(err);
    }

    let dest_puzzle = match resolve_safe_output_path(raw_puzzle_path, "qa/puzzles") {
        Ok(p) => p,
        Err(e) => return Ok(err_value(e)),
    };

    let mut snap = entry.game.game.to_scenario();
    snap["state"]["first_player"] = json!((entry.game.game.turn_player() as u64) + 1);
    let orig_decks = entry.game.deck_lists();
    let deck1: Vec<String> = if !orig_decks[0].is_empty() {
        orig_decks[0].clone()
    } else {
        snap["decks"]["1"]
            .as_array()
            .map(|a| a.iter().filter_map(Value::as_str).map(String::from).collect())
            .unwrap_or_default()
    };
    let deck2: Vec<String> = if !orig_decks[1].is_empty() {
        orig_decks[1].clone()
    } else {
        snap["decks"]["2"]
            .as_array()
            .map(|a| a.iter().filter_map(Value::as_str).map(String::from).collect())
            .unwrap_or_default()
    };

    let seat_0based: u8 = match args.get("seat").and_then(Value::as_u64) {
        None => entry.game.current_decision_player(),
        Some(0 | 1) => 0,
        Some(_) => 1,
    };
    let description = args
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let folder = args
        .get("folder")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim_matches('/')
        .to_string();
    let tags: Vec<String> = args
        .get("tags")
        .and_then(Value::as_array)
        .map(|arr| {
            let mut out: Vec<String> = arr
                .iter()
                .filter_map(Value::as_str)
                .map(|t| t.trim().to_lowercase())
                .filter(|t| !t.is_empty())
                .collect();
            out.sort();
            out.dedup();
            out
        })
        .unwrap_or_default();
    let lines = args
        .get("lines")
        .filter(|v| !v.is_null())
        .cloned()
        .unwrap_or_else(|| json!([]));
    let expectations = args
        .get("expectations")
        .filter(|v| !v.is_null())
        .cloned()
        .unwrap_or_else(|| json!([]));
    let assertions = args
        .get("assertions")
        .filter(|v| !v.is_null())
        .cloned()
        .unwrap_or_else(|| json!({ "engine": [], "ui": [] }));
    let rl_goal = args
        .get("rl_goal")
        .filter(|v| !v.is_null())
        .cloned()
        .unwrap_or_else(|| {
            json!({
                "mode": "playout_to_win",
                "max_turns": null,
                "max_actions": null,
                "win_assertions": [],
            })
        });
    let refutation_triggers = match parse_refutation_triggers(args.get("refutation_triggers")) {
        Ok(rt) => rt,
        Err(e) => return Ok(err_value(e)),
    };

    let mut puzzle_doc = json!({
        "schema": "studio-puzzle-v1",
        "id": id,
        "title": if title.is_empty() { id.clone() } else { title.clone() },
        "description": description,
        "folder": folder,
        "tags": tags,
        "source": { "kind": "mcp" },
        "decks": {
            "1": deck1,
            "2": deck2,
        },
        "seed": 0,
        "seat": (seat_0based as u64) + 1,
        "setup": {
            "kind": "staged",
            "state": snap["state"],
            "zones": snap["zones"],
            "prefix_actions": [],
        },
        "lines": lines,
        "expectations": expectations,
        "assertions": assertions,
        "rl_goal": rl_goal,
    });
    if !refutation_triggers.is_empty() {
        puzzle_doc["refutation_triggers"] = json!(refutation_triggers);
    }

    if let Some(parent) = dest_puzzle.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return Ok(err_value(format!("creating {}: {e}", parent.display())));
        }
    }
    let formatted = serde_json::to_string_pretty(&puzzle_doc)
        .map_err(|e| format!("serializing puzzle: {e}"))?;
    if let Err(e) = std::fs::write(&dest_puzzle, format!("{formatted}\n")) {
        return Ok(err_value(format!("writing {}: {e}", dest_puzzle.display())));
    }

    let mut search_exam_written: Option<String> = None;
    if let Some(raw_exam_path) = args
        .get("export_search_exam_path")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
    {
        let dest_exam = match resolve_safe_output_path(raw_exam_path, "qa/search-exams") {
            Ok(p) => p,
            Err(e) => return Ok(err_value(e)),
        };

        let mut exam_expectations = Vec::new();
        if let Some(exp_arr) = puzzle_doc["expectations"].as_array() {
            for exp in exp_arr {
                let kind = exp.get("kind").and_then(Value::as_str).unwrap_or("");
                let plan = exp
                    .get("plan")
                    .cloned()
                    .unwrap_or_else(|| json!([]));
                match kind {
                    "visits" | "visit_share_at_least" => {
                        let share = exp
                            .get("min_share")
                            .or_else(|| exp.get("share"))
                            .and_then(Value::as_f64)
                            .unwrap_or(0.1);
                        exam_expectations.push(json!({
                            "kind": "visit_share_at_least",
                            "plan": plan,
                            "share": share,
                            "label": exp.get("label").and_then(Value::as_str).unwrap_or("target line"),
                        }));
                    }
                    "avoid" | "not_chosen" => {
                        exam_expectations.push(json!({
                            "kind": "not_chosen",
                            "plan": plan,
                            "label": exp.get("label").and_then(Value::as_str).unwrap_or("avoid line"),
                        }));
                    }
                    "choose" | "chosen" => {
                        exam_expectations.push(json!({
                            "kind": "chosen",
                            "plan": plan,
                            "label": exp.get("label").and_then(Value::as_str).unwrap_or("expected line"),
                        }));
                    }
                    _ => {}
                }
            }
        }

        let mut lines_map = serde_json::Map::new();
        if let Some(ln_arr) = puzzle_doc["lines"].as_array() {
            for (idx, ln) in ln_arr.iter().enumerate() {
                let name = ln
                    .get("name")
                    .or_else(|| ln.get("id"))
                    .and_then(Value::as_str)
                    .map(String::from)
                    .unwrap_or_else(|| format!("line_{}", idx + 1));
                let acts = ln.get("actions").cloned().unwrap_or_else(|| json!([]));
                lines_map.insert(name, acts);
            }
        }

        let mut exam_doc = json!({
            "schema": "search-exam-v1",
            "id": puzzle_doc["id"],
            "title": puzzle_doc["title"],
            "why": puzzle_doc["description"],
            "source": { "kind": "mcp" },
            "model": { "name": "studio-authored", "sha256": "" },
            "setup": {
                "decks": [puzzle_doc["decks"]["1"], puzzle_doc["decks"]["2"]],
                "game_seed": 0,
                "actions": [],
                "staged": {
                    "state": snap["state"],
                    "zones": snap["zones"],
                },
            },
            "search": {
                "viewer": seat_0based,
                "regime": "open-list",
                "simulations": 128,
                "worlds": 4,
                "batch": 4,
                "search_seed": 0,
            },
            "expectations": exam_expectations,
            "lines": Value::Object(lines_map),
        });
        if !refutation_triggers.is_empty() {
            exam_doc["refutation_triggers"] = json!(refutation_triggers);
        }

        if let Some(parent) = dest_exam.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return Ok(err_value(format!("creating {}: {e}", parent.display())));
            }
        }
        let exam_formatted = serde_json::to_string_pretty(&exam_doc)
            .map_err(|e| format!("serializing search exam: {e}"))?;
        if let Err(e) = std::fs::write(&dest_exam, format!("{exam_formatted}\n")) {
            return Ok(err_value(format!("writing {}: {e}", dest_exam.display())));
        }
        search_exam_written = Some(dest_exam.to_string_lossy().replace('\\', "/"));
    }

    let puzzle_path_str = dest_puzzle.to_string_lossy().replace('\\', "/");
    let mut out = json!({
        "ok": true,
        "puzzle_path": puzzle_path_str,
    });
    if let Some(ep) = search_exam_written {
        out["search_exam_path"] = json!(ep);
    }
    Ok(out)
}

fn evaluate_goal_assertion(game: &digimon_engine::game::Game, solver_seat: u8, assertion: &Value) -> bool {
    let kind = assertion.get("kind").and_then(Value::as_str).unwrap_or("");
    let to_pid = |raw: Option<u64>| -> usize {
        match raw.unwrap_or((solver_seat as u64) + 1) {
            0 => 0,
            1 => 0,
            2 => 1,
            n => (n.saturating_sub(1) as usize).min(1),
        }
    };
    match kind {
        "winner" | "game_won" => {
            let expected = match assertion.get("player").and_then(Value::as_u64) {
                Some(p) => to_pid(Some(p)) as u8,
                None => solver_seat,
            };
            game.game_over && game.winner == Some(expected)
        }
        "memory_equals" => {
            let Some(val) = assertion.get("value").and_then(Value::as_i64) else {
                return false;
            };
            (game.memory as i64) == val
        }
        "memory_at_least" => {
            let Some(val) = assertion.get("value").and_then(Value::as_i64) else {
                return false;
            };
            (game.memory as i64) >= val
        }
        "zone_count" => {
            let pid = to_pid(assertion.get("player").and_then(Value::as_u64));
            let Some(val) = assertion.get("value").and_then(Value::as_u64) else {
                return false;
            };
            let p = game.player(pid as u8);
            let actual = match assertion.get("zone").and_then(Value::as_str).unwrap_or("") {
                "hand" => p.hand.len(),
                "security" => p.security.len(),
                "trash" => p.trash.len(),
                "field" | "battle_area" => p.battle_area.len(),
                "deck" => p.deck.len(),
                _ => return false,
            };
            actual == val as usize
        }
        "stack_top" => {
            let pid = to_pid(assertion.get("player").and_then(Value::as_u64));
            let idx = assertion
                .get("field_index")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize;
            let Some(expected_id) = assertion.get("card_id").and_then(Value::as_str) else {
                return false;
            };
            game.player(pid as u8)
                .battle_area
                .get(idx)
                .is_some_and(|perm| perm.top_card().card_id(&game.card_data) == expected_id)
        }
        "field_contains" => {
            let pid = to_pid(assertion.get("player").and_then(Value::as_u64));
            let Some(expected_id) = assertion.get("card_id").and_then(Value::as_str) else {
                return false;
            };
            game.player(pid as u8)
                .battle_area
                .iter()
                .any(|perm| perm.top_card().card_id(&game.card_data) == expected_id)
        }
        "field_not_contains" => {
            let pid = to_pid(assertion.get("player").and_then(Value::as_u64));
            let Some(expected_id) = assertion.get("card_id").and_then(Value::as_str) else {
                return false;
            };
            !game.player(pid as u8)
                .battle_area
                .iter()
                .any(|perm| perm.top_card().card_id(&game.card_data) == expected_id)
        }
        "effective_dp" => {
            let pid = to_pid(assertion.get("player").and_then(Value::as_u64));
            let idx = assertion
                .get("field_index")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u8;
            let Some(val) = assertion.get("value").and_then(Value::as_i64) else {
                return false;
            };
            game.effective_dp(PermanentHandle {
                player: pid as u8,
                index: idx,
            }) == Some(val as i32)
        }
        _ => false,
    }
}

fn line_satisfies_puzzle_goal(
    game: &digimon_engine::game::Game,
    solver_seat: u8,
    doc: &Value,
) -> bool {
    if game.game_over {
        return game.winner == Some(solver_seat);
    }
    let rl_goal = doc.get("rl_goal").unwrap_or(&Value::Null);
    let win_assertions = rl_goal
        .get("win_assertions")
        .and_then(Value::as_array)
        .filter(|a| !a.is_empty())
        .or_else(|| {
            doc.get("assertions")
                .and_then(|a| a.get("engine"))
                .and_then(Value::as_array)
                .filter(|a| !a.is_empty())
        });
    if let Some(assertions) = win_assertions {
        return assertions
            .iter()
            .all(|a| evaluate_goal_assertion(game, solver_seat, a));
    }
    false
}

struct PuzzleCandidateLine {
    id: String,
    name: String,
    role: String,
    actions: Vec<u16>,
}

fn extract_puzzle_lines(doc: &Value) -> Result<Vec<PuzzleCandidateLine>, String> {
    let mut out = Vec::new();
    let Some(lines_val) = doc.get("lines") else {
        return Ok(out);
    };
    if let Some(arr) = lines_val.as_array() {
        for (idx, ln) in arr.iter().enumerate() {
            let id = ln
                .get("id")
                .and_then(Value::as_str)
                .map(String::from)
                .unwrap_or_else(|| format!("line-{}", idx + 1));
            let name = ln
                .get("name")
                .and_then(Value::as_str)
                .map(String::from)
                .unwrap_or_else(|| id.clone());
            let role = ln
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or("target")
                .to_string();
            let actions = parse_u16_array(ln.get("actions"), &format!("lines[{idx}].actions"))?;
            out.push(PuzzleCandidateLine {
                id,
                name,
                role,
                actions,
            });
        }
    } else if let Some(obj) = lines_val.as_object() {
        let avoided_prefixes: Vec<Vec<u16>> = doc
            .get("expectations")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter(|e| {
                        matches!(
                            e.get("kind").and_then(Value::as_str),
                            Some("not_chosen" | "avoid")
                        )
                    })
                    .filter_map(|e| parse_u16_array(e.get("plan"), "expectation.plan").ok())
                    .filter(|p| !p.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        for (name, acts_val) in obj {
            let actions = parse_u16_array(Some(acts_val), &format!("lines.{name}"))?;
            let lower = name.to_lowercase();
            let is_avoid = lower.contains("avoid")
                || lower.contains("trap")
                || avoided_prefixes.iter().any(|p| actions.starts_with(p));
            let role = if is_avoid { "avoid" } else { "target" }.to_string();
            out.push(PuzzleCandidateLine {
                id: name.clone(),
                name: name.clone(),
                role,
                actions,
            });
        }
    }
    Ok(out)
}

fn tool_verify_puzzle(
    args: Value,
    registry: &mut GameRegistry,
    card_data: &HashMap<String, CardData>,
) -> Result<Value, String> {
    let doc = match read_puzzle_document(&args) {
        Ok(d) => d,
        Err(e) => return Ok(err_value(e)),
    };
    let model_id = args
        .get("model_id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(String::from);
    let adversary_sims = args
        .get("adversary_sims")
        .and_then(Value::as_u64)
        .unwrap_or(64) as u32;
    let seeds = args
        .get("seeds")
        .and_then(Value::as_u64)
        .unwrap_or(4)
        .max(1);

    let spec = match build_puzzle_live_game(&doc, card_data) {
        Ok(s) => s,
        Err(e) => return Ok(err_value(e)),
    };
    let refutation_triggers = match parse_refutation_triggers(doc.get("refutation_triggers")) {
        Ok(rt) => rt,
        Err(e) => return Ok(err_value(e)),
    };
    let candidate_lines = match extract_puzzle_lines(&doc) {
        Ok(l) => l,
        Err(e) => return Ok(err_value(e)),
    };

    registry.ensure_card_registry(card_data);
    // Split borrows manually across registry.models and registry.card_registry
    let creg = CardRegistry::from_cards(card_data);

    let mut uniform_eval = UniformEvaluator;
    let (eval_ref, profile): (&mut dyn PolicyValueFn, ObservationProfileId) = match model_id.as_deref() {
        Some(mid) => {
            let Some(cached) = registry.models.get_mut(mid) else {
                return Ok(err_value(format!("unknown model_id: {mid}")));
            };
            (cached.evaluator.as_mut(), cached.info.profile)
        }
        None => (&mut uniform_eval, ObservationProfileId::StandardLiteV2),
    };

    let mut target_results = Vec::new();
    let mut avoid_results = Vec::new();

    for ln in &candidate_lines {
        if ln.role != "target" && ln.role != "avoid" {
            continue;
        }
        let mut goal_met_seeds = 0u64;
        let mut last_winner: Option<u8> = None;
        let mut last_steps_json = Vec::new();
        let mut auto_adv_count = 0usize;
        let mut error_msg: Option<String> = None;

        for s in 0..seeds {
            let params = ScoreLineParams {
                actions: ln.actions.clone(),
                auto_adversary: true,
                adversary_sims,
                adversary_seed: s.wrapping_mul(0x5851f42d4c957f2d),
                refutation_triggers: refutation_triggers.clone(),
                max_adversary_steps: 64,
            };
            match score_line_with_evaluator(
                &spec.live_game.game,
                &creg,
                eval_ref,
                profile,
                &params,
            ) {
                Ok(res) => {
                    let solved = line_satisfies_puzzle_goal(&res.final_game, spec.seat, &doc);
                    if solved {
                        goal_met_seeds += 1;
                    }
                    if s == 0 {
                        last_winner = res.final_winner;
                        auto_adv_count = res.steps.iter().filter(|st| st.auto_adversary).count();
                        last_steps_json = res.steps.iter().map(|st| json!(st)).collect();
                    }
                }
                Err(e) => {
                    error_msg = Some(e);
                    break;
                }
            }
        }

        if ln.role == "target" {
            let passed = error_msg.is_none() && goal_met_seeds == seeds;
            target_results.push(json!({
                "id": ln.id,
                "name": ln.name,
                "passed": passed,
                "passed_seeds": goal_met_seeds,
                "total_seeds": seeds,
                "auto_adversary_steps": auto_adv_count,
                "final_winner": last_winner,
                "error": error_msg,
                "steps": last_steps_json,
            }));
        } else {
            let refuted_seeds = seeds.saturating_sub(goal_met_seeds);
            let passed = error_msg.is_none() && refuted_seeds == seeds;
            avoid_results.push(json!({
                "id": ln.id,
                "name": ln.name,
                "passed": passed,
                "refuted": passed,
                "refuted_seeds": refuted_seeds,
                "total_seeds": seeds,
                "auto_adversary_steps": auto_adv_count,
                "final_winner": last_winner,
                "error": error_msg,
                "steps": last_steps_json,
            }));
        }
    }

    let mut expectation_results = Vec::new();
    let expectations = doc
        .get("expectations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !expectations.is_empty() && model_id.is_some() {
        let mut pass_counts = vec![0u64; expectations.len()];
        let mut first_details = vec![String::new(); expectations.len()];
        let search_sims = doc
            .get("search")
            .and_then(|s| s.get("simulations"))
            .and_then(Value::as_u64)
            .map(|n| n as u32)
            .unwrap_or_else(|| adversary_sims.max(1));
        let search_worlds = doc
            .get("search")
            .and_then(|s| s.get("worlds"))
            .and_then(Value::as_u64)
            .map(|n| n as u32)
            .unwrap_or(2);
        for s in 0..seeds {
            let ip_params = InspectPositionParams {
                profile,
                simulations: search_sims,
                worlds: search_worlds,
                seed: s.wrapping_mul(0x5851f42d4c957f2d),
                regime: "open_list".to_string(),
                top_k: 12,
                expectations: expectations.clone(),
            };
            if let Ok(ip_val) =
                inspect_position_with_evaluator(&spec.live_game.game, &creg, eval_ref, &ip_params)
            {
                if let Some(arr) = ip_val.get("expectation_results").and_then(Value::as_array) {
                    for (i, er) in arr.iter().enumerate() {
                        if er.get("passed").and_then(Value::as_bool) == Some(true) {
                            pass_counts[i] += 1;
                        }
                        if s == 0 {
                            first_details[i] = er
                                .get("detail")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string();
                        }
                    }
                }
            }
        }
        for (i, exp) in expectations.iter().enumerate() {
            expectation_results.push(json!({
                "kind": exp.get("kind").and_then(Value::as_str).unwrap_or(""),
                "plan": exp.get("plan").cloned().unwrap_or_else(|| json!([])),
                "passed": pass_counts[i] == seeds,
                "passed_seeds": pass_counts[i],
                "total_seeds": seeds,
                "detail": first_details[i],
            }));
        }
    }

    let verified = target_results
        .iter()
        .all(|r| r.get("passed").and_then(Value::as_bool) == Some(true))
        && avoid_results
            .iter()
            .all(|r| r.get("passed").and_then(Value::as_bool) == Some(true));

    Ok(json!({
        "ok": true,
        "verified": verified,
        "target_results": target_results,
        "avoid_results": avoid_results,
        "expectation_results": expectation_results,
    }))
}

// ── tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_includes_42_tools() {
        let tools = list();
        // Expected count: 33 baseline tools + 9 Phase 1A/1B tools
        // (load_model, unload_model, list_models, inspect_position,
        // score_line, inspect_stack, load_puzzle, save_puzzle, verify_puzzle) = 42.
        assert_eq!(tools.len(), 42, "tool count drifted");
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
        assert!(
            view["delta"].is_object(),
            "forward step should carry a delta"
        );
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
        let seek = dispatch("seek", json!({ "game_id": id, "step_n": 3 }), &mut reg, &db).unwrap();
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
        assert!(resp["error"]
            .as_str()
            .unwrap()
            .contains("not constructed from a recording"));
    }
}
