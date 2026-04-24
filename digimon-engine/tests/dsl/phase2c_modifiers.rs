//! Phase 2c — modifier step dispatch (AddDpModifier; AddModifier and GrantKeyword follow).

use digimon_dsl::compiled::{CompiledBindingRef, CompiledModifierTarget, CompiledStep};
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

/// AddModifier with a known modifier type (CannotAttack) should register the
/// modifier on the target permanent.
#[test]
fn add_modifier_cannot_attack_blocks_attack_flag() {
    let card = make_test_card("T-MOD", "T-MOD");
    let mut runner = DebugRunner::builder()
        .add_card(card.clone())
        .hand(0, &["T-MOD"])
        .build();

    let handle = runner.place_on_field(0, "T-MOD", None);
    assert_eq!(runner.game.players[0].battle_area.len(), 1);

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();

    let step = CompiledStep::AddModifier {
        target: CompiledModifierTarget::Binding(CompiledBindingRef::Named("tgt".into())),
        modifier: "CannotAttack".into(),
        value: 0,
        expiry: "EndOfTurn".into(),
    };
    let mut bindings = digimon_engine::dsl_cards::bindings::Bindings::new();
    bindings.insert_permanent("tgt", handle);

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    assert!(
        runner.game.modifiers.has(handle, digimon_engine::enums::ModifierType::CannotAttack),
        "CannotAttack modifier should be registered on the target permanent"
    );
}

/// AddModifier with an unknown modifier string should no-op (no panic, no modifier registered).
#[test]
fn add_modifier_unknown_modifier_string_is_noop() {
    let card = make_test_card("T-MODUNK", "T-MODUNK");
    let mut runner = DebugRunner::builder()
        .add_card(card.clone())
        .hand(0, &["T-MODUNK"])
        .build();

    let handle = runner.place_on_field(0, "T-MODUNK", None);
    assert_eq!(runner.game.players[0].battle_area.len(), 1);

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();

    let step = CompiledStep::AddModifier {
        target: CompiledModifierTarget::Binding(CompiledBindingRef::Named("tgt".into())),
        modifier: "NotAModifier".into(),
        value: 0,
        expiry: "EndOfTurn".into(),
    };
    let mut bindings = digimon_engine::dsl_cards::bindings::Bindings::new();
    bindings.insert_permanent("tgt", handle);

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    // No modifier should be registered — the registry should have nothing for
    // the target since we started clean and dispatched an unknown modifier name.
    assert!(
        runner.game.modifiers.get(handle, digimon_engine::enums::ModifierType::CannotAttack).is_empty(),
        "no modifier should be registered for an unknown modifier string"
    );
}

/// AddModifier with a Filter target is deferred to Phase 2d and should no-op
/// for now: no modifier is registered on the target permanent.
#[test]
fn add_modifier_filter_target_is_noop_until_2d() {
    use digimon_dsl::compiled::{CompiledModifierTarget, CompiledPredicate};

    let card = make_test_card("T-MODFLT", "T-MODFLT");
    let mut runner = DebugRunner::builder()
        .add_card(card.clone())
        .hand(0, &["T-MODFLT"])
        .build();

    let handle = runner.place_on_field(0, "T-MODFLT", None);
    assert_eq!(runner.game.players[0].battle_area.len(), 1);

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();

    let step = CompiledStep::AddModifier {
        target: CompiledModifierTarget::Filter(CompiledPredicate::default()),
        modifier: "CannotAttack".into(),
        value: 0,
        expiry: "EndOfTurn".into(),
    };
    let mut bindings = digimon_engine::dsl_cards::bindings::Bindings::new();

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    // Filter branch is deferred to Phase 2d — no modifier should be registered.
    assert!(
        runner.game.modifiers.get(handle, digimon_engine::enums::ModifierType::CannotAttack).is_empty(),
        "Filter target branch should be a no-op until Phase 2d"
    );
}
