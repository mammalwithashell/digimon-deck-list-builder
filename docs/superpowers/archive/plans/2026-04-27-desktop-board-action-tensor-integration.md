# Desktop Board Action/Tensor Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrate the In Between game board redesign into the desktop Rust client while surfacing Rust-decoded player/agent actions and Rust board-state tensor snapshots through the Tauri game API.

**Architecture:** The Rust engine remains authoritative for legal actions, action decoding, agent policy decisions, and board-state tensors. Tauri responses carry action traces and compact tensor summaries to the React desktop client; the frontend renders those traces on the redesigned board without reimplementing engine semantics. The web/FastAPI surface is not the target, but TypeScript response types stay backward-compatible so shared frontend code still builds.

**Tech Stack:** Rust `digimon-engine`, Tauri v2 commands in `code/src-tauri`, React 19 + TypeScript + Vite frontend, Vitest, Cargo tests.

---

## Scope Check

This plan targets the desktop Rust implementation only:

- Rust action decoder and action mask contract live in `code/digimon-engine/src/action/`.
- Rust tensor builder lives in `code/digimon-engine/src/tensor.rs`.
- Desktop gameplay bridge lives in `code/src-tauri/src/engine_commands.rs`.
- React desktop adapter lives in `code/frontend/src/api/rustGameApi.ts`.
- Board rendering lives in `code/frontend/src/components/board/` and `code/frontend/src/pages/GamePage.tsx`.

Do not add a Python/FastAPI implementation in this plan. The hosted API may continue returning no traces; frontend fields added here must be optional.

## File Structure

### Rust Engine

- Create: `code/digimon-engine/src/action/explain.rs`
  - Pure, non-mutating explanation layer for flat action IDs.
  - Uses current `Game`, `GamePhase`, hand/field/trash context, and action-space constants.
  - Returns serializable `ActionExplanation`.

- Modify: `code/digimon-engine/src/action/mod.rs`
  - Export `explain`.

- Modify: `code/digimon-engine/tests/mask_and_tensor/main.rs`
  - Add `mod action_explain;`.

- Create: `code/digimon-engine/tests/mask_and_tensor/action_explain.rs`
  - Unit/integration coverage for explained action ranges.

### Tauri Desktop Bridge

- Modify: `code/src-tauri/src/engine_commands.rs`
  - Add DTOs for action trace and tensor summary.
  - Include `action_traces` in `ActionResponseDto` and `StepResponseDto`.
  - Include `tensor_summary` in traces when an agent chooses an action.
  - Change `run_agent_steps` to collect trace DTOs.
  - Add a debug command `rust_get_board_tensor_summary`.

- Modify: `code/src-tauri/src/lib.rs`
  - Register the new command if commands are listed explicitly.

### Frontend Types And API

- Modify: `code/frontend/src/types/game.ts`
  - Add `DecodedAction`, `TensorSummary`, and `ActionTrace` interfaces.

- Modify: `code/frontend/src/api/gameApi.ts`
  - Add optional `action_traces?: ActionTrace[]` to `ActionResponse` and `StepResponse`.

- Modify: `code/frontend/src/api/rustGameApi.ts`
  - Mirror Rust trace DTOs.
  - Translate trace payloads into frontend `ActionTrace`.
  - Export `getBoardTensorSummary(gameId, playerId)`.

- Modify: `code/frontend/src/stores/gameStore.ts`
  - Store recent action traces and latest tensor summaries.

- Modify: `code/frontend/src/pages/GamePage.tsx`
  - Append traces from `sendAction` and `stepGame`.
  - Pass traces/tensor metadata into board components.

### Board UI

- Modify: `code/frontend/src/components/board/GameBoard.tsx`
  - Render latest action trace in the top board chrome.
  - Render agent trace differently from human trace.

- Modify: `code/frontend/src/components/board/MemoryGauge.tsx`
  - Make the action pill display the latest decoded action label instead of static `Resolve`.

- Create: `code/frontend/src/components/board/ActionTraceTicker.tsx`
  - Small focused trace display for the board.

- Create: `code/frontend/src/components/board/TensorDebugBadge.tsx`
  - Compact desktop/debug badge showing tensor size, mask size, legal-action count, phase, and observer player.

- Modify: `code/frontend/src/index.css`
  - Add styles for the trace ticker and tensor badge using the In Between visual tokens.

### Frontend Tests

- Create: `code/frontend/src/api/rustGameApi.test.ts`
  - Verifies trace translation and tensor summary translation.

- Create: `code/frontend/src/components/board/ActionTraceTicker.test.tsx`
  - Verifies human/agent trace rendering.

---

## Task 1: Add A Non-Mutating Rust Action Explanation Layer

**Files:**
- Create: `code/digimon-engine/src/action/explain.rs`
- Modify: `code/digimon-engine/src/action/mod.rs`
- Create: `code/digimon-engine/tests/mask_and_tensor/action_explain.rs`
- Modify: `code/digimon-engine/tests/mask_and_tensor/main.rs`

- [ ] **Step 1: Write the failing Rust tests**

Create `code/digimon-engine/tests/mask_and_tensor/action_explain.rs`:

```rust
use std::collections::HashMap;

use digimon_engine::action::explain::{explain_action, ActionKind, ActionZone};
use digimon_engine::action::space::{
    encode_attack, encode_digivolve, HATCH, PASS, SECURITY_TARGET,
};
use digimon_engine::card_data::CardData;
use digimon_engine::enums::{CardKind, GamePhase};
use digimon_engine::game::Game;
use digimon_engine::rules::Rules;

fn test_card_db() -> HashMap<String, CardData> {
    let json = r#"{
        "BT1-001": {
            "card_id": "BT1-001", "card_name_eng": "Koromon",
            "card_effect_class_name": "BT1_001", "play_cost": 0, "dp": -1,
            "level": 2, "card_kind": 3, "rarity": 0, "card_colors": [0],
            "type_eng": ["Lesser"], "form_eng": ["In-Training"], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        },
        "BT1-010": {
            "card_id": "BT1-010", "card_name_eng": "Agumon",
            "card_effect_class_name": "BT1_010", "play_cost": 3, "dp": 2000,
            "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": ["Reptile"], "form_eng": ["Rookie"], "attribute_eng": ["Vaccine"],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        },
        "BT1-025": {
            "card_id": "BT1-025", "card_name_eng": "Greymon",
            "card_effect_class_name": "BT1_025", "play_cost": 5, "dp": 5000,
            "level": 4, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": ["Dinosaur"], "form_eng": ["Champion"], "attribute_eng": ["Vaccine"],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "",
            "evo_costs": [{"card_color": 0, "level": 3, "memory_cost": 2}]
        }
    }"#;
    CardData::load_from_str(json).unwrap()
}

fn test_deck() -> Vec<String> {
    let mut deck = Vec::new();
    for _ in 0..5 {
        deck.push("BT1-001".to_string());
    }
    for _ in 0..25 {
        deck.push("BT1-010".to_string());
    }
    for _ in 0..25 {
        deck.push("BT1-025".to_string());
    }
    deck
}

fn playable_game() -> Game {
    let db = test_card_db();
    let decks = vec![test_deck(), test_deck()];
    let mut game = Game::new(&decks, &db, Rules::standard(), Some(42)).unwrap();
    game.start_game();
    while let Some(p) = game.mulligan_current_player() {
        game.accept_mulligan(p, true).unwrap();
    }
    game.enter_main_phase();
    game.set_memory(5);
    game
}

#[test]
fn explains_main_phase_play_from_hand_with_card_context() {
    let game = playable_game();
    let pid = game.turn_player();
    let hand_idx = game
        .player(pid)
        .hand
        .iter()
        .position(|c| c.card_kind(&game.card_data) == CardKind::Digimon)
        .unwrap();

    let explanation = explain_action(&game, pid, hand_idx as u16);

    assert_eq!(explanation.action_id, hand_idx as u16);
    assert_eq!(explanation.player_id, pid);
    assert_eq!(explanation.kind, ActionKind::Play);
    assert_eq!(explanation.source_zone, Some(ActionZone::Hand));
    assert_eq!(explanation.source_index, Some(hand_idx as u16));
    assert!(explanation.label.contains("Play"));
    assert!(explanation.card_id.is_some());
}

#[test]
fn explains_breeding_hatch_and_pass() {
    let db = test_card_db();
    let decks = vec![test_deck(), test_deck()];
    let mut game = Game::new(&decks, &db, Rules::standard(), Some(42)).unwrap();
    game.start_game();
    while let Some(p) = game.mulligan_current_player() {
        game.accept_mulligan(p, true).unwrap();
    }
    assert_eq!(game.current_phase, GamePhase::Breeding);
    let pid = game.turn_player();

    let hatch = explain_action(&game, pid, HATCH);
    assert_eq!(hatch.kind, ActionKind::Hatch);
    assert_eq!(hatch.label, "Hatch from egg deck");

    let pass = explain_action(&game, pid, PASS);
    assert_eq!(pass.kind, ActionKind::Pass);
    assert_eq!(pass.label, "Pass / decline");
}

#[test]
fn explains_attack_security_target() {
    let mut game = playable_game();
    let pid = game.turn_player();
    let hand_idx = game
        .player(pid)
        .hand
        .iter()
        .position(|c| c.card_kind(&game.card_data) == CardKind::Digimon)
        .unwrap();
    game.play_from_hand(pid, hand_idx).unwrap();

    let action = encode_attack(0, SECURITY_TARGET);
    let explanation = explain_action(&game, pid, action);

    assert_eq!(explanation.kind, ActionKind::Attack);
    assert_eq!(explanation.source_zone, Some(ActionZone::Battle));
    assert_eq!(explanation.source_index, Some(0));
    assert_eq!(explanation.target_zone, Some(ActionZone::Security));
    assert!(explanation.label.contains("attacks security"));
}

#[test]
fn explains_digivolve_onto_breeding() {
    let game = playable_game();
    let pid = game.turn_player();
    let action = encode_digivolve(0, 14);

    let explanation = explain_action(&game, pid, action);

    assert_eq!(explanation.kind, ActionKind::Digivolve);
    assert_eq!(explanation.source_zone, Some(ActionZone::Hand));
    assert_eq!(explanation.source_index, Some(0));
    assert_eq!(explanation.target_zone, Some(ActionZone::Breeding));
    assert_eq!(explanation.target_index, None);
}
```

Modify `code/digimon-engine/tests/mask_and_tensor/main.rs`:

```rust
mod action_explain;
mod action_main_effects_parity;
mod card_registry_parity;
mod mask_end_of_turn_parity;
mod mask_main_effects_parity;
mod mask_main_parity;
mod tensor_and_mask;
mod tensor_helpers;
mod tensor_hidden_info;
mod tensor_source_contributions;
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```powershell
cargo test -p digimon-engine --test mask_and_tensor action_explain -- --nocapture
```

Expected: FAIL with an unresolved import for `digimon_engine::action::explain`.

- [ ] **Step 3: Add the action explanation module**

Create `code/digimon-engine/src/action/explain.rs`:

```rust
use serde::{Deserialize, Serialize};

