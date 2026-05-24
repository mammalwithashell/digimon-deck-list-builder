//! Phase 2c — permanent mutation step dispatch.

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledCardKind, CompiledClause, CompiledFormula, CompiledPerSelector,
    CompiledPlayerRef, CompiledPredicate, CompiledStep, CompiledZone,
};
use digimon_dsl::formula::{CardCountInZoneSpec, FormulaSpec, PerSelector};
use digimon_dsl::predicate::Zone;
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
        amount_fn: None,
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

#[test]
fn de_digivolve_amount_fn_compiles_from_yaml() {
    let yaml = r#"
card: DSL-DEDE-FN
name: Formula De-Digivolve
kind: digimon
color: [yellow]
level: 6
cost: 11
dp: 11000
effects:
  - when: when_digivolving
    process:
      - select_opponent_permanent:
          bind_as: target
          filter: { kind: digimon }
          prompt: Pick target
      - de_digivolve:
          target: target
          amount_fn:
            base: 0
            per:
              card_count_in_zone:
                zone: battle_area
                of: you
                filter: { kind: digimon }
            delta: 1
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("amount_fn yaml parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("amount_fn yaml compiles");
    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let CompiledStep::DeDigivolve {
        amount: None,
        amount_fn: Some(CompiledFormula::BasePerDelta { per, .. }),
        stop_at_level,
        ..
    } = &triggered.process[1]
    else {
        panic!("expected de_digivolve amount_fn step");
    };
    assert_eq!(
        *stop_at_level,
        Some(3),
        "DSL de_digivolve should default to the standard level-3 floor"
    );
    assert!(matches!(
        per,
        CompiledPerSelector::FilteredCardCountInZoneScoped {
            zone: CompiledZone::BattleArea,
            of: CompiledPlayerRef::You,
            ..
        }
    ));
}

#[test]
fn de_digivolve_amount_fn_uses_own_digimon_count_and_caps_to_sources() {
    let mut target_base = make_test_card("OPP-BASE", "OPP-BASE");
    target_base.card_kind = digimon_engine::enums::CardKind::Digimon;
    let mut target_mid_a = make_test_card("OPP-MID-A", "OPP-MID-A");
    target_mid_a.card_kind = digimon_engine::enums::CardKind::Digimon;
    let mut target_mid_b = make_test_card("OPP-MID-B", "OPP-MID-B");
    target_mid_b.card_kind = digimon_engine::enums::CardKind::Digimon;
    let mut target_top = make_test_card("OPP-TOP", "OPP-TOP");
    target_top.card_kind = digimon_engine::enums::CardKind::Digimon;
    let mut src = make_test_card("SRC", "SRC");
    src.card_kind = digimon_engine::enums::CardKind::Digimon;
    let mut ally_a = make_test_card("ALLY-A", "ALLY-A");
    ally_a.card_kind = digimon_engine::enums::CardKind::Digimon;
    let mut ally_b = make_test_card("ALLY-B", "ALLY-B");
    ally_b.card_kind = digimon_engine::enums::CardKind::Digimon;

    let mut runner = DebugRunner::builder()
        .add_card(target_base)
        .add_card(target_mid_a)
        .add_card(target_mid_b)
        .add_card(target_top)
        .add_card(src)
        .add_card(ally_a)
        .add_card(ally_b)
        .build();
    let target = runner.place_stack(1, &["OPP-BASE", "OPP-MID-A", "OPP-MID-B", "OPP-TOP"]);
    let source = runner.place_on_field(0, "SRC", None);
    runner.place_on_field(0, "ALLY-A", None);
    runner.place_on_field(0, "ALLY-B", None);
    let src_card = runner.game.players[0].battle_area[source.index as usize]
        .top_card()
        .handle();
    let step = CompiledStep::DeDigivolve {
        target: CompiledBindingRef::Named("target".into()),
        amount: None,
        amount_fn: Some(CompiledFormula::BasePerDelta {
            base: 0,
            per: CompiledPerSelector::FilteredCardCountInZoneScoped {
                zone: CompiledZone::BattleArea,
                of: CompiledPlayerRef::You,
                filter: Box::new(CompiledPredicate {
                    kind: Some(CompiledCardKind::Digimon),
                    ..Default::default()
                }),
            },
            delta: 1,
        }),
        stop_at_level: None,
    };
    let mut bindings = Bindings::new();
    bindings.insert_permanent("target", target);

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, Some(source), 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.players[1].battle_area[target.index as usize].stack_size(),
        1,
        "own three Digimon should produce amount 3 and peel all three materials"
    );

    let serializable = FormulaSpec::BasePerDelta {
        base: 0,
        per: PerSelector::CardCountInZone(CardCountInZoneSpec {
            zone: Zone::BattleArea,
            of: digimon_dsl::common::PlayerRef::You,
            filter: None,
        }),
        bucket: None,
        delta: 1,
    };
    assert!(serde_yml::to_string(&serializable)
        .expect("formula serializes")
        .contains("card_count_in_zone"));
}
