//! Phase 2g: DSL breeding permanent selections park and resume process tails.

use std::sync::{Arc, Mutex};

use digimon_dsl::compiled::{CompiledPredicate, CompiledStep};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_breeding_select, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::raw_rust::EngineRawRustRegistry;
use digimon_engine::dsl_cards::step::{run_steps, run_steps_with_runtime, RunOutcome, StepRuntime};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardKind;
use digimon_engine::selection::BreedingPermanentSelectionRef;

#[test]
fn dsl_select_breeding_permanent_binds_target() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("KING-DRASIL", "King Drasil"))
        .start();
    let p0 = 0;
    let source = runner.place_on_field(p0, "SRC", Some(0));
    let source_card = runner.top_card(source);
    runner.place_in_breeding(p0, "KING-DRASIL");

    let steps = vec![CompiledStep::SelectOwnBreedingPermanent {
        bind_as: Some("breeding_target".to_string()),
        prompt: "Choose breeding".to_string(),
        filter: CompiledPredicate::default(),
        optional: false,
        then: vec![CompiledStep::GainMemory(1)],
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), p0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };
    assert_eq!(outcome, RunOutcome::Parked);

    runner
        .game
        .resolve_selection(p0, encode_breeding_select(p0).unwrap())
        .expect("pick breeding");

    assert_eq!(runner.game.memory, 1);
}

#[test]
fn select_own_breeding_permanent_binds_selected_ref() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("KING-DRASIL", "King Drasil"))
        .start();
    let p0 = 0;
    let source = runner.place_on_field(p0, "SRC", Some(0));
    let source_card = runner.top_card(source);
    let breeding = runner.place_in_breeding(p0, "KING-DRASIL");
    let expected = BreedingPermanentSelectionRef {
        player: breeding.player,
        card: breeding.card,
    };

    let steps = vec![CompiledStep::SelectOwnBreedingPermanent {
        bind_as: Some("breeding_target".to_string()),
        prompt: "Choose breeding".to_string(),
        filter: CompiledPredicate::default(),
        optional: false,
        then: vec![CompiledStep::RawRust {
            fn_name: "assert_breeding_binding".to_string(),
            consumes: vec!["breeding_target".to_string()],
            binds: vec![],
        }],
    }];

    let seen = Arc::new(Mutex::new(None));
    let seen_slot = Arc::clone(&seen);
    let mut raw = EngineRawRustRegistry::new();
    raw.register_step("assert_breeding_binding", move |_ctx, bindings| {
        *seen_slot.lock().unwrap() = bindings.get_breeding_permanent_ref("breeding_target");
    });
    let runtime = StepRuntime::new(Arc::new(raw));

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), p0);
        run_steps_with_runtime(&steps, &mut ctx, &mut bindings, &runtime)
    };
    assert_eq!(outcome, RunOutcome::Parked);

    runner
        .game
        .resolve_selection(p0, encode_breeding_select(p0).unwrap())
        .expect("pick breeding");

    assert_eq!(*seen.lock().unwrap(), Some(expected));
}

/// RK-G001: the `filter` predicate field gates whether the breeding
/// selection opens. With a non-matching breeding permanent and a
/// `name_is` filter the step must fall through (no `Parked` outcome,
/// no `pending_selection`).
#[test]
fn select_own_breeding_permanent_filter_rejects_non_matching_name() {
    use digimon_dsl::compiled::CompiledPredicate;

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("OTHER-EGG", "Other Egg"))
        .start();
    let p0 = 0;
    let source = runner.place_on_field(p0, "SRC", Some(0));
    let source_card = runner.top_card(source);
    runner.place_in_breeding(p0, "OTHER-EGG");

    let mut filter = CompiledPredicate::default();
    filter.name_is = Some("King Drasil_7D6".to_string());
    let steps = vec![CompiledStep::SelectOwnBreedingPermanent {
        bind_as: Some("breeding_target".to_string()),
        prompt: "Choose [King Drasil_7D6]".to_string(),
        filter,
        optional: false,
        then: vec![CompiledStep::GainMemory(1)],
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), p0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };
    assert_eq!(
        outcome,
        RunOutcome::Synchronous,
        "filter rejects the breeding permanent, the step short-circuits, the surrounding effect resolves"
    );
    assert!(runner.game.pending_selection.is_none());
    assert_eq!(runner.game.memory, 0, "then-tail never runs");
}

