//! Tauri commands that expose the Rust `digimon-engine` directly to the
//! frontend. These are the sole gameplay backend on desktop — there is no
//! Python sidecar; ONNX inference runs in-process via `InferenceState`.
//! Response shapes mirror the hosted API's `/games/*` router so the web
//! and desktop frontends share one set of TypeScript types.
//!
//! All commands mutate shared state behind a `Mutex<Option<Game>>`. The
//! frontend calls `create_test_game` first, then uses `get_state` /
//! `play_card` / `attack_digimon` / `attack_player` / `end_turn` to drive it.

use std::collections::HashMap;
use std::sync::Mutex;

use digimon_engine::action::build_action_mask;
use digimon_engine::action::explain::{explain_action, ActionExplanation};
use digimon_engine::card_data::CardData;
use digimon_engine::card_registry::CardRegistry;
use digimon_engine::combat::AttackResult;
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlayerId};
use digimon_engine::game::Game;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::rules::Rules;
use digimon_engine::tensor::build_tensor;
use digimon_engine::tensor_profiles::default_profile;
use serde::{Deserialize, Serialize};

use crate::inference_state::InferenceState;

/// Per-player game configuration — which kind of decider drives each seat,
/// and (for `Trained`) which loaded model to consult.
///
/// Lock order invariant: always acquire `RustEngineState::game` before
/// `RustEngineState::session` — avoids deadlock between the two.
#[derive(Default)]
pub struct GameSession {
    pub registry: Option<CardRegistry>,
    pub player_kinds: Vec<PlayerKind>,
    pub player_model_ids: Vec<Option<String>>,
}

/// Shared mutable Rust-engine state, held by Tauri.
#[derive(Default)]
pub struct RustEngineState {
    pub game: Mutex<Option<Game>>,
    pub session: Mutex<GameSession>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PlayerKind {
    Human,
    Greedy,
    Trained,
}

impl Default for PlayerKind {
    fn default() -> Self {
        PlayerKind::Human
    }
}

// ─── DTOs ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardDto {
    pub card_id: String,
    pub card_name: String,
    pub card_kind: String,
    pub level: Option<u8>,
    pub dp: Option<i32>,
    pub play_cost: u16,
    pub colors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermanentDto {
    pub field_index: u8,
    pub top_card: CardDto,
    pub effective_dp: Option<i32>,
    pub is_suspended: bool,
    pub stack_size: usize,
    pub turn_played: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerDto {
    pub id: PlayerId,
    pub hand: Vec<CardDto>,
    pub battle_area: Vec<PermanentDto>,
    pub breeding: Option<PermanentDto>,
    pub deck_count: usize,
    pub trash_count: usize,
    pub security_count: usize,
    pub is_eliminated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStateDto {
    pub turn_count: u16,
    pub turn_player: PlayerId,
    pub current_phase: String,
    pub memory: i16,
    pub game_over: bool,
    pub winner: Option<PlayerId>,
    pub players: Vec<PlayerDto>,
    /// The player expected to make the next mulligan decision. `None` once
    /// mulligan is finalized (i.e., during every normal phase of play).
    pub mulligan_current_player: Option<PlayerId>,
    /// Whether each player has used their one re-draw. Indexed by player id.
    /// During mulligan, the UI hides the "Mulligan" button for a player whose
    /// entry here is `true`.
    pub mulligan_used: Vec<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackResultDto {
    pub result: String,
    pub state: GameStateDto,
}

// ─── DTO builders ─────────────────────────────────────────────────────

fn card_kind_str(k: CardKind) -> &'static str {
    match k {
        CardKind::Digimon => "Digimon",
        CardKind::Tamer => "Tamer",
        CardKind::Option => "Option",
        CardKind::DigiEgg => "DigiEgg",
        CardKind::Token => "Token",
        CardKind::Dual => "Dual",
    }
}

fn color_str(c: CardColor) -> &'static str {
    match c {
        CardColor::Red => "Red",
        CardColor::Blue => "Blue",
        CardColor::Yellow => "Yellow",
        CardColor::Green => "Green",
        CardColor::Black => "Black",
        CardColor::Purple => "Purple",
        CardColor::White => "White",
    }
}

fn phase_str(p: GamePhase) -> &'static str {
    match p {
        GamePhase::Mulligan => "Mulligan",
        GamePhase::Unsuspend => "Unsuspend",
        GamePhase::Draw => "Draw",
        GamePhase::Breeding => "Breeding",
        GamePhase::Main => "Main",
        GamePhase::EndTurn => "EndTurn",
        GamePhase::SelectTarget => "SelectTarget",
        GamePhase::SelectMaterial => "SelectMaterial",
        GamePhase::SelectTrash => "SelectTrash",
        GamePhase::SelectSource => "SelectSource",
        GamePhase::SelectHand => "SelectHand",
        GamePhase::SelectReveal => "SelectReveal",
        GamePhase::SelectSecurity => "SelectSecurity",
        GamePhase::EffectChoice => "EffectChoice",
        GamePhase::BlockTiming => "BlockTiming",
        GamePhase::CounterTiming => "CounterTiming",
        GamePhase::AllianceTiming => "AllianceTiming",
        GamePhase::EndOfTurnAction => "EndOfTurnAction",
        GamePhase::GameOver => "GameOver",
        GamePhase::SelectUnion => "SelectUnion",
        GamePhase::SelectPermutation => "SelectPermutation",
        GamePhase::SelectBudgeted => "SelectBudgeted",
    }
}

fn attack_result_str(r: AttackResult) -> &'static str {
    match r {
        AttackResult::Invalid => "Invalid",
        AttackResult::AttackerWins => "AttackerWins",
        AttackResult::DefenderWins => "DefenderWins",
        AttackResult::MutualDestruction => "MutualDestruction",
        AttackResult::SecurityCheckSurvived => "SecurityCheckSurvived",
        AttackResult::AttackerDeletedBySecurity => "AttackerDeletedBySecurity",
        AttackResult::GameWon => "GameWon",
        AttackResult::InProgress => "InProgress",
        AttackResult::Cancelled => "Cancelled",
    }
}

fn card_dto(card: &digimon_engine::card_source::CardSource, data: &[CardData]) -> CardDto {
    let d = &data[card.data_index];
    CardDto {
        card_id: d.card_id.clone(),
        card_name: d.card_name.clone(),
        card_kind: card_kind_str(d.card_kind).to_string(),
        level: d.level,
        dp: d.dp,
        play_cost: d.play_cost,
        colors: d.colors.iter().map(|&c| color_str(c).to_string()).collect(),
    }
}

