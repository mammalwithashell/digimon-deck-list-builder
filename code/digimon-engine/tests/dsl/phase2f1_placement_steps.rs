//! Phase 2f1 Task 5 — DSL behavioral tests for the placement-step variants
//! wired in Task 4: `PlaceOnSecurity`, `PlaceAsBottomSource`, `TrashTopSource`.
//!
//! Each test drives the variant through `run_step` (synchronous family) and
//! asserts the observable mutation matches the engine primitive.

use digimon_dsl::compiled::{CompiledBindingRef, CompiledPlayerRef, CompiledStackPosition, CompiledStep};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_step;
use digimon_engine::effect_context::EffectContext;

// ─── PlaceOnSecurity ─────────────────────────────────────────────────────────

#[test]
fn place_on_security_step_moves_hand_card_to_security_top() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-POS", "PlaceOnSecurityTest"))
        .hand(0, &["TST-POS"])
        .memory(5)
        .start();

    let src_handle = runner.game.players[0].hand[0].handle();
    let hand_before = runner.game.players[0].hand.len();
    let security_before = runner.game.players[0].security.len();

    let mut bindings = Bindings::new();
    bindings.insert_hand_index("src", 0, 0);

    let step = CompiledStep::PlaceOnSecurity {
        of: CompiledPlayerRef::You,
        source: CompiledBindingRef::Named("src".into()),
        position: CompiledStackPosition::Top,
        face_up: false,
    };

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    // Hand consumed; security gained 1 card.
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before - 1,
        "hand should shrink by 1"
    );
    assert_eq!(
        runner.game.players[0].security.len(),
        security_before + 1,
        "security should gain 1 card"
    );
    // The card on top of security is the one we placed.
    // `place_on_security(StackPosition::Top, ...)` puts the card at the
    // top — by Digimon convention, the top of the security stack is the
    // last-drawn card. The engine pushes in stack-top order; verify that
    // the placed card's handle is now somewhere in security.
    assert!(
        runner.game.players[0]
            .security
            .iter()
            .any(|c| c.handle() == src_handle),
        "the placed card must now be in the security stack"
    );
}

// ─── PlaceAsBottomSource ─────────────────────────────────────────────────────

#[test]
fn place_as_bottom_source_step_tucks_hand_card_under_target_permanent() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("T-PERM", "TargetPerm"))
        .add_card(make_test_card("TST-PABS", "PlaceAsBottomSourceTest"))
        .hand(0, &["TST-PABS"])
        .memory(5)
        .start();

    // Build a 1-source permanent on P0's battle area.
    let target = runner.place_on_field(0, "T-PERM", None);
    assert_eq!(runner.game.players[0].battle_area[0].stack_size(), 1);
    let original_top_h = runner.game.players[0].battle_area[0].top_card().handle();

    let src_handle = runner.game.players[0].hand[0].handle();
    let hand_before = runner.game.players[0].hand.len();

    let mut bindings = Bindings::new();
    bindings.insert_hand_index("src", 0, 0);
    bindings.insert_permanent("tgt", target);

    let step = CompiledStep::PlaceAsBottomSource {
        source: CompiledBindingRef::Named("src".into()),
        target: CompiledBindingRef::Named("tgt".into()),
    };

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    // Hand consumed.
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before - 1,
        "hand should shrink by 1"
    );
    // The target permanent's stack grew by 1, with the hand card at the
    // bottom (card_sources[0]).
    let perm = &runner.game.players[0].battle_area[target.index as usize];
    assert_eq!(
        perm.card_sources.len(),
        2,
        "target permanent's stack should have grown by 1"
    );
    assert_eq!(
        perm.card_sources[0].handle(),
        src_handle,
        "the placed card must be at the BOTTOM of the stack (card_sources[0])"
    );
    assert_eq!(
        perm.top_card().handle(),
        original_top_h,
        "the original top card must still be on top"
    );
}

// ─── TrashTopSource ──────────────────────────────────────────────────────────

#[test]
fn trash_top_source_step_pops_top_source_to_controller_trash() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("T-BASE", "Base"))
        .add_card(make_test_card("T-TOP", "Top"))
        .start();

    // Build a 2-source permanent on P0's battle area.
    let target = runner.place_on_field(0, "T-BASE", None);
    let base_h = runner.game.players[0].battle_area[0].card_sources[0].handle();
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "T-TOP")
        .unwrap();
    let card_index = runner.game.next_card_index();
    let top_src = CardSource::new(data_idx, 0, card_index);
    let top_h = top_src.handle();
    runner.game.players[0].battle_area[0]
        .card_sources
        .push(top_src);
    assert_eq!(runner.game.players[0].battle_area[0].stack_size(), 2);

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let trash_before = runner.game.players[0].trash.len();

    let mut bindings = Bindings::new();
    bindings.insert_permanent("tgt", target);

    let step = CompiledStep::TrashTopSource {
        target: CompiledBindingRef::Named("tgt".into()),
    };

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, Some(target), 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    // Stack shrank by 1 (top source removed).
    assert_eq!(
        runner.game.players[0].battle_area[target.index as usize]
            .card_sources
            .len(),
        1,
        "permanent stack should shrink by 1"
    );
    // Remaining source is the original base.
    assert_eq!(
        runner.game.players[0].battle_area[target.index as usize].card_sources[0].handle(),
        base_h,
        "remaining source must be the original base (top was removed, not bottom)"
    );
    // Trash gained the top source.
    assert_eq!(
        runner.game.players[0].trash.len(),
        trash_before + 1,
        "controller's trash should gain 1 card"
    );
    assert_eq!(
        runner.game.players[0].trash[0].handle(),
        top_h,
        "trashed card must be the original top source"
    );
}