/// RK-G001: the `filter` predicate field accepts a matching breeding
/// permanent and opens the selection prompt as usual.
#[test]
fn select_own_breeding_permanent_filter_accepts_matching_name() {
    use digimon_dsl::compiled::CompiledPredicate;

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("KING-DRASIL", "King Drasil_7D6"))
        .start();
    let p0 = 0;
    let source = runner.place_on_field(p0, "SRC", Some(0));
    let source_card = runner.top_card(source);
    runner.place_in_breeding(p0, "KING-DRASIL");

    let mut filter = CompiledPredicate::default();
    filter.name_is = Some("King Drasil_7D6".to_string());
    let steps = vec![CompiledStep::SelectOwnBreedingPermanent {
        bind_as: Some("breeding_target".to_string()),
        prompt: "Choose [King Drasil_7D6]".to_string(),
        filter,
        optional: false,
        then: vec![CompiledStep::GainMemory(1)],
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), p0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };
    assert_eq!(outcome, RunOutcome::Parked);
    runner
        .game
        .resolve_selection(p0, encode_breeding_select(p0).unwrap())
        .expect("pick breeding");
    assert_eq!(runner.game.memory, 1, "then-tail runs after selection");
}

#[test]
fn optional_select_own_breeding_permanent_accept_runs_success_then_outer_tail() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("KING-DRASIL", "King Drasil_7D6"))
        .start();
    let p0 = 0;
    let source = runner.place_on_field(p0, "SRC", Some(0));
    let source_card = runner.top_card(source);
    runner.place_in_breeding(p0, "KING-DRASIL");

    let mut filter = CompiledPredicate::default();
    filter.name_is = Some("King Drasil_7D6".to_string());
    let steps = vec![
        CompiledStep::SelectOwnBreedingPermanent {
            bind_as: Some("breeding_target".to_string()),
            prompt: "You may choose [King Drasil_7D6]".to_string(),
            filter,
            optional: true,
            then: vec![CompiledStep::GainMemory(2)],
        },
        CompiledStep::GainMemory(1),
    ];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), p0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };
    assert_eq!(outcome, RunOutcome::Parked);
    assert!(runner.game.pending_selection.as_ref().unwrap().is_optional);

    runner
        .game
        .resolve_selection(p0, encode_breeding_select(p0).unwrap())
        .expect("accept optional breeding selection");

    assert_eq!(
        runner.game.memory, 3,
        "accepting runs the selection success body and then resumes the outer tail"
    );
}

#[test]
fn optional_select_own_breeding_permanent_decline_skips_success_and_runs_outer_tail() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("KING-DRASIL", "King Drasil_7D6"))
        .start();
    let p0 = 0;
    let source = runner.place_on_field(p0, "SRC", Some(0));
    let source_card = runner.top_card(source);
    runner.place_in_breeding(p0, "KING-DRASIL");

    let mut filter = CompiledPredicate::default();
    filter.name_is = Some("King Drasil_7D6".to_string());
    let steps = vec![
        CompiledStep::SelectOwnBreedingPermanent {
            bind_as: Some("breeding_target".to_string()),
            prompt: "You may choose [King Drasil_7D6]".to_string(),
            filter,
            optional: true,
            then: vec![CompiledStep::GainMemory(2)],
        },
        CompiledStep::GainMemory(1),
    ];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), p0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };
    assert_eq!(outcome, RunOutcome::Parked);
    let mask = build_action_mask(&runner.game, p0);
    assert_eq!(mask[PASS as usize], 1.0, "optional prompt exposes PASS");

    runner
        .game
        .resolve_selection(p0, PASS)
        .expect("decline optional breeding selection");

    assert_eq!(
        runner.game.memory, 1,
        "declining skips the selection success body but still resumes the outer tail"
    );
}

#[test]
fn mandatory_select_own_breeding_permanent_still_hides_pass() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("KING-DRASIL", "King Drasil_7D6"))
        .start();
    let p0 = 0;
    let source = runner.place_on_field(p0, "SRC", Some(0));
    let source_card = runner.top_card(source);
    runner.place_in_breeding(p0, "KING-DRASIL");

    let steps = vec![CompiledStep::SelectOwnBreedingPermanent {
        bind_as: Some("breeding_target".to_string()),
        prompt: "Choose [King Drasil_7D6]".to_string(),
        filter: CompiledPredicate::default(),
        optional: false,
        then: vec![CompiledStep::GainMemory(1)],
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), p0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };
    assert_eq!(outcome, RunOutcome::Parked);
    let mask = build_action_mask(&runner.game, p0);
    assert_eq!(mask[PASS as usize], 0.0, "mandatory prompt hides PASS");
    assert!(runner.game.resolve_selection(p0, PASS).is_err());
}

#[test]
fn optional_select_own_breeding_permanent_no_candidate_runs_outer_tail() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("OTHER", "Other"))
        .start();
    let p0 = 0;
    let source = runner.place_on_field(p0, "SRC", Some(0));
    let source_card = runner.top_card(source);
    runner.place_in_breeding(p0, "OTHER");

    let mut filter = CompiledPredicate::default();
    filter.name_is = Some("King Drasil_7D6".to_string());
    let steps = vec![
        CompiledStep::SelectOwnBreedingPermanent {
            bind_as: Some("breeding_target".to_string()),
            prompt: "You may choose [King Drasil_7D6]".to_string(),
            filter,
            optional: true,
            then: vec![CompiledStep::GainMemory(2)],
        },
        CompiledStep::GainMemory(1),
    ];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), p0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };
    assert_eq!(outcome, RunOutcome::Synchronous);
    assert!(runner.game.pending_selection.is_none());
    assert_eq!(
        runner.game.memory, 1,
        "no matching candidate falls through and runs the surrounding tail"
    );
}