fn perm_dto(game: &Game, player: PlayerId, index: usize) -> PermanentDto {
    let perm = &game.player(player).battle_area[index];
    let handle = PermanentHandle {
        player,
        index: index as u8,
    };
    PermanentDto {
        field_index: index as u8,
        top_card: card_dto(perm.top_card(), &game.card_data),
        effective_dp: game.effective_dp(handle),
        is_suspended: perm.is_suspended,
        stack_size: perm.stack_size(),
        turn_played: perm.turn_played,
    }
}

fn player_dto(game: &Game, id: PlayerId) -> PlayerDto {
    let p = game.player(id);
    let battle_area: Vec<PermanentDto> = (0..p.battle_area.len())
        .map(|i| perm_dto(game, id, i))
        .collect();
    let hand: Vec<CardDto> = p
        .hand
        .iter()
        .map(|c| card_dto(c, &game.card_data))
        .collect();
    let breeding = p.breeding_area.as_ref().map(|perm| PermanentDto {
        field_index: 255, // breeding indicator
        top_card: card_dto(perm.top_card(), &game.card_data),
        effective_dp: perm.base_dp(&game.card_data),
        is_suspended: perm.is_suspended,
        stack_size: perm.stack_size(),
        turn_played: perm.turn_played,
    });
    PlayerDto {
        id,
        hand,
        battle_area,
        breeding,
        deck_count: p.deck.len(),
        trash_count: p.trash.len(),
        security_count: p.security.len(),
        is_eliminated: p.is_eliminated,
    }
}

fn game_state_dto(game: &Game) -> GameStateDto {
    let players: Vec<PlayerDto> = (0..game.rules.player_count)
        .map(|i| player_dto(game, i))
        .collect();
    GameStateDto {
        turn_count: game.turn_count,
        turn_player: game.turn_player(),
        current_phase: phase_str(game.current_phase).to_string(),
        memory: game.memory,
        game_over: game.game_over,
        winner: game.winner,
        players,
        mulligan_current_player: game.mulligan_current_player(),
        mulligan_used: game.mulligan_used.clone(),
    }
}

// ─── Test card database ───────────────────────────────────────────────

fn synth_card(id: &str, name: &str, kind: CardKind, dp: Option<i32>, cost: u16) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: name.to_string(),
        card_kind: kind,
        level: match kind {
            CardKind::DigiEgg => Some(2),
            CardKind::Digimon => Some(3),
            _ => None,
        },
        dp,
        play_cost: cost,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        keywords: Vec::new(),
        dual: None,
    }
}

/// Build a tiny test card database that works with the engine's built-in
/// TEST-001..005 effects plus some vanilla Digimon for combat.
///
/// `pub` so integration tests can create games using the same card pool.
pub fn test_card_db() -> HashMap<String, CardData> {
    let cards = vec![
        synth_card("TEST-001", "Memory Boost", CardKind::Digimon, Some(2000), 3),
        synth_card("TEST-002", "Draw Power", CardKind::Digimon, Some(2000), 3),
        synth_card("TEST-003", "Buff Captain", CardKind::Digimon, Some(2000), 4),
        synth_card("TEST-004", "Opp Scout", CardKind::Digimon, Some(3000), 4),
        synth_card("TEST-005", "Last Stand", CardKind::Digimon, Some(3000), 4),
        synth_card("VANILLA-3K", "Vanilla 3K", CardKind::Digimon, Some(3000), 3),
        synth_card("VANILLA-5K", "Vanilla 5K", CardKind::Digimon, Some(5000), 5),
        synth_card("EGG-01", "Testmon Egg", CardKind::DigiEgg, None, 0),
    ];
    let mut map = HashMap::new();
    for c in cards {
        map.insert(c.card_id.clone(), c);
    }
    map
}

/// `pub` so integration tests can create standard test decks.
pub fn test_deck() -> Vec<String> {
    // 50 main deck + 5 eggs.
    let mut deck = Vec::with_capacity(55);
    let mains = [
        "TEST-001",
        "TEST-002",
        "TEST-003",
        "TEST-004",
        "TEST-005",
        "VANILLA-3K",
        "VANILLA-5K",
    ];
    // Fill to ~50 cards by repeating.
    for i in 0..50 {
        deck.push(mains[i % mains.len()].to_string());
    }
    for _ in 0..5 {
        deck.push("EGG-01".to_string());
    }
    deck
}

// ─── Tauri commands ───────────────────────────────────────────────────

/// Create a new 2-player game with built-in test cards.
#[tauri::command]
pub fn create_test_game(state: tauri::State<'_, RustEngineState>) -> Result<GameStateDto, String> {
    let db = test_card_db();
    let decks = vec![test_deck(), test_deck()];
    let mut game = Game::new(&decks, &db, Rules::standard(), Some(42))
        .map_err(|e| format!("Game::new failed: {}", e))?;
    game.start_game();
    let dto = game_state_dto(&game);
    *state.game.lock().map_err(|e| e.to_string())? = Some(game);
    Ok(dto)
}

/// Get the current game state as JSON.
#[tauri::command]
pub fn get_rust_game_state(
    state: tauri::State<'_, RustEngineState>,
) -> Result<GameStateDto, String> {
    let guard = state.game.lock().map_err(|e| e.to_string())?;
    let game = guard.as_ref().ok_or("No active game")?;
    Ok(game_state_dto(game))
}

/// Play a card from a player's hand.
#[tauri::command]
pub fn rust_play_card(
    state: tauri::State<'_, RustEngineState>,
    player_id: PlayerId,
    hand_index: usize,
) -> Result<GameStateDto, String> {
    let mut guard = state.game.lock().map_err(|e| e.to_string())?;
    let game = guard.as_mut().ok_or("No active game")?;
    let field_index = game
        .play_from_hand(player_id, hand_index)
        .ok_or("Cannot play that card (invalid hand index or field is full)")?;
    let _ = field_index; // currently unused in the response
    Ok(game_state_dto(game))
}

