//! Phase 2f1 Task 3b — `EffectContext::play_from_materials` removes a
//! specific source from a permanent's digivolution stack and plays the
//! underlying card into battle area, with the supplied cost-delta semantics.
//!
//! Card text precedent: BT15-080 "place this card's bottom material into
//! battle area as a Digimon".
//!
//! ## Audit finding: hand-transit, mirroring `play_from_security`
//!
//! `Game::play_from_hand_with_cost(player, hand_index, cost_delta)` already
//! covers the full placement path — battle-area capacity check,
//! `CannotPlayDigimonByEffect` gate, `Permanent::new`, OnPlay drain,
//! OnEnterFieldAnyone broadcast, `Play` event emission. Re-introducing that
//! body would duplicate logic and risk drift. Instead: pop the chosen source
//! out of `perm.card_sources`, push to the end of `player.hand`, route
//! through `play_from_hand_with_cost` at that index. This mirrors the
//! hand-transit pattern committed in Task 3a (`play_from_security`).

use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CostDelta;

/// Helper: build a permanent on P0 with two stacked sources (base + top).
/// Returns (permanent_handle, base_source_handle, top_source_handle).
fn place_two_source_perm(
    runner: &mut DebugRunner,
) -> (
    digimon_engine::permanent::PermanentHandle,
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
fn play_from_materials_removes_source_and_plays_it_free() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("T-BASE", "BasePermTest"))
        .add_card(make_test_card("T-TOP", "TopMaterialTest"))
        .memory(3)
        .start();

    let (perm_handle, base_h, _top_h) = place_two_source_perm(&mut runner);

    assert_eq!(runner.game.memory, 3, "precondition: memory == 3");
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "precondition: P0 battle area has 1 permanent"
    );
    assert_eq!(
        runner.game.players[0].battle_area[0].stack_size(),
        2,
        "precondition: stack has 2 sources"
    );

    // Use the top of the stack as the source-card handle for the
    // EffectContext (typical: an effect on the top card extracts a material
    // from below itself).
    let src_handle = runner.game.players[0].battle_area[0].top_card().handle();

    // Play the BASE source (index 0) for free.
    let new_handle = {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, None, 0);
        ctx.play_from_materials(perm_handle, 0, CostDelta::Free)
    };
    assert!(
        new_handle.is_some(),
        "play_from_materials should succeed when index is in range"
    );

    // Memory unchanged for Free.
    assert_eq!(
        runner.game.memory, 3,
        "play_from_materials with CostDelta::Free must not change memory"
    );

    // Source 0 (base) was removed from the permanent's stack.
    assert_eq!(
        runner.game.players[0].battle_area[0].stack_size(),
        1,
        "base source must be removed from original permanent's card_sources"
    );

    // A NEW permanent appears on P0's battle area, top card matches the
    // extracted source's handle.
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        2,
        "a new permanent should have been added to the battle area"
    );
    let new_idx = new_handle.unwrap().index as usize;
    let top: &CardSource = runner.game.players[0].battle_area[new_idx].top_card();
    assert_eq!(
        top.handle(),
        base_h,
        "the new permanent's top must be the extracted source"
    );
}

#[test]
fn play_from_materials_subtracts_memory_under_fixed_cost() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("T-BASE", "BasePermTest"))
        .add_card(make_test_card("T-TOP", "TopMaterialTest"))
        .memory(5)
        .start();

    let (perm_handle, _base_h, _top_h) = place_two_source_perm(&mut runner);

    let src_handle = runner.game.players[0].battle_area[0].top_card().handle();

    // Fixed cost of 2 — memory should drop by 2 regardless of printed cost.
    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, None, 0);
        ctx.play_from_materials(perm_handle, 0, CostDelta::Fixed(2))
    };
    assert!(result.is_some(), "play succeeds under affordable Fixed cost");

    assert_eq!(
        runner.game.memory, 3,
        "memory must drop by exactly 2 under CostDelta::Fixed(2)"
    );
    assert_eq!(
        runner.game.players[0].battle_area[0].stack_size(),
        1,
        "source removed from original stack"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        2,
        "new permanent added"
    );
}

#[test]
fn play_from_materials_returns_none_for_out_of_bounds_source_index() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("T-BASE", "BasePermTest"))
        .add_card(make_test_card("T-TOP", "TopMaterialTest"))
        .memory(3)
        .start();

    let (perm_handle, _base_h, _top_h) = place_two_source_perm(&mut runner);
    let src_handle = runner.game.players[0].battle_area[0].top_card().handle();

    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, None, 0);
        ctx.play_from_materials(perm_handle, 99, CostDelta::Free)
    };

    assert!(
        result.is_none(),
        "out-of-bounds source_index must return None"
    );
    assert_eq!(runner.game.memory, 3, "memory untouched on failed play");
    assert_eq!(
        runner.game.players[0].battle_area[0].stack_size(),
        2,
        "original stack unchanged on out-of-bounds index"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "no new permanent on failed play"
    );
}

#[test]
fn play_from_materials_returns_none_for_invalid_permanent_handle() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("T-BASE", "BasePermTest"))
        .memory(3)
        .start();

    // No permanent on the field; fabricate a handle that doesn't exist.
    let bogus = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    let src_handle = digimon_engine::card_source::CardHandle(0);

    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, None, 0);
        ctx.play_from_materials(bogus, 0, CostDelta::Free)
    };

    assert!(
        result.is_none(),
        "invalid permanent handle must return None"
    );
    assert_eq!(runner.game.memory, 3, "memory untouched on failed play");
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        0,
        "no new permanent on failed play"
    );
}
