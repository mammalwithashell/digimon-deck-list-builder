//! Shared helpers for the rules_conformance chapter modules: driving
//! prompts (`TriggerOrder`, target / hand / field picks, optional prompts,
//! Option use) and reading the board and event log.
//!
//! Infrastructure, not a chapter: it holds no `#[test]`, is excluded from
//! `meta::CHAPTER_SOURCES`, and is counted in `meta::INFRA_FILES`.

use digimon_engine::action::space::{
    ATTACK_START, EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START,
    PLAY_HAND_START,
};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::PlayerId;
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

/// Deck / security / hand filler (`fixtures/common/VANILLA_LV3.yaml`).
pub const FILL: &str = "VANILLA_LV3";

/// Card id of a `TriggerOrder` branch: the drainer labels each branch
/// `"<card_id> slot <n> (<optional|mandatory>)"`.
pub fn branch_card(label: &str) -> &str {
    label.split(" slot ").next().unwrap_or(label)
}

/// The pending `TriggerOrder` prompt's branch labels. Panics unless the next
/// decision is a `TriggerOrder` owned by `player`.
pub fn order_choices(r: &DebugRunner, player: PlayerId) -> Vec<String> {
    let sel = r
        .pending_selection()
        .unwrap_or_else(|| panic!("expected a TriggerOrder prompt for P{player}; nothing pending"));
    assert_eq!(
        sel.kind,
        SelectionKind::TriggerOrder,
        "expected a TriggerOrder prompt for P{player}, got {sel:?}"
    );
    assert_eq!(
        sel.selecting_player, player,
        "the TriggerOrder prompt belongs to the wrong player: {sel:?}"
    );
    sel.effect_choices
        .as_ref()
        .expect("TriggerOrder branches")
        .iter()
        .map(|c| c.label.clone())
        .collect()
}

/// Resolve the pending `TriggerOrder` by picking the branch whose label starts
/// with `prefix` (a card id, or `"<card_id> slot <n>"`).
pub fn choose(r: &mut DebugRunner, prefix: &str) {
    let (player, action) = {
        let sel = r.pending_selection().expect("a TriggerOrder prompt");
        assert_eq!(sel.kind, SelectionKind::TriggerOrder, "{sel:?}");
        let choice = sel
            .effect_choices
            .as_ref()
            .expect("TriggerOrder branches")
            .iter()
            .find(|c| c.label == prefix || c.label.starts_with(&format!("{prefix} ")))
            .unwrap_or_else(|| panic!("no TriggerOrder branch for {prefix}: {sel:?}"));
        (sel.selecting_player, choice.action_id)
    };
    r.execute_action(player, action)
        .unwrap_or_else(|e| panic!("TriggerOrder pick {prefix} rejected: {e:?}"));
}

/// Resolve the pending target / replacement prompt by taking its ONLY legal
/// option. A prompt with several options (or a `TriggerOrder`, which is a
/// decision the scenario did not expect here) fails the test.
pub fn step_prompt(r: &mut DebugRunner) {
    let sel = r.pending_selection().expect("a pending prompt");
    assert_ne!(
        sel.kind,
        SelectionKind::TriggerOrder,
        "unexpected ordering decision: {sel:?}"
    );
    assert_eq!(
        sel.valid_action_ids.len(),
        1,
        "the scenario stages exactly one legal option: {sel:?}"
    );
    let (player, action) = (sel.selecting_player, sel.valid_action_ids[0]);
    r.execute_action(player, action)
        .unwrap_or_else(|e| panic!("prompt pick rejected: {e:?}"));
}

/// `step_prompt` until nothing is pending.
pub fn resolve_prompts(r: &mut DebugRunner) {
    for _ in 0..16 {
        if r.pending_selection().is_none() {
            return;
        }
        step_prompt(r);
    }
    panic!("prompts did not settle after 16 picks");
}