/// Attack an opposing Digimon.
#[tauri::command]
pub fn rust_attack_digimon(
    state: tauri::State<'_, RustEngineState>,
    attacker_player: PlayerId,
    attacker_index: u8,
    defender_player: PlayerId,
    defender_index: u8,
) -> Result<AttackResultDto, String> {
    let mut guard = state.game.lock().map_err(|e| e.to_string())?;
    let game = guard.as_mut().ok_or("No active game")?;
    let attacker = PermanentHandle {
        player: attacker_player,
        index: attacker_index,
    };
    let defender = PermanentHandle {
        player: defender_player,
        index: defender_index,
    };
    // Tauri UI only drives Main-phase attacks today; <Vortex> end-of-turn
    // attacks will get a dedicated command once §4.6 mask coverage lands.
    let result = game.attack_digimon(attacker, defender, false);
    Ok(AttackResultDto {
        result: attack_result_str(result).to_string(),
        state: game_state_dto(game),
    })
}

/// Attack the opposing player (security check).
#[tauri::command]
pub fn rust_attack_player(
    state: tauri::State<'_, RustEngineState>,
    attacker_player: PlayerId,
    attacker_index: u8,
    defender_player: PlayerId,
) -> Result<AttackResultDto, String> {
    let mut guard = state.game.lock().map_err(|e| e.to_string())?;
    let game = guard.as_mut().ok_or("No active game")?;
    let attacker = PermanentHandle {
        player: attacker_player,
        index: attacker_index,
    };
    let result = game.attack_player(attacker, defender_player, false);
    Ok(AttackResultDto {
        result: attack_result_str(result).to_string(),
        state: game_state_dto(game),
    })
}

/// End the current turn.
#[tauri::command]
pub fn rust_end_turn(state: tauri::State<'_, RustEngineState>) -> Result<GameStateDto, String> {
    let mut guard = state.game.lock().map_err(|e| e.to_string())?;
    let game = guard.as_mut().ok_or("No active game")?;
    game.end_turn();
    Ok(game_state_dto(game))
}

/// Pass turn (memory to -3, then end turn).
#[tauri::command]
pub fn rust_pass_turn(state: tauri::State<'_, RustEngineState>) -> Result<GameStateDto, String> {
    let mut guard = state.game.lock().map_err(|e| e.to_string())?;
    let game = guard.as_mut().ok_or("No active game")?;
    game.pass_turn();
    Ok(game_state_dto(game))
}

/// Apply a mulligan decision for the currently-deciding player.
/// `keep = true` keeps the opening hand; `keep = false` shuffles it back
/// and redraws the same count. Returns the updated state.
#[tauri::command]
pub fn rust_mulligan_decide(
    state: tauri::State<'_, RustEngineState>,
    keep: bool,
) -> Result<GameStateDto, String> {
    let mut guard = state.game.lock().map_err(|e| e.to_string())?;
    let game = guard.as_mut().ok_or("No active game")?;
    let p = game
        .mulligan_current_player()
        .ok_or("Mulligan is already complete")?;
    game.accept_mulligan(p, keep).map_err(|e| e.to_string())?;
    Ok(game_state_dto(game))
}

/// Hatch: move top egg to breeding area.
#[tauri::command]
pub fn rust_hatch(
    state: tauri::State<'_, RustEngineState>,
    player_id: PlayerId,
) -> Result<GameStateDto, String> {
    let mut guard = state.game.lock().map_err(|e| e.to_string())?;
    let game = guard.as_mut().ok_or("No active game")?;
    if !game.hatch(player_id) {
        return Err("Cannot hatch (no egg or breeding occupied)".into());
    }
    Ok(game_state_dto(game))
}

/// Move from breeding area to battle area.
#[tauri::command]
pub fn rust_move_from_breeding(
    state: tauri::State<'_, RustEngineState>,
    player_id: PlayerId,
) -> Result<GameStateDto, String> {
    let mut guard = state.game.lock().map_err(|e| e.to_string())?;
    let game = guard.as_mut().ok_or("No active game")?;
    if !game.move_from_breeding(player_id) {
        return Err("Cannot move from breeding".into());
    }
    Ok(game_state_dto(game))
}

