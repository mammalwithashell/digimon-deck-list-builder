//! Phase 2c — modifier step dispatch (AddDpModifier, AddModifier, GrantKeyword).

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

/// AddModifier with a Filter target — Phase 2d Task 8 implementation: applies
/// the modifier to every battle-area permanent matching the filter. Single-
/// permanent regression check; multi-permanent coverage lives in
/// `phase2d_add_modifier_filter.rs`.
#[test]
fn add_modifier_filter_target_applies_to_match() {
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

    // Filter branch now applies the modifier to every battle-area match.
    assert!(
        !runner.game.modifiers.get(handle, digimon_engine::enums::ModifierType::CannotAttack).is_empty(),
        "Filter target branch should register CannotAttack on the matching permanent"
    );
}

/// GrantKeyword with a known keyword (Blocker) should cause `game.has_keyword`
/// to return true for the target permanent.
#[test]
fn grant_keyword_blocker_is_queryable() {
    let card = make_test_card("T-KW", "T-KW");
    let mut runner = DebugRunner::builder()
        .add_card(card.clone())
        .hand(0, &["T-KW"])
        .build();

    let handle = runner.place_on_field(0, "T-KW", None);
    assert_eq!(runner.game.players[0].battle_area.len(), 1);

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();

    let step = CompiledStep::GrantKeyword {
        target: CompiledBindingRef::Named("tgt".into()),
        keyword: "Blocker".into(),
        expiry: "EndOfTurn".into(),
        value: None,
    };
    let mut bindings = Bindings::new();
    bindings.insert_permanent("tgt", handle);

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    assert!(
        runner.game.has_keyword(handle, digimon_engine::enums::Keyword::Blocker),
        "GrantKeyword Blocker should make has_keyword return true for the target"
    );
}

/// GrantKeyword with an unknown keyword name should no-op: no panic, and Blocker
/// is not present on the target.
#[test]
fn grant_keyword_unknown_name_is_noop() {
    let card = make_test_card("T-KWUNK", "T-KWUNK");
    let mut runner = DebugRunner::builder()
        .add_card(card.clone())
        .hand(0, &["T-KWUNK"])
        .build();

    let handle = runner.place_on_field(0, "T-KWUNK", None);
    assert_eq!(runner.game.players[0].battle_area.len(), 1);

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();

    let step = CompiledStep::GrantKeyword {
        target: CompiledBindingRef::Named("tgt".into()),
        keyword: "NotAKeyword".into(),
        expiry: "EndOfTurn".into(),
        value: None,
    };
    let mut bindings = Bindings::new();
    bindings.insert_permanent("tgt", handle);

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    // No keyword granted — Blocker should not be present.
    assert!(
        !runner.game.has_keyword(handle, digimon_engine::enums::Keyword::Blocker),
        "unknown keyword name should be a no-op: Blocker not granted"
    );
}