use crate::action::space::{
    decode_attack, decode_digivolve, decode_field_effect, decode_source_select,
    ACTION_SPACE_SIZE, ATTACK_END, ATTACK_START, BREEDING_TARGET, DIGIVOLVE_END,
    DIGIVOLVE_START, DNA_DIGIVOLVE_END, DNA_DIGIVOLVE_START, FIELD_EFFECT_END,
    FIELD_EFFECT_START, HAND_EFFECT_END, HAND_EFFECT_START, HATCH, MOVE_FROM_BREEDING,
    PASS, PLAY_HAND_END, PLAY_HAND_START, SECURITY_TARGET, SOURCE_SELECT_END,
    SOURCE_SELECT_START, TRASH_EFFECT_END, TRASH_EFFECT_START,
};
use crate::enums::{GamePhase, PlayerId};
use crate::game::Game;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Play,
    HandEffect,
    Hatch,
    Move,
    Pass,
    DnaDigivolve,
    Attack,
    Digivolve,
    FieldEffect,
    TrashEffect,
    SourceSelect,
    Selection,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionZone {
    Hand,
    Battle,
    Breeding,
    Security,
    Trash,
    Source,
    Revealed,
    EffectChoice,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionExplanation {
    pub action_id: u16,
    pub player_id: PlayerId,
    pub phase: String,
    pub kind: ActionKind,
    pub label: String,
    pub source_zone: Option<ActionZone>,
    pub source_index: Option<u16>,
    pub target_zone: Option<ActionZone>,
    pub target_index: Option<u16>,
    pub card_id: Option<String>,
    pub card_name: Option<String>,
}

pub fn explain_action(game: &Game, player_id: PlayerId, action_id: u16) -> ActionExplanation {
    if action_id as usize >= ACTION_SPACE_SIZE {
        return base(game, player_id, action_id, ActionKind::Unknown, format!("Unknown action {action_id}"));
    }

    match game.current_phase {
        GamePhase::Mulligan => explain_mulligan(game, player_id, action_id),
        GamePhase::Breeding => explain_breeding(game, player_id, action_id),
        GamePhase::Main => explain_main(game, player_id, action_id),
        GamePhase::EndOfTurnAction => explain_end_of_turn(game, player_id, action_id),
        GamePhase::SelectTarget
        | GamePhase::SelectMaterial
        | GamePhase::SelectTrash
        | GamePhase::SelectSource
        | GamePhase::SelectHand
        | GamePhase::SelectReveal
        | GamePhase::SelectSecurity
        | GamePhase::EffectChoice
        | GamePhase::BlockTiming
        | GamePhase::CounterTiming
        | GamePhase::AllianceTiming
        | GamePhase::SelectUnion
        | GamePhase::SelectPermutation
        | GamePhase::SelectBudgeted => explain_selection(game, player_id, action_id),
        GamePhase::Unsuspend | GamePhase::Draw | GamePhase::EndTurn | GamePhase::GameOver => {
            base(game, player_id, action_id, ActionKind::Unknown, format!("No action in {:?}", game.current_phase))
        }
    }
}

fn base(
    game: &Game,
    player_id: PlayerId,
    action_id: u16,
    kind: ActionKind,
    label: String,
) -> ActionExplanation {
    ActionExplanation {
        action_id,
        player_id,
        phase: format!("{:?}", game.current_phase),
        kind,
        label,
        source_zone: None,
        source_index: None,
        target_zone: None,
        target_index: None,
        card_id: None,
        card_name: None,
    }
}

fn with_hand_card(
    mut explanation: ActionExplanation,
    game: &Game,
    player_id: PlayerId,
    hand_idx: usize,
) -> ActionExplanation {
    if let Some(card) = game.player(player_id).hand.get(hand_idx) {
        explanation.card_id = Some(card.card_id(&game.card_data));
        explanation.card_name = Some(card.name(&game.card_data).to_string());
    }
    explanation
}

fn explain_mulligan(game: &Game, player_id: PlayerId, action_id: u16) -> ActionExplanation {
    match action_id {
        0 => base(game, player_id, action_id, ActionKind::Selection, "Keep opening hand".to_string()),
        1 => base(game, player_id, action_id, ActionKind::Selection, "Mulligan opening hand".to_string()),
        _ => base(game, player_id, action_id, ActionKind::Unknown, format!("Unknown mulligan action {action_id}")),
    }
}

fn explain_breeding(game: &Game, player_id: PlayerId, action_id: u16) -> ActionExplanation {
    match action_id {
        HATCH => base(game, player_id, action_id, ActionKind::Hatch, "Hatch from egg deck".to_string()),
        MOVE_FROM_BREEDING => base(game, player_id, action_id, ActionKind::Move, "Move from breeding area".to_string()),
        PASS => base(game, player_id, action_id, ActionKind::Pass, "Pass / decline".to_string()),
        _ => base(game, player_id, action_id, ActionKind::Unknown, format!("Unknown breeding action {action_id}")),
    }
}

fn explain_main(game: &Game, player_id: PlayerId, action_id: u16) -> ActionExplanation {
    if (PLAY_HAND_START..PLAY_HAND_END).contains(&action_id) {
        let hand_idx = action_id as usize;
        let mut e = base(game, player_id, action_id, ActionKind::Play, format!("Play hand card {hand_idx}"));
        e.source_zone = Some(ActionZone::Hand);
        e.source_index = Some(hand_idx as u16);
        return with_hand_card(e, game, player_id, hand_idx);
    }
    if (HAND_EFFECT_START..HAND_EFFECT_END).contains(&action_id) {
        let hand_idx = (action_id - HAND_EFFECT_START) as usize;
        let mut e = base(game, player_id, action_id, ActionKind::HandEffect, format!("Activate hand effect {hand_idx}"));
        e.source_zone = Some(ActionZone::Hand);
        e.source_index = Some(hand_idx as u16);
        return with_hand_card(e, game, player_id, hand_idx);
    }
    if action_id == PASS {
        return base(game, player_id, action_id, ActionKind::Pass, "Pass / decline".to_string());
    }
    if (DNA_DIGIVOLVE_START..DNA_DIGIVOLVE_END).contains(&action_id) {
        let hand_idx = action_id - DNA_DIGIVOLVE_START;
        let mut e = base(game, player_id, action_id, ActionKind::DnaDigivolve, format!("DNA digivolve with hand card {hand_idx}"));
        e.source_zone = Some(ActionZone::Hand);
        e.source_index = Some(hand_idx);
        return with_hand_card(e, game, player_id, hand_idx as usize);
    }
    if (ATTACK_START..ATTACK_END).contains(&action_id) {
        return explain_attack(game, player_id, action_id);
    }
    if (DIGIVOLVE_START..DIGIVOLVE_END).contains(&action_id) {
        return explain_digivolve(game, player_id, action_id);
    }
    if (FIELD_EFFECT_START..FIELD_EFFECT_END).contains(&action_id) {
        let (perm, effect) = decode_field_effect(action_id);
        let mut e = base(game, player_id, action_id, ActionKind::FieldEffect, format!("Activate field effect {effect} on slot {perm}"));
        e.source_zone = Some(ActionZone::Battle);
        e.source_index = Some(perm);
        return e;
    }
    if (TRASH_EFFECT_START..TRASH_EFFECT_END).contains(&action_id) {
        let trash_idx = action_id - TRASH_EFFECT_START;
        let mut e = base(game, player_id, action_id, ActionKind::TrashEffect, format!("Activate trash effect {trash_idx}"));
        e.source_zone = Some(ActionZone::Trash);
        e.source_index = Some(trash_idx);
        return e;
    }
    base(game, player_id, action_id, ActionKind::Unknown, format!("Unknown action {action_id}"))
}

fn explain_attack(game: &Game, player_id: PlayerId, action_id: u16) -> ActionExplanation {
    let (attacker, target) = decode_attack(action_id);
    let mut e = base(game, player_id, action_id, ActionKind::Attack, String::new());
    e.source_zone = Some(ActionZone::Battle);
    e.source_index = Some(attacker);
    if target == SECURITY_TARGET {
        e.target_zone = Some(ActionZone::Security);
        e.label = format!("Slot {attacker} attacks security");
    } else {
        e.target_zone = Some(ActionZone::Battle);
        e.target_index = Some(target);
        e.label = format!("Slot {attacker} attacks opponent slot {target}");
    }
    if let Some(perm) = game.player(player_id).battle_area.get(attacker as usize) {
        e.card_id = Some(perm.top_card().card_id(&game.card_data));
        e.card_name = Some(perm.top_card().name(&game.card_data).to_string());
    }
    e
}

fn explain_digivolve(game: &Game, player_id: PlayerId, action_id: u16) -> ActionExplanation {
    let (hand, field) = decode_digivolve(action_id);
    let mut e = base(game, player_id, action_id, ActionKind::Digivolve, String::new());
    e.source_zone = Some(ActionZone::Hand);
    e.source_index = Some(hand);
    if field == BREEDING_TARGET {
        e.target_zone = Some(ActionZone::Breeding);
        e.label = format!("Digivolve hand {hand} onto breeding area");
    } else {
        e.target_zone = Some(ActionZone::Battle);
        e.target_index = Some(field);
        e.label = format!("Digivolve hand {hand} onto slot {field}");
    }
    with_hand_card(e, game, player_id, hand as usize)
}

fn explain_end_of_turn(game: &Game, player_id: PlayerId, action_id: u16) -> ActionExplanation {
    if action_id == PASS {
        return base(game, player_id, action_id, ActionKind::Pass, "Pass / decline".to_string());
    }
    if (ATTACK_START..ATTACK_END).contains(&action_id) {
        return explain_attack(game, player_id, action_id);
    }
    if (FIELD_EFFECT_START..FIELD_EFFECT_END).contains(&action_id) {
        let (perm, effect) = decode_field_effect(action_id);
        let mut e = base(game, player_id, action_id, ActionKind::FieldEffect, format!("Resolve end-of-turn effect {effect} on slot {perm}"));
        e.source_zone = Some(ActionZone::Battle);
        e.source_index = Some(perm);
        return e;
    }
    base(game, player_id, action_id, ActionKind::Unknown, format!("Unknown end-of-turn action {action_id}"))
}

fn explain_selection(game: &Game, player_id: PlayerId, action_id: u16) -> ActionExplanation {
    if (SOURCE_SELECT_START..SOURCE_SELECT_END).contains(&action_id) {
        let (field, source) = decode_source_select(action_id);
        let mut e = base(game, player_id, action_id, ActionKind::SourceSelect, format!("Select source {source} on slot {field}"));
        e.source_zone = Some(ActionZone::Source);
        e.source_index = Some(source);
        e.target_zone = Some(ActionZone::Battle);
        e.target_index = Some(field);
        return e;
    }
    if action_id == PASS {
        return base(game, player_id, action_id, ActionKind::Pass, "Pass / decline".to_string());
    }
    let mut e = base(game, player_id, action_id, ActionKind::Selection, format!("Select option {action_id}"));
    if let Some(sel) = game.pending_selection.as_ref() {
        e.label = format!("{:?}: select {}", sel.kind, action_id);
    }
    e
}
```

Modify `code/digimon-engine/src/action/mod.rs`:

```rust
pub mod decode;
pub mod explain;
pub mod mask;
pub mod space;

pub use mask::build_action_mask;
```

- [ ] **Step 4: Run tests to verify they pass**

Run:

```powershell
cargo test -p digimon-engine --test mask_and_tensor action_explain -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add code/digimon-engine/src/action/explain.rs code/digimon-engine/src/action/mod.rs code/digimon-engine/tests/mask_and_tensor/action_explain.rs code/digimon-engine/tests/mask_and_tensor/main.rs
git commit -m "feat: explain rust action ids"
```

---

## Task 2: Add Rust Action Trace And Tensor Summary DTOs

**Files:**
- Modify: `code/src-tauri/src/engine_commands.rs`

- [ ] **Step 1: Write failing tests for trace/tensor DTO shape**

Add these tests inside the existing `#[cfg(test)] mod tests` in `code/src-tauri/src/engine_commands.rs`:

```rust
#[test]
fn tensor_summary_reports_engine_contract() {
    let (game, registry) = build_playable_game();
    let pid = current_decision_player(&game);
    let mask = digimon_engine::action::build_action_mask(&game, pid);
    let summary = tensor_summary_for(&game, pid, &registry, &mask);

    assert_eq!(summary.player_id, pid);
    assert_eq!(summary.tensor_size, digimon_engine::tensor::TENSOR_SIZE);
    assert_eq!(summary.mask_size, digimon_engine::action::space::ACTION_SPACE_SIZE);
    assert_eq!(summary.tensor_size, 1375);
    assert_eq!(summary.mask_size, 2168);
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
    assert_eq!(trace.decoded.kind, digimon_engine::action::explain::ActionKind::Pass);
    assert!(trace.tensor_summary.is_some());

    let json = serde_json::to_string(&trace).unwrap();
    assert!(json.contains("\"actor\":\"human\""));
    assert!(json.contains("\"tensor_size\":1375"));
    assert!(json.contains("\"mask_size\":2168"));

    game.decode_action(digimon_engine::action::space::PASS, pid);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```powershell
cargo test -p digimon-tcg tensor_summary_reports_engine_contract action_trace_serializes_human_action_context -- --nocapture
```

Expected: FAIL with missing `TensorSummaryDto`, `ActionTraceDto`, `tensor_summary_for`, or `action_trace_for`.

- [ ] **Step 3: Add DTOs and helpers**

In `code/src-tauri/src/engine_commands.rs`, add imports near the existing imports:

```rust
use digimon_engine::action::explain::{explain_action, ActionExplanation};
```

Add these DTOs after `GameEventDto`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorSummaryDto {
    pub player_id: PlayerId,
    pub tensor_size: usize,
    pub mask_size: usize,
    pub legal_action_count: usize,
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
```

Add these helpers near `validate_shapes`:

```rust
fn tensor_summary_for(
    game: &Game,
    player_id: PlayerId,
    registry: &CardRegistry,
    mask: &[f32],
) -> TensorSummaryDto {
    let tensor = build_tensor(game, player_id, registry);
    TensorSummaryDto {
        player_id,
        tensor_size: tensor.len(),
        mask_size: mask.len(),
        legal_action_count: mask.iter().filter(|&&v| v > 0.0).count(),
        turn_count: game.turn_count,
        phase: format!("{:?}", game.current_phase),
        memory: game.memory,
        tensor_head: tensor.iter().take(16).copied().collect(),
    }
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run:

```powershell
cargo test -p digimon-tcg tensor_summary_reports_engine_contract action_trace_serializes_human_action_context -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add code/src-tauri/src/engine_commands.rs
git commit -m "feat: add desktop action trace DTOs"
```

---

## Task 3: Return Human And Agent Action Traces From Tauri Commands

**Files:**
- Modify: `code/src-tauri/src/engine_commands.rs`

- [ ] **Step 1: Write failing tests for command response trace shape**

Add this test in `code/src-tauri/src/engine_commands.rs` tests:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```powershell
cargo test -p digimon-tcg action_response_includes_human_trace -- --nocapture
```

Expected: FAIL because `ActionResponseDto` does not have `action_traces`.

- [ ] **Step 3: Add trace arrays to response DTOs**

Change `ActionResponseDto`:

```rust
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
```

Change `StepResponseDto`:

```rust
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
```

Update all test construction sites for `ActionResponseDto` and `StepResponseDto` in `engine_commands.rs` by adding `action_traces: Vec::new()` where the trace content is not under test.

- [ ] **Step 4: Update `rust_submit_action` and `rust_step_game`**

In `rust_submit_action`, replace the body between `let game = ensure_game...` and response construction with this structure:

```rust
let pid = current_decision_player(game);
let action_u16 = u16::try_from(action)
    .map_err(|_| format!("action {action} is out of range for a u16 action ID"))?;
let registry = session_guard.registry.as_ref().ok_or_else(|| {
    "action trace: session has no card registry (game not created?)".to_string()
})?;
let mask_before = build_action_mask(game, pid);
let mut action_traces = vec![action_trace_for(
    game,
    "human",
    pid,
    action_u16,
    Some(tensor_summary_for(game, pid, registry, &mask_before)),
)];
game.decode_action(action_u16, pid);
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
```

Change `rust_step_game` to collect traces:

```rust
let action_traces = run_agent_steps(game, &session_guard, &inference)?;
let mask = action_mask_bytes(game);
let pid = current_decision_player(game);
let is_human_turn = matches!(
    decider_kind(&session_guard, pid),
    PlayerKind::Human
) || game.game_over;
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
```

Change `run_agent_steps` signature and internals:

```rust
pub fn run_agent_steps(
    game: &mut Game,
    session: &GameSession,
    inference: &InferenceState,
) -> Result<Vec<ActionTraceDto>, String> {
    const MAX_AGENT_STEPS: usize = 10_000;
    let mut traces = Vec::new();
    for _ in 0..MAX_AGENT_STEPS {
        if game.game_over {
            return Ok(traces);
        }
        let pid = current_decision_player(game);
        let kind = decider_kind(session, pid);
        let action = match kind {
            PlayerKind::Human => return Ok(traces),
            PlayerKind::Greedy => {
                let mask = build_action_mask(game, pid);
                digimon_engine::policies::greedy_action(game, &mask) as usize
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
                let mask = build_action_mask(game, pid);
                validate_shapes(&obs, &mask, model_id)?;
                inference.predict(model_id, &obs, &mask)?
            }
        };
        let action_u16 = u16::try_from(action)
            .map_err(|_| format!("agent returned out-of-range action {action}"))?;
        let registry = session.registry.as_ref().ok_or_else(|| {
            "agent trace: session has no card registry (game not created?)".to_string()
        })?;
        let mask_before = build_action_mask(game, pid);
        let actor = match kind {
            PlayerKind::Human => "human",
            PlayerKind::Greedy => "agent_greedy",
            PlayerKind::Trained => "agent_trained",
        };
        traces.push(action_trace_for(
            game,
            actor,
            pid,
            action_u16,
            Some(tensor_summary_for(game, pid, registry, &mask_before)),
        ));
        game.decode_action(action_u16, pid);
    }
    Err(format!(
        "agent step loop exceeded {MAX_AGENT_STEPS} iterations; possible mask bug"
    ))
}
```

Update tests that call `run_agent_steps`:

```rust
let traces = run_agent_steps(&mut game, &session, &inference).unwrap();
```

When the test expects no agent work, assert:

```rust
assert!(traces.is_empty());
```

When the test expects agent work, assert:

```rust
assert!(!traces.is_empty());
assert!(traces.iter().all(|t| t.actor.starts_with("agent_")));
```

- [ ] **Step 5: Run tests**

Run:

```powershell
cargo test -p digimon-tcg action_response_includes_human_trace -- --nocapture
cargo test -p digimon-tcg run_agent_steps -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add code/src-tauri/src/engine_commands.rs
git commit -m "feat: return desktop action traces"
```

---

## Task 4: Add A Rust Tensor Summary Command For Desktop Debug UI

**Files:**
- Modify: `code/src-tauri/src/engine_commands.rs`
- Modify: `code/src-tauri/src/lib.rs`

- [ ] **Step 1: Add the command implementation**

Add this command near `rust_get_mask`:

```rust
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
```

- [ ] **Step 2: Register the command**

Open `code/src-tauri/src/lib.rs`. In the `tauri::generate_handler![...]` list, add:

```rust
engine_commands::rust_get_board_tensor_summary,
```

Keep the new command near the other `engine_commands::rust_*` gameplay commands.

- [ ] **Step 3: Run Rust build/test**

Run:

```powershell
cargo test -p digimon-tcg tensor_summary_reports_engine_contract -- --nocapture
cargo check -p digimon-tcg
```

Expected: PASS for test and successful cargo check.

- [ ] **Step 4: Commit**

```powershell
git add code/src-tauri/src/engine_commands.rs code/src-tauri/src/lib.rs
git commit -m "feat: expose desktop tensor summary command"
```

---

## Task 5: Add Frontend Types And Rust API Translation

**Files:**
- Modify: `code/frontend/src/types/game.ts`
- Modify: `code/frontend/src/api/gameApi.ts`
- Modify: `code/frontend/src/api/rustGameApi.ts`
- Create: `code/frontend/src/api/rustGameApi.test.ts`

- [ ] **Step 1: Write frontend API tests**

Create `code/frontend/src/api/rustGameApi.test.ts`:

```ts
import { describe, expect, it } from 'vitest';
import type { ActionTrace, TensorSummary } from '@/types/game';

describe('rust trace types', () => {
  it('supports agent traces with tensor summaries', () => {
    const summary: TensorSummary = {
      playerId: 1,
      tensorSize: 1375,
      maskSize: 2168,
      legalActionCount: 4,
      turnCount: 3,
      phase: 'Main',
      memory: 2,
      tensorHead: [0.1, 3, 0.2],
    };
    const trace: ActionTrace = {
      actor: 'agent_trained',
      playerId: 1,
      actionId: 62,
      decoded: {
        actionId: 62,
        playerId: 1,
        phase: 'Main',
        kind: 'pass',
        label: 'Pass / decline',
        sourceZone: null,
        sourceIndex: null,
        targetZone: null,
        targetIndex: null,
        cardId: null,
        cardName: null,
      },
      tensorSummary: summary,
    };

    expect(trace.tensorSummary?.tensorSize).toBe(1375);
    expect(trace.tensorSummary?.maskSize).toBe(2168);
    expect(trace.decoded.label).toBe('Pass / decline');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```powershell
npm run test -- rustGameApi.test.ts
```

Expected: FAIL because `ActionTrace` and `TensorSummary` do not exist.

- [ ] **Step 3: Add frontend types**

Append to `code/frontend/src/types/game.ts`:

```ts
export type ActionActor = 'human' | 'agent_greedy' | 'agent_trained' | string;

export type DecodedActionKind =
  | 'play'
  | 'hand_effect'
  | 'hatch'
  | 'move'
  | 'pass'
  | 'dna_digivolve'
  | 'attack'
  | 'digivolve'
  | 'field_effect'
  | 'trash_effect'
  | 'source_select'
  | 'selection'
  | 'unknown';

export type DecodedActionZone =
  | 'hand'
  | 'battle'
  | 'breeding'
  | 'security'
  | 'trash'
  | 'source'
  | 'revealed'
  | 'effect_choice';

export interface DecodedAction {
  actionId: number;
  playerId: number;
  phase: string;
  kind: DecodedActionKind;
  label: string;
  sourceZone: DecodedActionZone | null;
  sourceIndex: number | null;
  targetZone: DecodedActionZone | null;
  targetIndex: number | null;
  cardId: string | null;
  cardName: string | null;
}

export interface TensorSummary {
  playerId: number;
  tensorSize: number;
  maskSize: number;
  legalActionCount: number;
  turnCount: number;
  phase: string;
  memory: number;
  tensorHead: number[];
}

export interface ActionTrace {
  actor: ActionActor;
  playerId: number;
  actionId: number;
  decoded: DecodedAction;
  tensorSummary: TensorSummary | null;
}
```

- [ ] **Step 4: Update API response types**

In `code/frontend/src/api/gameApi.ts`, import `ActionTrace`:

```ts
import type { GameState, GameEvent, ActionTrace } from '@/types/game';
```

Add optional traces to `ActionResponse`:

```ts
interface ActionResponse {
  state: GameState;
  action_mask: number[];
  is_game_over: boolean;
  logs?: string[];
  events?: GameEvent[];
  action_context?: Record<string, unknown>;
  action_traces?: ActionTrace[];
}
```

Add optional traces to `StepResponse`:

```ts
interface StepResponse {
  state: GameState;
  action_mask: number[];
  logs: string[];
  events?: GameEvent[];
  is_human_turn: boolean;
  is_game_over: boolean;
  action_traces?: ActionTrace[];
}
```

- [ ] **Step 5: Translate snake_case Rust DTOs in `rustGameApi.ts`**

In `code/frontend/src/api/rustGameApi.ts`, import the new types:

```ts
import type {
  ActionTrace,
  DecodedAction,
  GameEvent,
  GameState,
  PermanentInfo,
  PlayerState,
  TensorSummary,
} from '@/types/game';
```

Add Rust DTO interfaces:

```ts
interface RustDecodedAction {
  action_id: number;
  player_id: number;
  phase: string;
  kind: string;
  label: string;
  source_zone: string | null;
  source_index: number | null;
  target_zone: string | null;
  target_index: number | null;
  card_id: string | null;
  card_name: string | null;
}

interface RustTensorSummary {
  player_id: number;
  tensor_size: number;
  mask_size: number;
  legal_action_count: number;
  turn_count: number;
  phase: string;
  memory: number;
  tensor_head: number[];
}

interface RustActionTrace {
  actor: string;
  player_id: number;
  action_id: number;
  decoded: RustDecodedAction;
  tensor_summary: RustTensorSummary | null;
}
```

Add to Rust response interfaces:

```ts
interface RustActionResponse {
  state: GameStateDto;
  action_mask: number[];
  is_game_over: boolean;
  logs: string[];
  events: GameEvent[];
  action_context: Record<string, unknown>;
  action_traces?: RustActionTrace[];
}

interface RustStepResponse {
  state: GameStateDto;
  action_mask: number[];
  logs: string[];
  events: GameEvent[];
  is_human_turn: boolean;
  is_game_over: boolean;
  action_traces?: RustActionTrace[];
}
```

Add mapping helpers:

```ts
function toTensorSummary(dto: RustTensorSummary | null | undefined): TensorSummary | null {
  if (!dto) return null;
  return {
    playerId: dto.player_id,
    tensorSize: dto.tensor_size,
    maskSize: dto.mask_size,
    legalActionCount: dto.legal_action_count,
    turnCount: dto.turn_count,
    phase: dto.phase,
    memory: dto.memory,
    tensorHead: dto.tensor_head,
  };
}

function toDecodedAction(dto: RustDecodedAction): DecodedAction {
  return {
    actionId: dto.action_id,
    playerId: dto.player_id,
    phase: dto.phase,
    kind: dto.kind as DecodedAction['kind'],
    label: dto.label,
    sourceZone: dto.source_zone as DecodedAction['sourceZone'],
    sourceIndex: dto.source_index,
    targetZone: dto.target_zone as DecodedAction['targetZone'],
    targetIndex: dto.target_index,
    cardId: dto.card_id,
    cardName: dto.card_name,
  };
}

function toActionTrace(dto: RustActionTrace): ActionTrace {
  return {
    actor: dto.actor,
    playerId: dto.player_id,
    actionId: dto.action_id,
    decoded: toDecodedAction(dto.decoded),
    tensorSummary: toTensorSummary(dto.tensor_summary),
  };
}

function toActionTraces(dtos: RustActionTrace[] | undefined): ActionTrace[] {
  return (dtos ?? []).map(toActionTrace);
}
```

Update `sendAction` return:

```ts
return {
  state: dtoToGameState(resp.state),
  action_mask: resp.action_mask,
  is_game_over: resp.is_game_over,
  logs: resp.logs,
  events: resp.events,
  action_context: resp.action_context,
  action_traces: toActionTraces(resp.action_traces),
};
```

Update `stepGame` return:

```ts
return {
  state: dtoToGameState(resp.state),
  action_mask: resp.action_mask,
  logs: resp.logs,
  events: resp.events,
  is_human_turn: resp.is_human_turn,
  is_game_over: resp.is_game_over,
  action_traces: toActionTraces(resp.action_traces),
};
```

Add export:

```ts
export async function getBoardTensorSummary(
  _gameId: string,
  playerId: number,
): Promise<TensorSummary> {
  const resp = await invoke<RustTensorSummary>('rust_get_board_tensor_summary', {
    playerId,
  });
  const summary = toTensorSummary(resp);
  if (!summary) {
    throw new Error('Rust returned no tensor summary');
  }
  return summary;
}
```

- [ ] **Step 6: Run frontend tests and build**

Run:

```powershell
npm run test -- rustGameApi.test.ts
npm run build
```

Expected: PASS. Build may still show the pre-existing Tauri dynamic import chunking warning.

- [ ] **Step 7: Commit**

```powershell
git add code/frontend/src/types/game.ts code/frontend/src/api/gameApi.ts code/frontend/src/api/rustGameApi.ts code/frontend/src/api/rustGameApi.test.ts
git commit -m "feat: type desktop action traces"
```

---

## Task 6: Store Action Traces And Tensor Summaries In Frontend State

**Files:**
- Modify: `code/frontend/src/stores/gameStore.ts`
- Modify: `code/frontend/src/pages/GamePage.tsx`

- [ ] **Step 1: Update store types and actions**

In `code/frontend/src/stores/gameStore.ts`, update imports:

```ts
import type {
  ActionTrace,
  GameState,
  GameEvent,
  GamePhase,
  PlayerState,
  PendingSelection,
  PendingAttack,
  TensorSummary,
} from '@/types/game';
```

Add to `GameStore`:

```ts
actionTraces: ActionTrace[];
latestTensorSummary: TensorSummary | null;
appendActionTraces: (traces: ActionTrace[]) => void;
setLatestTensorSummary: (summary: TensorSummary | null) => void;
```

Add to `initialState`:

```ts
actionTraces: [],
latestTensorSummary: null,
```

Add actions:

```ts
appendActionTraces: (traces) =>
  set((s) => ({
    actionTraces: [...s.actionTraces, ...traces].slice(-20),
    latestTensorSummary:
      traces
        .slice()
        .reverse()
        .find((t) => t.tensorSummary != null)?.tensorSummary
      ?? s.latestTensorSummary,
  })),
setLatestTensorSummary: (summary) => set({ latestTensorSummary: summary }),
```

Do not clear traces on every action. In `store.clearLogs()` calls at game start, also calling `store.appendActionTraces([])` is unnecessary. For this plan, rely on `store.reset()` when returning to menu and add explicit trace clearing inside the game-start success path:

```ts
store.clearActionTraces();
```

To support that, add to `GameStore`:

```ts
clearActionTraces: () => void;
```

Add to the store implementation:

```ts
clearActionTraces: () => set({ actionTraces: [], latestTensorSummary: null }),
```

- [ ] **Step 2: Wire traces from action responses**

In `code/frontend/src/pages/GamePage.tsx`, after each response from `sendAction`, `stepGame`, and `surrenderGame`, append traces if present.

In the `handleAction` callback, replace:

```ts
await sendAction(actionId);
```

with:

```ts
const result = await sendAction(actionId);
if (result.action_traces?.length) {
  store.appendActionTraces(result.action_traces);
}
```

In `handleStartGame`, after:

```ts
const stepResult = await gameApi.stepGame(result.game_id);
```

add:

```ts
if (stepResult.action_traces?.length) {
  store.appendActionTraces(stepResult.action_traces);
}
```

In WebSocket paths, do not fabricate traces. WebSocket trace support is outside this desktop-focused plan.

- [ ] **Step 3: Run build**

Run:

```powershell
npm run build
```

Expected: PASS.

- [ ] **Step 4: Commit**

```powershell
git add code/frontend/src/stores/gameStore.ts code/frontend/src/pages/GamePage.tsx
git commit -m "feat: store desktop action traces"
```

---

## Task 7: Render Action Trace And Tensor Badges On The Redesigned Board

**Files:**
- Create: `code/frontend/src/components/board/ActionTraceTicker.tsx`
- Create: `code/frontend/src/components/board/TensorDebugBadge.tsx`
- Modify: `code/frontend/src/components/board/GameBoard.tsx`
- Modify: `code/frontend/src/components/board/MemoryGauge.tsx`
- Modify: `code/frontend/src/index.css`
- Create: `code/frontend/src/components/board/ActionTraceTicker.test.tsx`

- [ ] **Step 1: Add component tests**

Create `code/frontend/src/components/board/ActionTraceTicker.test.tsx`:

```tsx
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { ActionTraceTicker } from './ActionTraceTicker';
import type { ActionTrace } from '@/types/game';

function trace(actor: string, label: string): ActionTrace {
  return {
    actor,
    playerId: actor === 'human' ? 0 : 1,
    actionId: 62,
    decoded: {
      actionId: 62,
      playerId: actor === 'human' ? 0 : 1,
      phase: 'Main',
      kind: 'pass',
      label,
      sourceZone: null,
      sourceIndex: null,
      targetZone: null,
      targetIndex: null,
      cardId: null,
      cardName: null,
    },
    tensorSummary: null,
  };
}

describe('ActionTraceTicker', () => {
  it('renders the latest decoded action', () => {
    render(<ActionTraceTicker traces={[trace('human', 'Pass / decline')]} />);
    expect(screen.getByText('HUMAN')).toBeTruthy();
    expect(screen.getByText('Pass / decline')).toBeTruthy();
  });

  it('labels trained agent actions distinctly', () => {
    render(<ActionTraceTicker traces={[trace('agent_trained', 'Slot 0 attacks security')]} />);
    expect(screen.getByText('AGENT TRAINED')).toBeTruthy();
    expect(screen.getByText('Slot 0 attacks security')).toBeTruthy();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```powershell
npm run test -- ActionTraceTicker.test.tsx
```

Expected: FAIL because `ActionTraceTicker` does not exist. If `@testing-library/react` is not installed, install it with:

```powershell
npm install -D @testing-library/react @testing-library/jest-dom
```

Then rerun the test.

- [ ] **Step 3: Create `ActionTraceTicker`**

Create `code/frontend/src/components/board/ActionTraceTicker.tsx`:

```tsx
import type { ActionTrace } from '@/types/game';

interface ActionTraceTickerProps {
  traces: ActionTrace[];
}

function actorLabel(actor: string): string {
  if (actor === 'human') return 'HUMAN';
  if (actor === 'agent_greedy') return 'AGENT GREEDY';
  if (actor === 'agent_trained') return 'AGENT TRAINED';
  return actor.replaceAll('_', ' ').toUpperCase();
}

export function ActionTraceTicker({ traces }: ActionTraceTickerProps) {
  const latest = traces.at(-1);

  if (!latest) {
    return (
      <div className="ib-action-trace" aria-label="No action trace yet">
        <span className="ib-action-trace__actor">TRACE</span>
        <span className="ib-action-trace__label">Awaiting action</span>
      </div>
    );
  }

  return (
    <div className={`ib-action-trace ib-action-trace--${latest.actor.startsWith('agent') ? 'agent' : 'human'}`}>
      <span className="ib-action-trace__actor">{actorLabel(latest.actor)}</span>
      <span className="ib-action-trace__label">{latest.decoded.label}</span>
    </div>
  );
}
```

- [ ] **Step 4: Create `TensorDebugBadge`**

Create `code/frontend/src/components/board/TensorDebugBadge.tsx`:

```tsx
import type { TensorSummary } from '@/types/game';

interface TensorDebugBadgeProps {
  summary: TensorSummary | null;
}

export function TensorDebugBadge({ summary }: TensorDebugBadgeProps) {
  if (!summary) return null;

  return (
    <div className="ib-tensor-badge" aria-label="Board tensor summary">
      <span>P{summary.playerId}</span>
      <span>T{summary.tensorSize}</span>
      <span>A{summary.maskSize}</span>
      <span>L{summary.legalActionCount}</span>
      <span>{summary.phase}</span>
    </div>
  );
}
```

- [ ] **Step 5: Pass traces into `GameBoard`**

In `code/frontend/src/components/board/GameBoard.tsx`, import:

```tsx
import { ActionTraceTicker } from './ActionTraceTicker';
import { TensorDebugBadge } from './TensorDebugBadge';
import type { ActionTrace, TensorSummary } from '@/types/game';
```

Extend `GameBoardProps`:

```ts
actionTraces?: ActionTrace[];
latestTensorSummary?: TensorSummary | null;
```

Add to function props:

```ts
actionTraces = [],
latestTensorSummary = null,
```

Inside `.ib-board__top-chrome`, after the existing tags, add:

```tsx
<ActionTraceTicker traces={actionTraces} />
```

Near the hand-count chip, add:

```tsx
<TensorDebugBadge summary={latestTensorSummary} />
```

In `code/frontend/src/pages/GamePage.tsx`, pass:

```tsx
actionTraces={store.actionTraces}
latestTensorSummary={store.latestTensorSummary}
```

to `GameBoard`.

- [ ] **Step 6: Make the memory gauge action pill dynamic**

In `code/frontend/src/components/board/MemoryGauge.tsx`, add prop:

```ts
latestActionLabel?: string | null;
```

Update signature:

```ts
export function MemoryGauge({
  value,
  localPlayer,
  currentPhase,
  previewCost,
  latestActionLabel,
}: MemoryGaugeProps) {
```

Replace static action text:

```tsx
{latestActionLabel ?? 'Resolve'}
```

In `GameBoard.tsx`, derive:

```ts
const latestActionLabel = actionTraces.at(-1)?.decoded.label ?? null;
```

Pass to gauge:

```tsx
<MemoryGauge
  value={memoryGauge}
  localPlayer={1}
  currentPhase={currentPhase}
  previewCost={previewCost}
  latestActionLabel={latestActionLabel}
/>
```

- [ ] **Step 7: Add CSS**

Append to `code/frontend/src/index.css`:

```css
.ib-action-trace {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 24px;
  max-width: 360px;
  padding: 0 10px;
  border: 1px solid var(--ib-line);
  background: oklch(0.10 0.012 250 / 0.82);
  color: var(--ib-bone-d);
  font-family: var(--ib-font-mono);
  font-size: 10px;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}

.ib-action-trace--agent {
  border-color: var(--ib-opp);
  box-shadow: 0 0 14px oklch(0.72 0.15 230 / 0.24);
}

.ib-action-trace--human {
  border-color: var(--ib-player);
  box-shadow: 0 0 14px oklch(0.74 0.17 50 / 0.22);
}

.ib-action-trace__actor {
  color: var(--ib-bone-dd);
  white-space: nowrap;
}

.ib-action-trace__label {
  min-width: 0;
  overflow: hidden;
  color: var(--ib-bone);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ib-tensor-badge {
  position: absolute;
  left: 18px;
  bottom: 8px;
  z-index: 13;
  display: inline-flex;
  align-items: center;
  gap: 7px;
  padding: 6px 10px;
  border: 1px solid var(--ib-line);
  background: oklch(0.10 0.012 250 / 0.9);
  color: var(--ib-bone-dd);
  font-family: var(--ib-font-mono);
  font-size: 9px;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}

.ib-tensor-badge span:nth-child(2),
.ib-tensor-badge span:nth-child(3) {
  color: var(--ib-horizon);
}
```

- [ ] **Step 8: Run tests and build**

Run:

```powershell
npm run test -- ActionTraceTicker.test.tsx
npm run build
```

Expected: PASS.

- [ ] **Step 9: Commit**

```powershell
git add code/frontend/src/components/board/ActionTraceTicker.tsx code/frontend/src/components/board/TensorDebugBadge.tsx code/frontend/src/components/board/ActionTraceTicker.test.tsx code/frontend/src/components/board/GameBoard.tsx code/frontend/src/components/board/MemoryGauge.tsx code/frontend/src/pages/GamePage.tsx code/frontend/src/index.css
git commit -m "feat: render desktop action traces on board"
```

---

## Task 8: Desktop Board Redesign Hardening

**Files:**
- Modify: `code/frontend/src/components/board/GameBoard.tsx`
- Modify: `code/frontend/src/components/board/PlayerHalf.tsx`
- Modify: `code/frontend/src/components/board/BattleArea.tsx`
- Modify: `code/frontend/src/components/board/HandZone.tsx`
- Modify: `code/frontend/src/components/board/MemoryGauge.tsx`
- Modify: `code/frontend/src/index.css`

- [ ] **Step 1: Confirm board redesign remains interaction-safe**

Run:

```powershell
npm run build
```

Expected: PASS.

Manually inspect these behavior points in desktop dev mode:

```powershell
npm run dev:desktop -- --host 127.0.0.1 --port 5173
```

Expected:

- Hand cards still click and drag.
- Empty battle slots still accept play drops when legal.
- Occupied battle slots still accept digivolve drops when legal.
- Hatch, move, trash viewer, and security attack actions still work.
- The action trace ticker updates after a human action.
- The ticker updates after a greedy or trained agent action.
- The tensor badge shows `T1375` and `A2168`.

- [ ] **Step 2: Add a visual regression note to docs**

Create `docs/DESKTOP_BOARD_INTEGRATION.md`:

```markdown
# Desktop Board Integration

The desktop client uses the Rust engine through Tauri commands in
`code/src-tauri/src/engine_commands.rs`.

The live board renders the In Between design language:

- graphite/obsidian mat
- horizon memory seam
- sharp resource/security chrome
- action trace ticker
- tensor debug badge

Action semantics are not decoded in React. Rust produces `action_traces`
from `digimon_engine::action::explain::explain_action`, and the board renders
those traces. Agent tensor snapshots are summarized in `TensorSummaryDto`;
the raw 1375-float tensor is not sent to the UI by default.

Canonical contracts:

- Action mask size: `2168`
- Board-state tensor size: `1375`
- Rust action decoder: `code/digimon-engine/src/action/decode.rs`
- Rust action explainer: `code/digimon-engine/src/action/explain.rs`
- Rust tensor builder: `code/digimon-engine/src/tensor.rs`
```

- [ ] **Step 3: Commit**

```powershell
git add docs/DESKTOP_BOARD_INTEGRATION.md
git commit -m "docs: document desktop board action tensor integration"
```

---

## Task 9: Full Verification

**Files:**
- No code edits expected.

- [ ] **Step 1: Run Rust engine contract tests**

Run:

```powershell
cargo test -p digimon-engine --test mask_and_tensor -- --nocapture
```

Expected: PASS.

- [ ] **Step 2: Run Tauri bridge tests**

Run:

```powershell
cargo test -p digimon-tcg -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Run frontend unit tests**

Run:

```powershell
npm run test -- rustGameApi.test.ts ActionTraceTicker.test.tsx
```

Expected: PASS.

- [ ] **Step 4: Run frontend build**

Run:

```powershell
npm run build
```

Expected: PASS. The existing Tauri dynamic import chunk warning may appear; it is unrelated to this integration.

- [ ] **Step 5: Run desktop smoke test**

Run:

```powershell
npm run dev:desktop -- --host 127.0.0.1 --port 5173
```

Open `http://127.0.0.1:5173`.

Expected:

- Start a local game against Greedy Agent.
- Keep or mulligan.
- Perform one legal action.
- The board trace displays the human action.
- The agent acts automatically.
- The board trace displays the agent action.
- Tensor badge reports `T1375`, `A2168`, and a legal-action count above zero.

- [ ] **Step 6: Final commit**

If verification required any small fixes:

```powershell
git add code docs
git commit -m "fix: stabilize desktop board action tensor integration"
```

If no fixes were needed, do not create an empty commit.

---

## Self-Review

### Spec Coverage

- Desktop client board redesign integration: Tasks 7 and 8.
- Rust implementation target: Tasks 1 through 4 all modify Rust engine/Tauri.
- Action decoder wiring: Tasks 1 through 3 add non-mutating explanations and include traces in action responses.
- Agent action wiring: Task 3 collects traces from `run_agent_steps` for greedy and trained agents.
- Board-state tensor wiring: Tasks 2 through 4 summarize Rust tensors from `build_tensor`; Task 7 renders the summary.
- Frontend display: Tasks 5 through 7 type, store, and render traces and tensor summaries.
- Verification: Task 9.

### Placeholder Scan

No unresolved fill-in markers remain. Every code-changing task includes concrete code or exact edits.

### Type Consistency

Rust DTO names are `ActionTraceDto`, `TensorSummaryDto`, and `ActionExplanation`.
Frontend names are `ActionTrace`, `TensorSummary`, and `DecodedAction`.
Rust snake_case fields are translated to frontend camelCase in `rustGameApi.ts`.
