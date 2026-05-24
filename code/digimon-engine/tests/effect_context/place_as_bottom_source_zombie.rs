//! Regression tests for `G-PERMANENT-EMPTY-DURING-MATERIAL-EXTRACTION` —
//! sibling of the digivolve-from-material fix landed in PR #533.
//!
//! `Game::place_as_bottom_source_observed` (`game_actions.rs`) takes a
//! `CardSourceRef::Material(carrier, index)` source and pushes it under a
//! target permanent. When the carrier's `card_sources` Vec contains only the
//! single source being extracted, the carrier ends with an empty
//! `card_sources` Vec but its slot remains in `battle_area` — a "zombie"
//! permanent. Any downstream trigger fan-out that iterates `battle_area`
//! and calls `Permanent::top_card()` panics with
//! `Permanent must have at least one card`.
//!
//! These tests assert the post-condition that no permanent in any player's
//! `battle_area` has `card_sources.is_empty()` after the place operation.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardSourceRef;

/// Base zombie regression: single-source carrier at a HIGHER battle_area
/// index than the target. After place, the carrier slot must be removed
/// rather than left as a zombie. The target's index is unaffected (carrier
/// at higher index → no shift on the target).
#[test]
fn place_as_bottom_source_from_material_emptying_carrier_removes_slot() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("T-TARGET", "TargetTest"))
        .add_card(make_test_card("T-CARRIER", "CarrierTest"))
        .memory(3)
        .start();

    // Target at index 0 — receives the bottom source.
    let target = runner.place_on_field(0, "T-TARGET", None);
    assert_eq!(target.index, 0);
    // Carrier at index 1 — its only card is what we'll route under target.
    let carrier = runner.place_on_field(0, "T-CARRIER", None);
    assert_eq!(carrier.index, 1);
    assert_eq!(
        runner.game.players[0].battle_area[carrier.index as usize]
            .card_sources
            .len(),
        1,
        "precondition: carrier has exactly 1 source"
    );

    let src_handle = runner.game.players[0].battle_area[target.index as usize]
        .top_card()
        .handle();

    let ok = {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, Some(target), 0);
        ctx.place_as_bottom_source(CardSourceRef::Material(carrier, 0), target, false)
    };
    assert!(ok, "place_as_bottom_source should succeed");

    // No zombies anywhere.
    for (pid, player) in runner.game.players.iter().enumerate() {
        for (slot, perm) in player.battle_area.iter().enumerate() {
            assert!(
                !perm.card_sources.is_empty(),
                "zombie permanent at p{}.battle_area[{}]: card_sources is empty",
                pid,
                slot
            );
        }
    }

    // Carrier should be gone, target should hold the routed card at the bottom.
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "carrier slot should be removed after soft-remove"
    );
    assert_eq!(
        runner.game.players[0].battle_area[0].card_sources.len(),
        2,
        "target should have 2 cards: routed bottom + its original top"
    );
}

/// Lower-indexed carrier variant: the carrier is at a LOWER index than the
/// target. After the soft-remove, the target's index shifts down by 1; the
/// `Game::place_as_bottom_source_observed` cleanup must adjust the target
/// handle BEFORE the `battle_area[target.index].push_under(card)` read.
#[test]
fn place_as_bottom_source_lower_indexed_carrier_shifts_target_index() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("T-CARRIER", "CarrierTest"))
        .add_card(make_test_card("T-TARGET", "TargetTest"))
        .memory(3)
        .start();

    // Carrier at index 0 — will be soft-removed after extraction.
    let carrier = runner.place_on_field(0, "T-CARRIER", None);
    assert_eq!(carrier.index, 0);
    // Target at index 1 — should shift down to index 0.
    let target = runner.place_on_field(0, "T-TARGET", None);
    assert_eq!(target.index, 1);

    let target_top_pre = runner.game.players[0].battle_area[target.index as usize]
        .top_card()
        .handle();
    let carrier_card_pre = runner.game.players[0].battle_area[carrier.index as usize]
        .top_card()
        .handle();

    let src_handle = target_top_pre;

    let ok = {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, Some(target), 0);
        ctx.place_as_bottom_source(CardSourceRef::Material(carrier, 0), target, false)
    };
    assert!(ok, "place should succeed");

    // No zombies.
    for (pid, player) in runner.game.players.iter().enumerate() {
        for (slot, perm) in player.battle_area.iter().enumerate() {
            assert!(
                !perm.card_sources.is_empty(),
                "zombie at p{}.battle_area[{}]",
                pid,
                slot
            );
        }
    }

    // Only the target remains, at the post-shift index 0, with 2 cards:
    // the carrier's card (bottom) + its original top.
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "only the target should remain — carrier was soft-removed"
    );
    let surviving = &runner.game.players[0].battle_area[0];
    assert_eq!(
        surviving.card_sources.len(),
        2,
        "target should have routed bottom + original top"
    );
    assert_eq!(
        surviving.card_sources[0].handle(),
        carrier_card_pre,
        "carrier's card must be at the bottom of the target stack"
    );
    assert_eq!(
        surviving.card_sources[1].handle(),
        target_top_pre,
        "target's original top must remain on top after the place"
    );
}
