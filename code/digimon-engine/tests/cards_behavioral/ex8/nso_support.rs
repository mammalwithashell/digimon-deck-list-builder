//! Shared fixtures for the EX8 [NSo] (Nightmare Soldiers) card tests
//! (EX8-006 / 032 / 033 / 060 / 062 / 064).
//!
//! Boards are SYNTHETIC: generic cards that carry the traits/levels the
//! printed text keys on, never recorded decks.

#![allow(dead_code)]

use digimon_engine::action::space::{
    encode_attack, HAND_EFFECT_START, PASS, PLAY_HAND_START, TRASH_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::owned_card_choice::CardChoiceZone;

/// A Digimon carrying the [NSo] trait.
pub fn nso(id: &str, level: u8, dp: i32, cost: u16, color: CardColor) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.traits = vec!["NSo".to_string()];
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card
}

/// A Digimon with no [NSo] trait.
pub fn plain(id: &str, level: u8, dp: i32, color: CardColor) -> CardData {
    let mut card = nso(id, level, dp, level as u16, color);
    card.traits = vec!["Other".to_string()];
    card
}

pub fn data_idx(r: &DebugRunner, id: &str) -> usize {
    r.game
        .card_data
        .iter()
        .position(|c| c.card_id == id)
        .unwrap_or_else(|| panic!("unknown card id {id}"))
}

pub fn push_hand(r: &mut DebugRunner, p: u8, id: &str) -> usize {
    let idx = data_idx(r, id);
    let next = r.game.next_card_index();
    r.game.players[p as usize]
        .hand
        .push(CardSource::new(idx, p, next));
    r.game.players[p as usize].hand.len() - 1
}

pub fn push_trash(r: &mut DebugRunner, p: u8, id: &str) {
    let idx = data_idx(r, id);
    let next = r.game.next_card_index();
    r.game.players[p as usize]
        .trash
        .push(CardSource::new(idx, p, next));
}

pub fn hand_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

pub fn trash_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

pub fn field_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

pub fn slot_of(r: &DebugRunner, p: u8, id: &str) -> usize {
    field_ids(r, p)
        .iter()
        .position(|s| s == id)
        .unwrap_or_else(|| panic!("{id} is not on player {p}'s field"))
}

pub fn handle_of(r: &DebugRunner, p: u8, id: &str) -> PermanentHandle {
    PermanentHandle {
        player: p,
        index: slot_of(r, p, id) as u8,
    }
}

/// Pick a hand card at the pending hand selection. Selections that carry an
/// owned physical menu are answered through it; plain `Hand` selections
/// (e.g. a discard cost) through the hand-slot action ids.
pub fn pick_hand(r: &mut DebugRunner, p: u8, id: &str) {
    let view = r.pending_selection_view().expect("a pending hand selection");
    let action = if view.card_choice.is_some() {
        physical_choice::card_action(&r.game, id, CardChoiceZone::Hand)
    } else {
        let slot = hand_ids(r, p)
            .iter()
            .position(|s| s == id)
            .unwrap_or_else(|| panic!("{id} not in hand")) as u16;
        [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
            .into_iter()
            .find(|a| view.valid_action_ids.contains(a))
            .unwrap_or_else(|| panic!("{id} not selectable: {view:?}"))
    };
    r.execute_action(view.selecting_player, action).expect("pick hand card");
}

/// Pick a card from the trash at the pending trash selection (the owned
/// physical menu when it has one, else the trash-slot action ids).
pub fn pick_trash(r: &mut DebugRunner, id: &str) {
    let view = r.pending_selection_view().expect("a pending trash selection");
    let action = if view.card_choice.is_some() {
        physical_choice::card_action(&r.game, id, CardChoiceZone::Trash)
    } else {
        let owner = view.zone_owner.unwrap_or(view.selecting_player);
        let slot = trash_ids(r, owner)
            .iter()
            .position(|s| s == id)
            .unwrap_or_else(|| panic!("{id} not in trash")) as u16;
        let action = TRASH_EFFECT_START + slot;
        assert!(view.valid_action_ids.contains(&action), "{id} not selectable: {view:?}");
        action
    };
    r.execute_action(view.selecting_player, action).expect("pick trash card");
}

/// The printed card ids offered as legal commits by the pending prompt (hand or
/// trash cards for a plain, menu-less selection).
pub fn offered_ids(r: &DebugRunner) -> Vec<String> {
    let view = r.pending_selection_view().expect("a pending selection");
    if view.card_choice.is_some() {
        return physical_choice::commits(&r.game)
            .iter()
            .filter_map(|c| c.card_id.clone())
            .collect();
    }
    let hand = hand_ids(r, view.selecting_player);
    let trash = trash_ids(r, view.zone_owner.unwrap_or(view.selecting_player));
    view.valid_action_ids
        .iter()
        .filter_map(|a| match *a {
            0..=59 => hand.get((*a % 30) as usize).cloned(),
            TRASH_EFFECT_START..=1194 => trash.get((*a - TRASH_EFFECT_START) as usize).cloned(),
            _ => None,
        })
        .collect()
}

/// Declare the maximum at every pending count declaration (<De-Digivolve N>).
pub fn declare_max(r: &mut DebugRunner) {
    for _ in 0..8 {
        if !r.pending_is_count_declaration() {
            return;
        }
        r.declare_max_count().expect("declare the maximum");
    }
}

/// Pick a battle-area permanent (either player's) at a pending field prompt.
pub fn pick_field(r: &mut DebugRunner, owner: u8, id: &str) {
    let view = r.pending_selection_view().expect("a pending field selection");
    let slot = slot_of(r, owner, id) as u16;
    let action = encode_attack(0, slot);
    assert!(
        view.valid_action_ids.contains(&action),
        "{id} (slot {slot}) not selectable: {view:?}"
    );
    r.execute_action(view.selecting_player, action).expect("pick field permanent");
}

pub fn pass(r: &mut DebugRunner) {
    let view = r.pending_selection_view().expect("a pending selection to pass");
    assert!(
        view.is_optional || view.valid_action_ids.contains(&PASS),
        "selection is not optional: {view:?}"
    );
    r.execute_action(view.selecting_player, PASS).expect("pass");
}

/// Keep taking the first non-PASS action until nothing is pending (guards a
/// runaway loop). Used to walk past attack windows that are not under test.
pub fn drain_declining(r: &mut DebugRunner) {
    for _ in 0..32 {
        let Some(view) = r.pending_selection_view() else {
            return;
        };
        let action = if view.is_optional || view.valid_action_ids.contains(&PASS) {
            PASS
        } else {
            view.valid_action_ids[0]
        };
        r.execute_action(view.selecting_player, action).expect("drain");
    }
}

#[path = "../../support/owned_card_choice.rs"]
mod physical_choice;
pub use physical_choice::{card_action, commits};
