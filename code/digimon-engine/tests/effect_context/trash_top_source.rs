//! Phase 2f1 Task 3d — `EffectContext::trash_top_source` strips the top
//! digivolution source from a permanent and routes it to the controller's
//! trash. Card-text precedent: "trash the top digivolution source of this
//! Digimon".
//!
//! Distinct from `trash_card_source` (which targets a specific source by
//! handle anywhere in the stack) and `armor_purge_top` (which has the
//! same effect but panics on insufficient stack and is intended for the
//! `<Armor Purge>` keyword auto-install). This primitive returns `bool`
//! so card-script callers can branch on success without panic.

use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::permanent::PermanentHandle;

/// Helper: build a permanent on P0 with two stacked sources (base + top).
/// Returns (permanent_handle, base_source_handle, top_source_handle).
fn place_two_source_perm(
    runner: &mut DebugRunner,
) -> (
    PermanentHandle,
    digimon_engine::card_source::CardHandle,
    digimon_engine::card_source::CardHandle,
) {
    // Place the base card via the standard path.
    let handle = runner.place_on_field(0, "T-BASE", None);
    assert_eq!(runner.game.players[0].battle_area[0].stack_size(), 1);
    let base_h = runner.game.players[0].battle_area[0].card_sources[0].handle();

    // Manually push a second CardSource on top.
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "T-TOP")
        .expect("T-TOP should be registered");
    let card_index = runner.game.next_card_index();
    let top_src = CardSource::new(data_idx, 0, card_index);
    let top_h = top_src.handle();
    runner.game.players[0].battle_area[0]
        .card_sources
        .push(top_src);

    assert_eq!(runner.game.players[0].battle_area[0].stack_size(), 2);
    (handle, base_h, top_h)
}

#[test]
fn trash_top_source_removes_top_and_routes_to_trash() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("T-BASE", "BasePermTest"))
        .add_card(make_test_card("T-TOP", "TopMaterialTest"))
        .start();

    let (perm_handle, base_h, top_h) = place_two_source_perm(&mut runner);

    // Sanity: the captured top handle is indeed at index 1.
    assert_eq!(
        runner.game.players[0].battle_area[0].card_sources[1].handle(),
        top_h,
        "precondition: index 1 == top source handle"
    );
    assert_eq!(
        runner.game.players[0].trash.len(),
        0,
        "precondition: trash empty"
    );

    let src_handle = runner.game.players[0].battle_area[0].top_card().handle();

    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, Some(perm_handle), 0);
        ctx.trash_top_source(perm_handle)
    };

    assert!(result, "trash_top_source returns true on success");

    // Stack size shrinks by exactly 1.
    assert_eq!(
        runner.game.players[0].battle_area[0].card_sources.len(),
        1,
        "stack should shrink to 1 after trashing the top source"
    );
    // The remaining source is the original base — proves the TOP went, not
    // the bottom.
    assert_eq!(
        runner.game.players[0].battle_area[0].card_sources[0].handle(),
        base_h,
        "remaining source must be the original base (not the trashed top)"
    );

    // The trashed card is now in the controller's trash, identified by handle.
    let trash = &runner.game.players[0].trash;
    assert_eq!(trash.len(), 1, "exactly one card moved to trash");
    assert_eq!(
        trash[0].handle(),
        top_h,
        "trashed card handle must match the original top source"
    );
}

#[test]
fn trash_top_source_returns_false_on_empty_stack() {
    // Build a runner with a permanent that has had its only source manually
    // removed — stack length 0. (Not a normal in-game state, but the
    // primitive's defensive contract is that empty-stack callers get a clean
    // `false`, no panic, no mutation.)
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("T-BASE", "BasePermTest"))
        .start();

    let perm_handle = runner.place_on_field(0, "T-BASE", None);
    // Drain the stack to 0 sources by hand.
    runner.game.players[0].battle_area[perm_handle.index as usize]
        .card_sources
        .clear();
    assert_eq!(
        runner.game.players[0].battle_area[perm_handle.index as usize]
            .card_sources
            .len(),
        0
    );

    // Use a dummy src_handle from another zone — no top_card to pull from.
    // Construct a synthetic CardHandle by reading any other card we have.
    // The simplest path: re-read a handle from the empty stack permanent's
    // permanent_handle — but card_sources is empty. We use a trash-pushed
    // sentinel instead by adding any CardSource manually first, capturing
    // its handle, then clearing.
    // Easier: make a fresh CardSource and capture its handle without
    // attaching it anywhere we care about — the EffectContext only stores
    // the handle, doesn't dereference it for this primitive.
    let dummy_idx = runner.game.next_card_index();
    let dummy_src = CardSource::new(0, 0, dummy_idx);
    let dummy_handle = dummy_src.handle();

    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, dummy_handle, Some(perm_handle), 0);
        ctx.trash_top_source(perm_handle)
    };

    assert!(!result, "trash_top_source returns false on empty stack");
    assert_eq!(
        runner.game.players[0].battle_area[perm_handle.index as usize]
            .card_sources
            .len(),
        0,
        "empty-stack: no mutation"
    );
    assert_eq!(
        runner.game.players[0].trash.len(),
        0,
        "empty-stack: nothing pushed to trash"
    );
}

#[test]
fn trash_top_source_returns_false_on_invalid_handle() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("T-BASE", "BasePermTest"))
        .start();

    // Place a single permanent so the battle area has length 1.
    let valid_handle = runner.place_on_field(0, "T-BASE", None);

    // Construct an out-of-bounds permanent handle (player has only 1 perm,
    // index 5 is invalid).
    let bogus_handle = PermanentHandle {
        player: 0,
        index: 5,
    };

    let src_handle = runner.game.players[0].battle_area[valid_handle.index as usize]
        .top_card()
        .handle();

    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, Some(valid_handle), 0);
        ctx.trash_top_source(bogus_handle)
    };

    assert!(!result, "trash_top_source returns false on invalid handle");

    // The valid permanent was untouched.
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "battle area length unchanged on invalid handle"
    );
    assert_eq!(
        runner.game.players[0].battle_area[valid_handle.index as usize]
            .card_sources
            .len(),
        1,
        "valid permanent's stack unchanged on invalid handle"
    );
    assert_eq!(
        runner.game.players[0].trash.len(),
        0,
        "invalid handle: nothing pushed to trash"
    );
}
