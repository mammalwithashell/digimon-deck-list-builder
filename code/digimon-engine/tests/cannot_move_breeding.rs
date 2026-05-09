//! `ModifierType::CannotMove` consult-site test (Track C / D, 2026-05-08).
//!
//! Pins enforcement at `Game::move_from_breeding` for both the
//! player-action path and `move_from_breeding_by_effect` (which delegates
//! to the same helper). A `CannotMove` modifier installed on the
//! breeding-area handle (`{ player, index: BREEDING_TARGET }`) blocks the
//! move; the breeding permanent stays put. Distinct from `CannotSuspend`,
//! which only blocks orientation flips.

use digimon_engine::action::space::BREEDING_TARGET;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;

fn lv3_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c
}

fn breeding_handle(player: u8) -> PermanentHandle {
    PermanentHandle {
        player,
        index: BREEDING_TARGET as u8,
    }
}

fn runner_with_lv3_in_breeding() -> DebugRunner {
    let mut r = DebugRunner::builder().add_card(lv3_digimon("EGG3")).start();
    r.place_in_breeding(0, "EGG3");
    r
}

#[test]
fn baseline_breeding_move_succeeds_without_modifier() {
    // Sanity: with no `CannotMove` installed, the level-3 breeding
    // permanent moves to the battle area as usual. Establishes that
    // the runner's setup is otherwise legal so the next two tests
    // cleanly attribute failures to the modifier.
    let mut r = runner_with_lv3_in_breeding();

    assert!(
        r.game.move_from_breeding(0),
        "level-3 breeding permanent must move when no CannotMove is set"
    );
    assert!(r.game.player(0).breeding_area.is_none());
    assert_eq!(r.game.player(0).battle_area.len(), 1);
}

#[test]
fn cannot_move_blocks_player_action_breeding_to_battle() {
    let mut r = runner_with_lv3_in_breeding();

    r.game.modifiers.add(
        breeding_handle(0),
        ModifierEntry::simple(ModifierType::CannotMove, 0, Expiry::Permanent, 0),
    );

    assert!(
        !r.game.move_from_breeding(0),
        "CannotMove on the breeding permanent must block the player-action move"
    );
    assert!(
        r.game.player(0).breeding_area.is_some(),
        "breeding permanent must stay in the breeding area"
    );
    assert_eq!(
        r.game.player(0).battle_area.len(),
        0,
        "no permanent must enter the battle area"
    );
}

#[test]
fn cannot_move_blocks_effect_driven_breeding_to_battle() {
    // `move_from_breeding_by_effect` delegates to `move_from_breeding`, so
    // the same gate covers effect-initiated promotion. Pinning explicitly
    // so a future divergence between the two helpers can't slip past.
    let mut r = runner_with_lv3_in_breeding();

    r.game.modifiers.add(
        breeding_handle(0),
        ModifierEntry::simple(ModifierType::CannotMove, 0, Expiry::Permanent, 0),
    );

    assert!(
        !r.game.move_from_breeding_by_effect(0),
        "CannotMove must block the effect-driven move helper"
    );
    assert!(r.game.player(0).breeding_area.is_some());
    assert_eq!(r.game.player(0).battle_area.len(), 0);
}

#[test]
fn cannot_move_on_opposing_breeding_does_not_block_own_move() {
    // The modifier is permanent-scoped and keyed by handle. A
    // CannotMove installed on the OPPOSING player's breeding handle
    // must not affect P0's move.
    let mut r = DebugRunner::builder().add_card(lv3_digimon("EGG3")).start();
    r.place_in_breeding(0, "EGG3");

    r.game.modifiers.add(
        breeding_handle(1),
        ModifierEntry::simple(ModifierType::CannotMove, 0, Expiry::Permanent, 1),
    );

    assert!(
        r.game.move_from_breeding(0),
        "CannotMove on opponent's breeding handle must not block P0's move"
    );
}
