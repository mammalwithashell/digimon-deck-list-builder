//! Tauri commands that expose the Rust `digimon-engine` directly to the frontend.
//!
//! These run in parallel with the existing Python sidecar (which is still
//! responsible for RL inference). Eventually the frontend will prefer these
//! commands and the sidecar will be retired.
//!
//! All commands mutate shared state behind a `Mutex<Option<Game>>`. The
//! frontend calls `create_test_game` first, then uses `get_state` /
//! `play_card` / `attack_digimon` / `attack_player` / `end_turn` to drive it.

use std::collections::HashMap;
use std::sync::Mutex;

use digimon_engine::action::build_action_mask;
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlayerId};
use digimon_engine::game::Game;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::rules::Rules;
use serde::{Deserialize, Serialize};

/// Shared mutable Rust-engine state, held by Tauri.
#[derive(Default)]
pub struct RustEngineState {
    pub game: Mutex<Option<Game>>,
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

fn perm_dto(
    game: &Game,
    player: PlayerId,
    index: usize,
) -> PermanentDto {
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
    let hand: Vec<CardDto> = p.hand.iter().map(|c| card_dto(c, &game.card_data)).collect();
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
    }
}

/// Build a tiny test card database that works with the engine's built-in
/// TEST-001..005 effects plus some vanilla Digimon for combat.
fn test_card_db() -> HashMap<String, CardData> {
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

fn test_deck() -> Vec<String> {
    // 50 main deck + 5 eggs.
    let mut deck = Vec::with_capacity(55);
    let mains = [
        "TEST-001", "TEST-002", "TEST-003", "TEST-004", "TEST-005",
        "VANILLA-3K", "VANILLA-5K",
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
pub fn create_test_game(
    state: tauri::State<'_, RustEngineState>,
) -> Result<GameStateDto, String> {
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
pub fn rust_end_turn(
    state: tauri::State<'_, RustEngineState>,
) -> Result<GameStateDto, String> {
    let mut guard = state.game.lock().map_err(|e| e.to_string())?;
    let game = guard.as_mut().ok_or("No active game")?;
    game.end_turn();
    Ok(game_state_dto(game))
}

/// Pass turn (memory to -3, then end turn).
#[tauri::command]
pub fn rust_pass_turn(
    state: tauri::State<'_, RustEngineState>,
) -> Result<GameStateDto, String> {
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

// ─── Action-ID dispatch envelope (parity with Python sidecar shape) ───
//
// The frontend's gameApi.ts speaks in flat action IDs and expects responses
// shaped like Python's ActionResponse. These commands and DTOs let the
// frontend bind against the same interface regardless of which engine is
// backing it.

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
pub struct ActionResponseDto {
    pub state: GameStateDto,
    pub action_mask: Vec<u8>,
    pub is_game_over: bool,
    pub logs: Vec<String>,
    pub events: Vec<GameEventDto>,
    pub action_context: serde_json::Value,
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
/// optional deck lists; when absent, uses the standard test decks.
#[tauri::command]
pub fn rust_create_game(
    state: tauri::State<'_, RustEngineState>,
    deck1: Option<Vec<String>>,
    deck2: Option<Vec<String>>,
) -> Result<CreateGameResponseDto, String> {
    let db = test_card_db();
    let decks = vec![
        deck1.unwrap_or_else(test_deck),
        deck2.unwrap_or_else(test_deck),
    ];
    let mut game = Game::new(&decks, &db, Rules::standard(), Some(42))
        .map_err(|e| format!("Game::new failed: {}", e))?;
    game.start_game();
    let mask = action_mask_bytes(&game);
    let dto = game_state_dto(&game);
    *state.game.lock().map_err(|e| e.to_string())? = Some(game);
    Ok(CreateGameResponseDto {
        game_id: "rust-local".to_string(),
        state: dto,
        action_mask: mask,
    })
}

/// Execute a flat RL-action-ID against the current game. Mirrors the Python
/// sidecar's `POST /games/{id}/actions` surface so the frontend's gameApi
/// can bind against this without knowing which backend is running.
#[tauri::command]
pub fn rust_submit_action(
    state: tauri::State<'_, RustEngineState>,
    action: u32,
) -> Result<ActionResponseDto, String> {
    let mut guard = state.game.lock().map_err(|e| e.to_string())?;
    let game = ensure_game(&mut guard)?;
    let pid = current_decision_player(game);
    game.decode_action(action as u16, pid);
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
    })
}

/// Advance agent turns. The Rust mode has no AI-opponent policy yet, so
/// this is a pure state snapshot with the current mask — matches the
/// `stepGame` shape so the frontend's step-after-action loop is a no-op
/// rather than an error.
#[tauri::command]
pub fn rust_step_game(
    state: tauri::State<'_, RustEngineState>,
) -> Result<StepResponseDto, String> {
    let mut guard = state.game.lock().map_err(|e| e.to_string())?;
    let game = ensure_game(&mut guard)?;
    let mask = action_mask_bytes(game);
    let is_over = game.game_over;
    Ok(StepResponseDto {
        state: game_state_dto(game),
        action_mask: mask,
        logs: Vec::new(),
        events: Vec::new(),
        is_human_turn: true,
        is_game_over: is_over,
    })
}

/// Read the current action mask.
#[tauri::command]
pub fn rust_get_mask(
    state: tauri::State<'_, RustEngineState>,
) -> Result<Vec<u8>, String> {
    let guard = state.game.lock().map_err(|e| e.to_string())?;
    let game = guard.as_ref().ok_or("No active game")?;
    Ok(action_mask_bytes(game))
}

/// Read the accumulated log (empty for now — Rust engine doesn't log yet).
#[tauri::command]
pub fn rust_get_log(
    state: tauri::State<'_, RustEngineState>,
) -> Result<Vec<String>, String> {
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

/// Delete the active game (used by the frontend cleanup path).
#[tauri::command]
pub fn rust_delete_game(
    state: tauri::State<'_, RustEngineState>,
) -> Result<(), String> {
    *state.game.lock().map_err(|e| e.to_string())? = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dto_roundtrip_through_json() {
        let db = test_card_db();
        let decks = vec![test_deck(), test_deck()];
        let mut game =
            Game::new(&decks, &db, Rules::standard(), Some(42)).unwrap();
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
}
