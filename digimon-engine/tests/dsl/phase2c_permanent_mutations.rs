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
        .start();  // advance past Mulligan into turn 1

    // Capture hand size before playing (net-zero round-trip baseline).
    let hand_before = runner.game.players[0].hand.len();

    // Play the card from hand: hand -1, battle_area +1.
    let field_index = runner
        .play(0, 0)
        .expect("play should succeed with enough memory");
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
    assert_eq!(runner.game.players[0].hand.len(), hand_before - 1);

    let handle = runner.perm_handle(0, field_index);
    let src_card = runner.game.players[0].battle_area[field_index].top_card().handle();

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