/// Resolve a pending hand pick by choosing `card_id` from `player`'s hand.
pub fn pick_hand_card(r: &mut DebugRunner, player: PlayerId, card_id: &str) {
    let index = hand_ids(r, player)
        .iter()
        .position(|c| c == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in P{player}'s hand"));
    let action = PLAY_HAND_START + index as u16;
    let sel = r.pending_selection().expect("a hand pick");
    assert_eq!(sel.selecting_player, player, "{sel:?}");
    assert!(
        sel.valid_action_ids.contains(&action),
        "{card_id} (hand index {index}) is a legal pick: {sel:?}"
    );
    r.execute_action(player, action)
        .unwrap_or_else(|e| panic!("hand pick {card_id} rejected: {e:?}"));
}

/// Resolve a pending own-/opponent-field pick by choosing battle-area `slot`
/// (`OwnField` / `OppField` encode a slot as `ATTACK_START + slot`).
pub fn pick_field(r: &mut DebugRunner, slot: u16) {
    let action = ATTACK_START + slot;
    let sel = r.pending_selection().expect("a field pick");
    assert!(
        matches!(sel.kind, SelectionKind::OwnField | SelectionKind::OppField),
        "expected a field pick, got {sel:?}"
    );
    assert!(
        sel.valid_action_ids.contains(&action),
        "slot {slot} is a legal pick: {sel:?}"
    );
    let player = sel.selecting_player;
    r.execute_action(player, action)
        .unwrap_or_else(|e| panic!("field pick {slot} rejected: {e:?}"));
}

/// Accept the pending optional prompt (an optional trigger or an optional
/// immediate-type effect), asserting it belongs to `player`.
pub fn accept(r: &mut DebugRunner, player: PlayerId) {
    let sel = r.pending_selection().expect("an optional prompt");
    assert!(sel.is_optional, "expected an optional prompt: {sel:?}");
    assert_eq!(sel.selecting_player, player, "{sel:?}");
    r.accept_optional_trigger()
        .unwrap_or_else(|e| panic!("accept rejected: {e:?}"));
}

/// Use the Option `card_id` from `player`'s hand ([Main] timing).
pub fn use_option(r: &mut DebugRunner, player: PlayerId, card_id: &str) {
    let index = hand_ids(r, player)
        .iter()
        .position(|c| c == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in P{player}'s hand"));
    let result = r.game.play_option_from_hand(player, index);
    assert_ne!(
        result,
        OptionPlayResult::Invalid,
        "{card_id} could not be used (phase {:?})",
        r.current_phase()
    );
}

/// `(player, source card id)` of every effect-sourced memory change since `cp`,
/// in event order.
pub fn memory_sources(r: &DebugRunner, cp: usize) -> Vec<(PlayerId, String)> {
    r.events_since(cp)
        .iter()
        .filter_map(|e| match e {
            GameEvent::MemoryChange {
                player,
                source_card_id: Some(id),
                ..
            } => Some((*player, id.clone())),
            _ => None,
        })
        .collect()
}

pub fn card_ids(r: &DebugRunner, cards: &[CardSource]) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

pub fn hand_ids(r: &DebugRunner, p: PlayerId) -> Vec<String> {
    card_ids(r, &r.game.player(p).hand)
}

pub fn trash_ids(r: &DebugRunner, p: PlayerId) -> Vec<String> {
    let mut ids = card_ids(r, &r.game.player(p).trash);
    ids.sort();
    ids
}

pub fn field_top_ids(r: &DebugRunner, p: PlayerId) -> Vec<String> {
    r.game
        .player(p)
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

pub fn owned(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|s| s.to_string()).collect()
}

/// Put a face-down `card_id` at the bottom of `host`'s stack (a card "under"
/// a Tamer).
pub fn attach_face_down(r: &mut DebugRunner, host: PermanentHandle, card_id: &str) {
    let data_index = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let index = r.game.next_card_index();
    let mut card = CardSource::new(data_index, host.player, index);
    card.face_down = true;
    r.game.players[host.player as usize].battle_area[host.index as usize]
        .card_sources
        .insert(0, card);
}

pub fn set_suspended(r: &mut DebugRunner, h: PermanentHandle, suspended: bool) {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended = suspended;
}

pub fn is_suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

/// Action id that declares the [Main] activated effect of the battle-area
/// permanent `h` (its `FIELD_EFFECT` slot for [Main]).
pub fn field_main_action(h: PermanentHandle) -> u16 {
    FIELD_EFFECT_START + h.index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN
}