// ─── Action-ID dispatch envelope (parity with hosted-API shape) ───
//
// The frontend's `gameApi.ts` speaks in flat action IDs and expects
// responses shaped like the hosted API's `ActionResponse`. These
// commands and DTOs let desktop and web callers bind against one set of
// TypeScript types regardless of which backend is serving the request.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEventDto {
    #[serde(rename = "type")]
    pub event_type: String,
    pub seq: u32,
    pub player: i32,
    pub source_card_id: Option<String>,
    pub source_slot: Option<i32>,
    pub target_card_id: Option<String>,
    pub target_slot: Option<i32>,
    pub meta: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorSummaryDto {
    pub player_id: PlayerId,
    pub profile_id: String,
    pub profile_version: u16,
    pub tensor_size: usize,
    pub mask_size: usize,
    pub legal_action_count: usize,
    pub card_id_slot_count: usize,
    pub scalar_slot_count: usize,
    pub turn_count: u16,
    pub phase: String,
    pub memory: i16,
    pub tensor_head: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionTraceDto {
    pub actor: String,
    pub player_id: PlayerId,
    pub action_id: u16,
    pub decoded: ActionExplanation,
    pub tensor_summary: Option<TensorSummaryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResponseDto {
    pub state: GameStateDto,
    pub action_mask: Vec<u8>,
    pub is_game_over: bool,
    pub logs: Vec<String>,
    pub events: Vec<GameEventDto>,
    pub action_context: serde_json::Value,
    pub action_traces: Vec<ActionTraceDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGameResponseDto {
    pub game_id: String,
    pub state: GameStateDto,
    pub action_mask: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResponseDto {
    pub state: GameStateDto,
    pub action_mask: Vec<u8>,
    pub logs: Vec<String>,
    pub events: Vec<GameEventDto>,
    pub is_human_turn: bool,
    pub is_game_over: bool,
    pub action_traces: Vec<ActionTraceDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurrenderResponseDto {
    pub state: GameStateDto,
    pub action_mask: Vec<u8>,
    pub logs: Vec<String>,
    pub events: Vec<GameEventDto>,
    pub is_game_over: bool,
    pub surrendered_by: PlayerId,
}

/// Mirror of HeadlessRunner::current_decision_player — who is expected to
/// submit the next action.
fn current_decision_player(game: &Game) -> PlayerId {
    if let Some(p) = game.mulligan_current_player() {
        return p;
    }
    if let Some(sel) = game.pending_selection.as_ref() {
        return sel.selecting_player;
    }
    game.turn_player()
}

/// Build a `u8` action mask for the current decider. The frontend stores
/// `number[]`; `0`/`1` bytes round-trip transparently through Tauri/serde.
fn action_mask_bytes(game: &Game) -> Vec<u8> {
    let pid = current_decision_player(game);
    build_action_mask(game, pid)
        .into_iter()
        .map(|v| if v > 0.0 { 1u8 } else { 0u8 })
        .collect()
}

/// Drain structured gameplay events emitted by the engine during the last
/// action. The Rust engine does not yet emit `GameEvent`s (animations in
/// Rust mode will be a no-op until a follow-up milestone adds this). For now
/// we return an empty vector so the response shape matches Python's.
fn drain_events(_game: &mut Game) -> Vec<GameEventDto> {
    Vec::new()
}

/// Ensure a game exists (auto-seed a test game on first call) and return a
/// mutable reference. Lets the frontend skip the explicit create-game step
/// during rapid iteration.
fn ensure_game<'a>(
    guard: &'a mut std::sync::MutexGuard<'_, Option<Game>>,
) -> Result<&'a mut Game, String> {
    if guard.is_none() {
        let db = test_card_db();
        let decks = vec![test_deck(), test_deck()];
        let mut game = Game::new(&decks, &db, Rules::standard(), Some(42))
            .map_err(|e| format!("Game::new failed: {}", e))?;
        game.start_game();
        **guard = Some(game);
    }
    guard.as_mut().ok_or_else(|| "No active game".to_string())
}

/// Create a new local game using the built-in test card database. Accepts
/// optional deck lists; when absent, uses the standard test decks. Accepts
/// optional per-player kinds (`human` / `greedy` / `trained`) and model IDs
/// (one per player, only honored for `trained`). When both are omitted, both
/// seats default to `human`.
///
/// Player indices in the request are 0-based (matching the Rust engine's
/// `PlayerId` convention).
#[tauri::command]
pub fn rust_create_game(
    state: tauri::State<'_, RustEngineState>,
    inference: tauri::State<'_, InferenceState>,
    deck1: Option<Vec<String>>,
    deck2: Option<Vec<String>>,
    player_kinds: Option<Vec<PlayerKind>>,
    player_model_ids: Option<Vec<Option<String>>>,
) -> Result<CreateGameResponseDto, String> {
    let db = test_card_db();
    let decks = vec![
        deck1.unwrap_or_else(test_deck),
        deck2.unwrap_or_else(test_deck),
    ];
    let player_count = decks.len();
    let mut game = Game::new(&decks, &db, Rules::standard(), Some(42))
        .map_err(|e| format!("Game::new failed: {}", e))?;
    game.start_game();

    let kinds = normalize_player_kinds(player_kinds, player_count)?;
    let model_ids = normalize_player_model_ids(player_model_ids, player_count, &kinds)?;
    validate_models_loaded(&inference, &kinds, &model_ids)?;
    // Fresh episode — reset LSTM state on every model any player will drive.
    // Silent for models the user passed but never loaded; the predict path
    // will surface that as a clean error instead.
    for model_id in model_ids.iter().flatten() {
        let _ = inference.reset(model_id);
    }

    let registry = CardRegistry::from_cards(&db);
    let mask = action_mask_bytes(&game);
    let dto = game_state_dto(&game);

    {
        let mut game_guard = state.game.lock().map_err(|e| e.to_string())?;
        let mut session_guard = state.session.lock().map_err(|e| e.to_string())?;
        *game_guard = Some(game);
        *session_guard = GameSession {
            registry: Some(registry),
            player_kinds: kinds,
            player_model_ids: model_ids,
        };
    }

    Ok(CreateGameResponseDto {
        game_id: "rust-local".to_string(),
        state: dto,
        action_mask: mask,
    })
}

fn normalize_player_kinds(
    provided: Option<Vec<PlayerKind>>,
    player_count: usize,
) -> Result<Vec<PlayerKind>, String> {
    match provided {
        None => Ok(vec![PlayerKind::Human; player_count]),
        Some(v) if v.len() == player_count => Ok(v),
        Some(v) => Err(format!(
            "player_kinds length {} does not match player count {}",
            v.len(),
            player_count
        )),
    }
}

fn normalize_player_model_ids(
    provided: Option<Vec<Option<String>>>,
    player_count: usize,
    kinds: &[PlayerKind],
) -> Result<Vec<Option<String>>, String> {
    let v = match provided {
        None => vec![None; player_count],
        Some(v) if v.len() == player_count => v,
        Some(v) => {
            return Err(format!(
                "player_model_ids length {} does not match player count {}",
                v.len(),
                player_count
            ));
        }
    };
    for (i, (kind, model_id)) in kinds.iter().zip(v.iter()).enumerate() {
        if *kind == PlayerKind::Trained && model_id.is_none() {
            return Err(format!(
                "player {i} is 'trained' but no model_id was provided"
            ));
        }
    }
    Ok(v)
}

fn validate_models_loaded(
    inference: &InferenceState,
    kinds: &[PlayerKind],
    model_ids: &[Option<String>],
) -> Result<(), String> {
    for (i, (kind, model_id)) in kinds.iter().zip(model_ids.iter()).enumerate() {
        if *kind != PlayerKind::Trained {
            continue;
        }
        let id = model_id.as_deref().expect("trained kind without model_id");
        if !inference.is_loaded(id)? {
            return Err(format!(
                "player {i} requested model '{id}' which is not loaded; \
                 call rust_load_model first"
            ));
        }
    }
    Ok(())
}

/// Execute a flat RL-action-ID against the current game. Mirrors the
/// hosted API's `POST /games/{id}/actions` surface so the frontend's
/// `gameApi.ts` can bind against this without knowing which backend is
/// running.
///
/// After the human's action lands we also drain any agent turns that
/// immediately follow, so the response reflects the next state the
/// player needs to see — same pattern the hosted API uses in
/// `InteractiveGame.step`.
#[tauri::command]
pub fn rust_submit_action(
    state: tauri::State<'_, RustEngineState>,
    inference: tauri::State<'_, InferenceState>,
    action: u32,
) -> Result<ActionResponseDto, String> {
    let mut game_guard = state.game.lock().map_err(|e| e.to_string())?;
    let session_guard = state.session.lock().map_err(|e| e.to_string())?;
    let game = ensure_game(&mut game_guard)?;
    let pid = current_decision_player(game);
    let action_u16 = u16::try_from(action)
        .map_err(|_| format!("action {action} is out of range for a u16 action ID"))?;
    let mask_before = build_action_mask(game, pid);
    let human_trace = action_trace_for(
        game,
        "human",
        pid,
        action_u16,
        optional_tensor_summary_for(game, pid, session_guard.registry.as_ref(), &mask_before),
    );
    game.decode_action(action_u16, pid);
    let mut action_traces = vec![human_trace];
    action_traces.extend(run_agent_steps(game, &session_guard, &inference)?);
    let events = drain_events(game);
    let mask = action_mask_bytes(game);
    let is_over = game.game_over;
    Ok(ActionResponseDto {
        state: game_state_dto(game),
        action_mask: mask,
        is_game_over: is_over,
        logs: Vec::new(),
        events,
        action_context: serde_json::json!({}),
        action_traces,
    })
}

/// Drive the game forward until a human decision is required, or the game
/// ends. Each loop iteration inspects the current decider's `PlayerKind`:
///
/// - `Human`: stop — frontend should render and await the next `rust_submit_action`.
/// - `Greedy`: pick the first valid action (deterministic heuristic used for
///   local testing until a stronger built-in bot lands).
/// - `Trained`: run the ONNX policy attached to that player's `model_id`.
///
/// The response's `is_human_turn` reflects state *after* the loop: `true`
/// means the frontend should wait for user input, `false` means the game
/// ended while the last decider was still an agent.
#[tauri::command]
pub fn rust_step_game(
    state: tauri::State<'_, RustEngineState>,
    inference: tauri::State<'_, InferenceState>,
) -> Result<StepResponseDto, String> {
    let mut game_guard = state.game.lock().map_err(|e| e.to_string())?;
    let session_guard = state.session.lock().map_err(|e| e.to_string())?;
    let game = ensure_game(&mut game_guard)?;
    let action_traces = run_agent_steps(game, &session_guard, &inference)?;
    let mask = action_mask_bytes(game);
    let pid = current_decision_player(game);
    let is_human_turn =
        matches!(decider_kind(&session_guard, pid), PlayerKind::Human) || game.game_over;
    let is_over = game.game_over;
    Ok(StepResponseDto {
        state: game_state_dto(game),
        action_mask: mask,
        logs: Vec::new(),
        events: Vec::new(),
        is_human_turn,
        is_game_over: is_over,
        action_traces,
    })
}

fn decider_kind(session: &GameSession, pid: PlayerId) -> PlayerKind {
    session
        .player_kinds
        .get(pid as usize)
        .copied()
        .unwrap_or(PlayerKind::Human)
}

/// Step the game forward while the current decider is a non-human agent.
/// Bail out as soon as the game ends or a human seat is up — matches the
/// hosted API's `InteractiveGame.run_step` contract so the frontend
/// state machine doesn't care which backend is driving.
///
/// `pub` so integration tests under `tests/` can drive the game loop
/// directly without going through the Tauri IPC layer.
pub fn run_agent_steps(
    game: &mut Game,
    session: &GameSession,
    inference: &InferenceState,
) -> Result<Vec<ActionTraceDto>, String> {
    // Cap iterations defensively so a bug in mask generation can't turn this
    // into an infinite spin. Normal games resolve in far fewer than this.
    const MAX_AGENT_STEPS: usize = 10_000;
    let mut traces = Vec::new();
    for _ in 0..MAX_AGENT_STEPS {
        if game.game_over {
            return Ok(traces);
        }
        let pid = current_decision_player(game);
        let kind = decider_kind(session, pid);
        let (actor, action) = match kind {
            PlayerKind::Human => return Ok(traces),
            PlayerKind::Greedy => {
                let mask_before = build_action_mask(game, pid);
                (
                    "agent_greedy",
                    digimon_engine::policies::greedy_action(game, &mask_before) as usize,
                )
            }
            PlayerKind::Trained => {
                let model_id = session
                    .player_model_ids
                    .get(pid as usize)
                    .and_then(|m| m.as_deref())
                    .ok_or_else(|| format!("trained agent for player {pid} has no model_id"))?;
                let registry = session.registry.as_ref().ok_or_else(|| {
                    "inference: session has no card registry (game not created?)".to_string()
                })?;
                let obs = build_tensor(game, pid, registry);
                let mask_before = build_action_mask(game, pid);
                validate_shapes(&obs, &mask_before, model_id)?;
                (
                    "agent_trained",
                    inference.predict(model_id, &obs, &mask_before)?,
                )
            }
        };
        let action_u16 = u16::try_from(action)
            .map_err(|_| format!("agent returned out-of-range action {action}"))?;
        let mask_before = build_action_mask(game, pid);
        traces.push(action_trace_for(
            game,
            actor,
            pid,
            action_u16,
            optional_tensor_summary_for(game, pid, session.registry.as_ref(), &mask_before),
        ));
        game.decode_action(action_u16, pid);
    }
    Err(format!(
        "agent step loop exceeded {MAX_AGENT_STEPS} iterations; possible mask bug"
    ))
}

/// Sanity-check obs/mask sizes before feeding them to the policy so a
/// shape mismatch shows up here (with a clear error) rather than inside
/// the ONNX session.
fn validate_shapes(obs: &[f32], mask: &[f32], model_id: &str) -> Result<(), String> {
    if obs.len() != digimon_engine::tensor::TENSOR_SIZE {
        return Err(format!(
            "model '{model_id}' obs length {} != engine TENSOR_SIZE {}",
            obs.len(),
            digimon_engine::tensor::TENSOR_SIZE
        ));
    }
    if mask.len() != digimon_engine::action::space::ACTION_SPACE_SIZE {
        return Err(format!(
            "model '{model_id}' mask length {} != engine ACTION_SPACE_SIZE {}",
            mask.len(),
            digimon_engine::action::space::ACTION_SPACE_SIZE
        ));
    }
    Ok(())
}

fn tensor_summary_for(
    game: &Game,
    player_id: PlayerId,
    registry: &CardRegistry,
    mask: &[f32],
) -> TensorSummaryDto {
    let tensor = build_tensor(game, player_id, registry);
    let profile = default_profile();
    TensorSummaryDto {
        player_id,
        profile_id: profile.id.to_string(),
        profile_version: profile.version as u16,
        tensor_size: tensor.len(),
        mask_size: mask.len(),
        legal_action_count: mask.iter().filter(|&&v| v > 0.0).count(),
        card_id_slot_count: profile.card_id_slot_count,
        scalar_slot_count: profile.scalar_slot_count,
        turn_count: game.turn_count,
        phase: format!("{:?}", game.current_phase),
        memory: game.memory,
        tensor_head: tensor.iter().take(16).copied().collect(),
    }
}

fn optional_tensor_summary_for(
    game: &Game,
    player_id: PlayerId,
    registry: Option<&CardRegistry>,
    mask: &[f32],
) -> Option<TensorSummaryDto> {
    registry.map(|registry| tensor_summary_for(game, player_id, registry, mask))
}

fn action_trace_for(
    game: &Game,
    actor: &str,
    player_id: PlayerId,
    action_id: u16,
    tensor_summary: Option<TensorSummaryDto>,
) -> ActionTraceDto {
    ActionTraceDto {
        actor: actor.to_string(),
        player_id,
        action_id,
        decoded: explain_action(game, player_id, action_id),
        tensor_summary,
    }
}

/// Read the current action mask.
#[tauri::command]
pub fn rust_get_mask(state: tauri::State<'_, RustEngineState>) -> Result<Vec<u8>, String> {
    let guard = state.game.lock().map_err(|e| e.to_string())?;
    let game = guard.as_ref().ok_or("No active game")?;
    Ok(action_mask_bytes(game))
}

#[tauri::command]
pub fn rust_get_board_tensor_summary(
    state: tauri::State<'_, RustEngineState>,
    player_id: PlayerId,
) -> Result<TensorSummaryDto, String> {
    let game_guard = state.game.lock().map_err(|e| e.to_string())?;
    let session_guard = state.session.lock().map_err(|e| e.to_string())?;
    let game = game_guard.as_ref().ok_or("No active game")?;
    let registry = session_guard.registry.as_ref().ok_or_else(|| {
        "tensor summary: session has no card registry (game not created?)".to_string()
    })?;
    let mask = build_action_mask(game, player_id);
    Ok(tensor_summary_for(game, player_id, registry, &mask))
}

/// Read the accumulated log (empty for now — Rust engine doesn't log yet).
#[tauri::command]
pub fn rust_get_log(state: tauri::State<'_, RustEngineState>) -> Result<Vec<String>, String> {
    let _guard = state.game.lock().map_err(|e| e.to_string())?;
    Ok(Vec::new())
}

/// Surrender ends the game with the opposing player as winner.
#[tauri::command]
pub fn rust_surrender(
    state: tauri::State<'_, RustEngineState>,
    player_id: PlayerId,
) -> Result<SurrenderResponseDto, String> {
    let mut guard = state.game.lock().map_err(|e| e.to_string())?;
    let game = guard.as_mut().ok_or("No active game")?;
    let winner = if player_id == 0 { 1 } else { 0 };
    game.declare_winner(winner);
    let mask = action_mask_bytes(game);
    Ok(SurrenderResponseDto {
        state: game_state_dto(game),
        action_mask: mask,
        logs: Vec::new(),
        events: Vec::new(),
        is_game_over: true,
        surrendered_by: player_id,
    })
}

/// Delete the active game (used by the frontend cleanup path). Clears the
/// session (player kinds, model-id bindings, card registry) at the same
/// time — but loaded ONNX policies stay in the inference cache since the
/// next game will likely reuse them.
#[tauri::command]
pub fn rust_delete_game(state: tauri::State<'_, RustEngineState>) -> Result<(), String> {
    let mut game_guard = state.game.lock().map_err(|e| e.to_string())?;
    let mut session_guard = state.session.lock().map_err(|e| e.to_string())?;
    *game_guard = None;
    *session_guard = GameSession::default();
    Ok(())
}

/// Load an ONNX model into the inference cache, keyed by `model_id`. This
/// is the low-level command the Phase-C model manager will wrap — for now
/// the frontend can call it directly with a filesystem path for testing.
/// Idempotent: reloading the same `model_id` replaces the previous entry.
#[tauri::command]
pub fn rust_load_model(
    inference: tauri::State<'_, InferenceState>,
    model_id: String,
    onnx_path: String,
) -> Result<(), String> {
    let path = std::path::PathBuf::from(&onnx_path);
    if !path.exists() {
        return Err(format!("ONNX file not found: {onnx_path}"));
    }
    inference.load(model_id, &path)
}

/// Unload a model from the cache, freeing its ONNX session.
#[tauri::command]
pub fn rust_unload_model(
    inference: tauri::State<'_, InferenceState>,
    model_id: String,
) -> Result<bool, String> {
    inference.unload(&model_id)
}

/// List currently-loaded model IDs.
#[tauri::command]
pub fn rust_list_loaded_models(
    inference: tauri::State<'_, InferenceState>,
) -> Result<Vec<String>, String> {
    inference.loaded_ids()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dto_roundtrip_through_json() {
        let db = test_card_db();
        let decks = vec![test_deck(), test_deck()];
        let mut game = Game::new(&decks, &db, Rules::standard(), Some(42)).unwrap();
        game.start_game();
        let dto = game_state_dto(&game);
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"turn_count\":1"));
        let parsed: GameStateDto = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.turn_count, 1);
        assert_eq!(parsed.players.len(), 2);
    }

    #[test]
    fn test_deck_is_legal_size() {
        let deck = test_deck();
        let mains = deck.iter().filter(|c| !c.starts_with("EGG")).count();
        let eggs = deck.iter().filter(|c| c.starts_with("EGG")).count();
        assert_eq!(mains, 50);
        assert_eq!(eggs, 5);
    }

    // Rust-side mirror of the Tauri command logic: drives `decode_action`
    // directly on a `Game`. The Tauri handlers themselves need a runtime
    // State<>, which is awkward to fabricate in a unit test — but the
    // interesting logic (dispatch + mask rebuild + envelope) can be exercised
    // directly.
    #[test]
    fn rust_submit_action_dispatches_and_rebuilds_mask() {
        let db = test_card_db();
        let decks = vec![test_deck(), test_deck()];
        let mut game = Game::new(&decks, &db, Rules::standard(), Some(42)).unwrap();
        game.start_game();

        // Drive through any initial mulligan decisions to reach a playable phase.
        while let Some(p) = game.mulligan_current_player() {
            game.accept_mulligan(p, /* keep */ true).unwrap();
        }

        // Submit a PASS action via decode_action — the same path
        // rust_submit_action takes — and confirm the envelope round-trips.
        let pid = current_decision_player(&game);
        game.decode_action(digimon_engine::action::space::PASS, pid);

        let mask = action_mask_bytes(&game);
        let dto = game_state_dto(&game);
        let resp = ActionResponseDto {
            state: dto,
            action_mask: mask,
            is_game_over: game.game_over,
            logs: Vec::new(),
            events: Vec::<GameEventDto>::new(),
            action_context: serde_json::json!({}),
            action_traces: Vec::new(),
        };

        assert_eq!(
            resp.action_mask.len(),
            digimon_engine::action::space::ACTION_SPACE_SIZE
        );
        assert!(!resp.is_game_over);
        // Serde round-trip — frontend relies on this envelope shape.
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"action_mask\""));
        assert!(json.contains("\"events\""));
        assert!(json.contains("\"is_game_over\":false"));
    }

    // ─── agent step loop ───────────────────────────────────────────────
    //
    // The Tauri command wrappers take `tauri::State<'_, _>` which we can't
    // construct in a unit test, so these tests exercise the internal
    // `run_agent_steps` entry point directly with real state objects.

    use digimon_engine::inference::{InferenceError, OnnxPolicy};

    /// Mock policy that always returns a fixed action (or the first valid
    /// action if the preferred one is masked off). Lets us test the Trained
    /// branch without an engine-shape .onnx file.
    struct FixedActionPolicy {
        preferred: usize,
        reset_count: std::cell::Cell<usize>,
        predict_count: std::cell::Cell<usize>,
    }

    impl FixedActionPolicy {
        fn new(preferred: usize) -> Self {
            Self {
                preferred,
                reset_count: std::cell::Cell::new(0),
                predict_count: std::cell::Cell::new(0),
            }
        }
    }

    impl OnnxPolicy for FixedActionPolicy {
        fn predict(&mut self, _obs: &[f32], mask: &[f32]) -> Result<usize, InferenceError> {
            self.predict_count.set(self.predict_count.get() + 1);
            if mask.get(self.preferred).copied().unwrap_or(0.0) > 0.0 {
                Ok(self.preferred)
            } else {
                // Fall back to first valid — matches how a real policy behaves
                // after masking kills its preferred action's logit.
                Ok(mask.iter().position(|&v| v > 0.0).unwrap_or(0))
            }
        }
        fn reset(&mut self) {
            self.reset_count.set(self.reset_count.get() + 1);
        }
    }

    fn build_playable_game() -> (Game, CardRegistry) {
        let db = test_card_db();
        let decks = vec![test_deck(), test_deck()];
        let mut game = Game::new(&decks, &db, Rules::standard(), Some(42)).unwrap();
        game.start_game();
        while let Some(p) = game.mulligan_current_player() {
            game.accept_mulligan(p, /* keep */ true).unwrap();
        }
        let registry = CardRegistry::from_cards(&db);
        (game, registry)
    }

    #[test]
    fn tensor_summary_reports_engine_contract() {
        let (game, registry) = build_playable_game();
        let pid = current_decision_player(&game);
        let mask = digimon_engine::action::build_action_mask(&game, pid);
        let summary = tensor_summary_for(&game, pid, &registry, &mask);

        assert_eq!(summary.player_id, pid);
        assert_eq!(summary.profile_id, "standard_v1");
        assert_eq!(summary.profile_version, 1);
        assert_eq!(summary.tensor_size, digimon_engine::tensor::TENSOR_SIZE);
        assert_eq!(
            summary.mask_size,
            digimon_engine::action::space::ACTION_SPACE_SIZE
        );
        assert_eq!(summary.tensor_size, 1375);
        assert_eq!(summary.mask_size, 2168);
        assert_eq!(summary.card_id_slot_count, 520);
        assert_eq!(summary.scalar_slot_count, 855);
        assert!(summary.legal_action_count > 0);
        assert_eq!(summary.phase, format!("{:?}", game.current_phase));
    }

    #[test]
    fn action_trace_serializes_human_action_context() {
        let (mut game, registry) = build_playable_game();
        let pid = current_decision_player(&game);
        let mask = digimon_engine::action::build_action_mask(&game, pid);
        let trace = action_trace_for(
            &game,
            "human",
            pid,
            digimon_engine::action::space::PASS,
            Some(tensor_summary_for(&game, pid, &registry, &mask)),
        );

        assert_eq!(trace.actor, "human");
        assert_eq!(trace.player_id, pid);
        assert_eq!(trace.action_id, digimon_engine::action::space::PASS);
        assert_eq!(
            trace.decoded.kind,
            digimon_engine::action::explain::ActionKind::Pass
        );
        assert!(trace.tensor_summary.is_some());

        let json = serde_json::to_string(&trace).unwrap();
        assert!(json.contains("\"actor\":\"human\""));
        assert!(json.contains("\"tensor_size\":1375"));
        assert!(json.contains("\"mask_size\":2168"));

        game.decode_action(digimon_engine::action::space::PASS, pid);
    }

    #[test]
    fn action_response_includes_human_trace() {
        let (mut game, registry) = build_playable_game();
        let pid = current_decision_player(&game);
        let mask_before = digimon_engine::action::build_action_mask(&game, pid);
        let action = digimon_engine::action::space::PASS;
        let human_trace = action_trace_for(
            &game,
            "human",
            pid,
            action,
            Some(tensor_summary_for(&game, pid, &registry, &mask_before)),
        );
        game.decode_action(action, pid);

        let resp = ActionResponseDto {
            state: game_state_dto(&game),
            action_mask: action_mask_bytes(&game),
            is_game_over: game.game_over,
            logs: Vec::new(),
            events: Vec::<GameEventDto>::new(),
            action_context: serde_json::json!({}),
            action_traces: vec![human_trace],
        };

        assert_eq!(resp.action_traces.len(), 1);
        assert_eq!(resp.action_traces[0].actor, "human");
        assert_eq!(resp.action_traces[0].action_id, action);
    }

    #[test]
    fn action_response_allows_human_trace_without_registry() {
        let (mut game, _registry) = build_playable_game();
        let pid = current_decision_player(&game);
        let mask_before = digimon_engine::action::build_action_mask(&game, pid);
        let action = digimon_engine::action::space::PASS;
        let human_trace = action_trace_for(
            &game,
            "human",
            pid,
            action,
            optional_tensor_summary_for(&game, pid, None, &mask_before),
        );
        game.decode_action(action, pid);

        let resp = ActionResponseDto {
            state: game_state_dto(&game),
            action_mask: action_mask_bytes(&game),
            is_game_over: game.game_over,
            logs: Vec::new(),
            events: Vec::<GameEventDto>::new(),
            action_context: serde_json::json!({}),
            action_traces: vec![human_trace],
        };

        assert_eq!(resp.action_traces.len(), 1);
        assert_eq!(resp.action_traces[0].actor, "human");
        assert_eq!(resp.action_traces[0].action_id, action);
        assert!(resp.action_traces[0].tensor_summary.is_none());
    }

    #[test]
    fn run_agent_steps_stops_when_current_decider_is_human() {
        let (mut game, registry) = build_playable_game();
        let session = GameSession {
            registry: Some(registry),
            player_kinds: vec![PlayerKind::Human, PlayerKind::Human],
            player_model_ids: vec![None, None],
        };
        let inference = InferenceState::default();
        let before = (game.turn_count, game.current_phase);
        let traces = run_agent_steps(&mut game, &session, &inference).unwrap();
        let after = (game.turn_count, game.current_phase);
        assert!(traces.is_empty());
        assert_eq!(before, after, "human seat should not advance state");
    }

    #[test]
    fn run_agent_steps_greedy_advances_until_game_resolves_or_human_up() {
        let (mut game, registry) = build_playable_game();
        // Make both seats greedy — the loop should drive to game_over, since
        // neither side ever stops for a human decision. `MAX_AGENT_STEPS`
        // caps the loop if decode_action somehow no-ops.
        let session = GameSession {
            registry: Some(registry),
            player_kinds: vec![PlayerKind::Greedy, PlayerKind::Greedy],
            player_model_ids: vec![None, None],
        };
        let inference = InferenceState::default();
        let traces = run_agent_steps(&mut game, &session, &inference).unwrap();
        assert!(!traces.is_empty());
        assert!(traces.iter().all(|trace| trace.actor.starts_with("agent_")));
        assert!(
            game.game_over,
            "two greedy agents should play the game to completion"
        );
    }

    #[test]
    fn run_agent_steps_greedy_traces_without_registry() {
        let (mut game, _registry) = build_playable_game();
        let session = GameSession {
            registry: None,
            player_kinds: vec![PlayerKind::Greedy, PlayerKind::Greedy],
            player_model_ids: vec![None, None],
        };
        let inference = InferenceState::default();
        let traces = run_agent_steps(&mut game, &session, &inference).unwrap();

        assert!(!traces.is_empty());
        assert!(traces.iter().all(|trace| trace.actor == "agent_greedy"));
        assert!(traces.iter().all(|trace| trace.tensor_summary.is_none()));
        assert!(
            game.game_over,
            "two greedy agents should still play to completion without registry"
        );
    }

    #[test]
    fn run_agent_steps_trained_seat_consults_loaded_policy() {
        let (mut game, registry) = build_playable_game();
        let inference = InferenceState::default();

        // Player 0 starts as decider — pass until player 1 (the trained
        // seat) is up so the loop is forced into the Trained branch.
        while current_decision_player(&game) == 0 && !game.game_over {
            let pid = current_decision_player(&game);
            game.decode_action(digimon_engine::action::space::PASS, pid);
        }
        let trained_pid = current_decision_player(&game);
        assert!(
            !game.game_over,
            "test setup expected game to still be live after player-0 passes"
        );

        let policy = Box::new(FixedActionPolicy::new(0)); // always PASS
        inference.insert_for_test("bot-42", policy);

        let mut kinds = vec![PlayerKind::Human; 2];
        kinds[trained_pid as usize] = PlayerKind::Trained;
        let mut model_ids: Vec<Option<String>> = vec![None; 2];
        model_ids[trained_pid as usize] = Some("bot-42".into());
        let session = GameSession {
            registry: Some(registry),
            player_kinds: kinds,
            player_model_ids: model_ids,
        };

        let before_pid = current_decision_player(&game);
        let traces = run_agent_steps(&mut game, &session, &inference).unwrap();
        assert!(!traces.is_empty());
        assert!(traces.iter().all(|trace| trace.actor.starts_with("agent_")));
        // After the trained seat's turn the loop should have left us on a
        // human decider (or the game should be over).
        assert!(
            current_decision_player(&game) != before_pid || game.game_over,
            "trained policy never advanced state; decider still {before_pid}"
        );
    }

    #[test]
    fn run_agent_steps_errors_when_trained_model_is_unloaded() {
        let (mut game, registry) = build_playable_game();
        let pid = current_decision_player(&game);
        // Force player 0 (who's up) to be Trained with a model_id that
        // doesn't exist in the cache.
        let kinds = match pid {
            0 => vec![PlayerKind::Trained, PlayerKind::Human],
            _ => vec![PlayerKind::Human, PlayerKind::Trained],
        };
        let model_ids = match pid {
            0 => vec![Some("nope".into()), None],
            _ => vec![None, Some("nope".into())],
        };
        let session = GameSession {
            registry: Some(registry),
            player_kinds: kinds,
            player_model_ids: model_ids,
        };
        let inference = InferenceState::default();
        let err = run_agent_steps(&mut game, &session, &inference)
            .expect_err("missing model should error");
        assert!(
            err.contains("not loaded") || err.contains("nope"),
            "error message should call out the missing model, got {err:?}"
        );
    }

    #[test]
    fn normalize_player_kinds_defaults_to_all_human() {
        let kinds = normalize_player_kinds(None, 2).unwrap();
        assert_eq!(kinds, vec![PlayerKind::Human, PlayerKind::Human]);
    }

    #[test]
    fn normalize_player_kinds_rejects_length_mismatch() {
        let err = normalize_player_kinds(Some(vec![PlayerKind::Greedy]), 2).unwrap_err();
        assert!(err.contains("length"));
    }

    #[test]
    fn normalize_player_model_ids_rejects_trained_without_model() {
        let kinds = vec![PlayerKind::Trained, PlayerKind::Human];
        let err = normalize_player_model_ids(Some(vec![None, None]), 2, &kinds).unwrap_err();
        assert!(err.contains("player 0"));
        assert!(err.contains("model_id"));
    }

    #[test]
    fn validate_shapes_rejects_wrong_obs_and_mask() {
        let ok_obs = vec![0.0f32; digimon_engine::tensor::TENSOR_SIZE];
        let ok_mask = vec![0.0f32; digimon_engine::action::space::ACTION_SPACE_SIZE];
        assert!(validate_shapes(&ok_obs, &ok_mask, "m").is_ok());

        let short_obs = vec![0.0f32; 10];
        assert!(validate_shapes(&short_obs, &ok_mask, "m").is_err());

        let short_mask = vec![0.0f32; 10];
        assert!(validate_shapes(&ok_obs, &short_mask, "m").is_err());
    }
}
