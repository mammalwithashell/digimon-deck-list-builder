//! Phase 2c — modifier step dispatch (AddDpModifier; AddModifier and GrantKeyword follow).

use digimon_dsl::compiled::{CompiledBindingRef, CompiledStep};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_step;
use digimon_engine::effect_context::EffectContext;

/// `make_test_card` produces a Digimon with base DP = 2000.
/// Adding 3000 should yield 5000 effective DP.
#[test]
fn add_dp_modifier_end_of_turn_raises_effective_dp() {
    let card = make_test_card("T-DP", "T-DP");
    let mut runner = DebugRunner::builder()
        .add_card(card.clone())
        .hand(0, &["T-DP"])
        .build();

    let handle = runner.place_on_field(0, "T-DP", None);
    assert_eq!(runner.game.players[0].battle_area.len(), 1);

    let base_dp = runner.effective_dp(handle).expect("T-DP should have a base DP");

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();

    let step = CompiledStep::AddDpModifier {
        target: CompiledBindingRef::Named("tgt".into()),
        value: 3000,
        expiry: "EndOfTurn".into(),
    };
    let mut bindings = Bindings::new();
    bindings.insert_permanent("tgt", handle);

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    let after_dp = runner.effective_dp(handle).expect("permanent still on field");
    assert_eq!(
        after_dp,
        base_dp + 3000,
        "effective DP should be base ({base_dp}) + 3000 after AddDpModifier"
    );
}

/// An unknown expiry string should no-op: DP stays at base.
#[test]
fn add_dp_modifier_with_bad_expiry_is_noop() {
    let card = make_test_card("T-DPBAD", "T-DPBAD");
    let mut runner = DebugRunner::builder()
        .add_card(card.clone())
        .hand(0, &["T-DPBAD"])
        .build();

    let handle = runner.place_on_field(0, "T-DPBAD", None);
    assert_eq!(runner.game.players[0].battle_area.len(), 1);

    let base_dp = runner.effective_dp(handle).expect("T-DPBAD should have a base DP");

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();

    let step = CompiledStep::AddDpModifier {
        target: CompiledBindingRef::Named("tgt".into()),
        value: 3000,
        expiry: "NotARealExpiry".into(),
    };
    let mut bindings = Bindings::new();
    bindings.insert_permanent("tgt", handle);

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    let after_dp = runner.effective_dp(handle).expect("permanent still on field");
    assert_eq!(
        after_dp,
        base_dp,
        "effective DP should be unchanged after AddDpModifier with bad expiry"
    );
}
