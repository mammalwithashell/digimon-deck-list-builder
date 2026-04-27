//! Phase 2c — permanent mutation step dispatch.

use digimon_dsl::compiled::{CompiledBindingRef, CompiledStep};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_step;
use digimon_engine::effect_context::EffectContext;

#[test]
fn delete_permanent_via_named_binding_removes_from_battle_area() {
    let card = make_test_card("T-DEL", "T-DEL");
    let mut runner = DebugRunner::builder()
        .add_card(card.clone())
        .hand(0, &["T-DEL"])
        .build();

    // Place the card on the field (player 0).
    let handle = runner.place_on_field(0, "T-DEL", None);
    assert_eq!(runner.game.players[0].battle_area.len(), 1);

    // Source card: use the permanent's top card as the "caster".
    let src_card = runner.game.players[0].battle_area[0].top_card().handle();

    let step = CompiledStep::DeletePermanent {
        target: CompiledBindingRef::Named("tgt".into()),
    };
    let mut bindings = Bindings::new();
    bindings.insert_permanent("tgt", handle);

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.players[0].battle_area.len(),
        0,
        "permanent should have been deleted from battle area"
    );
}

#[test]
fn return_to_hand_moves_permanent_to_owner_hand() {
    let card = make_test_card("T-RTH", "T-RTH");
    let mut runner = DebugRunner::builder()
        .add_card(card.clone())
        .hand(0, &["T-RTH"])
        .memory(5) // pre-fund so the 3-cost play is affordable
        .start(); // advance past Mulligan into turn 1

    // Capture hand size before playing (net-zero round-trip baseline).
    let hand_before = runner.game.players[0].hand.len();

    // Play the card from hand: hand -1, battle_area +1.
    let field_index = runner
        .play(0, 0)
        .expect("play should succeed with enough memory");
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
    assert_eq!(runner.game.players[0].hand.len(), hand_before - 1);

    let handle = runner.perm_handle(0, field_index);
    let src_card = runner.game.players[0].battle_area[field_index]
        .top_card()
        .handle();

    let step = CompiledStep::ReturnToHand {
        target: CompiledBindingRef::Named("tgt".into()),
    };
    let mut bindings = Bindings::new();
    bindings.insert_permanent("tgt", handle);

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.players[0].battle_area.len(),
        0,
        "permanent should have left the battle area"
    );
    // Net-zero round-trip: play decremented hand by 1, ReturnToHand increments
    // it back by 1; final hand size must equal the original hand_before.
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "hand size should be net-zero after play + ReturnToHand"
    );
}

#[test]
fn suspend_then_unsuspend_round_trip() {
    let card = make_test_card("T-SUS", "T-SUS");
    let mut runner = DebugRunner::builder()
        .add_card(card.clone())
        .hand(0, &["T-SUS"])
        .build();

    let handle = runner.place_on_field(0, "T-SUS", None);
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
    assert!(
        !runner.game.players[0].battle_area[0].is_suspended,
        "permanent should start unsuspended"
    );

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let mut bindings = Bindings::new();
    bindings.insert_permanent("tgt", handle);

    // --- Suspend ---
    let suspend_step = CompiledStep::Suspend {
        target: CompiledBindingRef::Named("tgt".into()),
    };
    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&suspend_step, &mut ctx, &mut bindings);
    }
    assert!(
        runner.game.players[0].battle_area[0].is_suspended,
        "permanent should be suspended after Suspend step"
    );

    // --- Unsuspend ---
    let unsuspend_step = CompiledStep::Unsuspend {
        target: CompiledBindingRef::Named("tgt".into()),
    };
    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&unsuspend_step, &mut ctx, &mut bindings);
    }
    assert!(
        !runner.game.players[0].battle_area[0].is_suspended,
        "permanent should be unsuspended after Unsuspend step"
    );
}

#[test]
fn return_to_deck_top_removes_permanent() {
    let card = make_test_card("T-RTD", "T-RTD");
    let mut runner = DebugRunner::builder()
        .add_card(card.clone())
        .hand(0, &["T-RTD"])
        .build();

    let handle = runner.place_on_field(0, "T-RTD", None);
    assert_eq!(runner.game.players[0].battle_area.len(), 1);

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let mut bindings = Bindings::new();
    bindings.insert_permanent("tgt", handle);

    let step = CompiledStep::ReturnToDeck {
        target: CompiledBindingRef::Named("tgt".into()),
        position: digimon_dsl::compiled::CompiledStackPosition::Top,
        include_sources: false,
    };
    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.players[0].battle_area.len(),
        0,
        "permanent should have been removed from battle area by ReturnToDeck"
    );
}

#[test]
fn de_digivolve_amount_one_pops_one_source() {
    // Build a permanent with 2 CardSources: a base card + one digivolved on top.
    let base_card = make_test_card("T-BASE", "T-BASE");
    let top_card_data = make_test_card("T-TOP", "T-TOP");
    let mut runner = DebugRunner::builder()
        .add_card(base_card.clone())
        .add_card(top_card_data.clone())
        .hand(0, &["T-BASE"])
        .build();

    // Place the base card on the field (1 source).
    let handle = runner.place_on_field(0, "T-BASE", None);
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
    assert_eq!(runner.game.players[0].battle_area[0].stack_size(), 1);

    // Manually push a second CardSource on top to simulate digivolving.
    {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "T-TOP")
            .expect("T-TOP should be registered");
        let card_index = runner.game.next_card_index();
        let top_src = digimon_engine::card_source::CardSource::new(data_idx, 0, card_index);
        runner.game.players[0].battle_area[0]
            .card_sources
            .push(top_src);
    }
    assert_eq!(
        runner.game.players[0].battle_area[0].stack_size(),
        2,
        "stack should have 2 sources after manual push"
    );

    // The "caster" src_card is the top card of the permanent.
    let src_card = runner.game.players[0].battle_area[0].top_card().handle();

    let step = CompiledStep::DeDigivolve {
        target: CompiledBindingRef::Named("tgt".into()),
        amount: Some(1),
        stop_at_level: None,
    };
    let mut bindings = Bindings::new();
    bindings.insert_permanent("tgt", handle);

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.players[0].battle_area[0].stack_size(),
        1,
        "de_digivolve(amount=1) should have popped exactly one source"
    );
}