fn compile_steps(step_yaml: &str) -> Vec<CompiledStep> {
    let yaml = format!(
        r#"
card: DSL-BREEDING-MOVE
name: Breeding Move Harness
kind: tamer
color: [black]
cost: 3
effects:
  - when: on_play
    process:
{step_yaml}
"#
    );
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(&yaml).expect("YAML parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("YAML compiles");
    match &compiled.effects[0] {
        digimon_dsl::compiled::CompiledClause::Triggered(t) => t.process.clone(),
        other => panic!("expected triggered clause, got {other:?}"),
    }
}

fn make_digimon(id: &str, level: u8) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card
}

#[test]
fn optional_filtered_move_from_breeding_accept_promotes_lv3_plus() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_digimon("ROOKIE", 3))
        .start();
    let p0 = 0;
    let source = runner.place_on_field(p0, "SRC", Some(0));
    let source_card = runner.top_card(source);
    let breeding = runner.place_in_breeding(p0, "ROOKIE");
    let breeding_card = breeding.card;

    let steps = compile_steps(
        r#"      - select_own_breeding_permanent:
          bind_as: breeding_target
          prompt: "You may move 1 Lv.3+ Digimon from breeding"
          optional: true
          filter: { level_gte: 3 }
          then:
            - move_from_breeding: { of: you }
"#,
    );

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), p0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };
    assert_eq!(outcome, RunOutcome::Parked);
    assert_eq!(
        runner.pending_selection_view().unwrap().valid_action_ids,
        vec![encode_breeding_select(p0).unwrap()]
    );

    runner
        .execute_action(p0, encode_breeding_select(p0).unwrap())
        .expect("accept breeding move");

    assert!(runner.game.player(p0).breeding_area.is_none());
    assert!(
        runner.game.player(p0).battle_area.iter().any(|perm| {
            perm.top_card().handle() == breeding_card
                && perm.top_card().card_id(&runner.game.card_data) == "ROOKIE"
        }),
        "accepting the optional prompt promotes the Lv.3 breeding Digimon"
    );
}

#[test]
fn optional_filtered_move_from_breeding_decline_leaves_breeding_occupied() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_digimon("ROOKIE", 3))
        .start();
    let p0 = 0;
    let source = runner.place_on_field(p0, "SRC", Some(0));
    let source_card = runner.top_card(source);
    let breeding = runner.place_in_breeding(p0, "ROOKIE");
    let breeding_card = breeding.card;

    let steps = compile_steps(
        r#"      - select_own_breeding_permanent:
          bind_as: breeding_target
          prompt: "You may move 1 Lv.3+ Digimon from breeding"
          optional: true
          filter: { level_gte: 3 }
          then:
            - move_from_breeding: { of: you }
      - gain_memory: 1
"#,
    );

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), p0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };
    assert_eq!(outcome, RunOutcome::Parked);
    assert_eq!(
        build_action_mask(&runner.game, p0)[PASS as usize],
        1.0,
        "optional prompt exposes decline"
    );

    runner
        .execute_action(p0, PASS)
        .expect("decline breeding move");

    let breeding_after = runner
        .game
        .player(p0)
        .breeding_area
        .as_ref()
        .expect("declining leaves breeding occupied");
    assert_eq!(breeding_after.top_card().handle(), breeding_card);
    assert_eq!(
        runner.memory(),
        1,
        "decline skips move tail but resumes the surrounding process tail"
    );
}

#[test]
fn optional_filtered_move_from_breeding_skips_prompt_for_level_two() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_digimon("EGG", 2))
        .start();
    let p0 = 0;
    let source = runner.place_on_field(p0, "SRC", Some(0));
    let source_card = runner.top_card(source);
    runner.place_in_breeding(p0, "EGG");

    let steps = compile_steps(
        r#"      - select_own_breeding_permanent:
          bind_as: breeding_target
          prompt: "You may move 1 Lv.3+ Digimon from breeding"
          optional: true
          filter: { level_gte: 3 }
          then:
            - move_from_breeding: { of: you }
      - gain_memory: 1
"#,
    );

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), p0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };
    assert_eq!(outcome, RunOutcome::Synchronous);
    assert!(runner.game.pending_selection.is_none());
    assert!(
        runner.game.player(p0).breeding_area.is_some(),
        "Lv.2 breeding Digimon must not be offered to P-130-style prompt"
    );
    assert_eq!(
        runner.memory(),
        1,
        "no eligible candidate skips the prompt and runs the outer tail"
    );
}
