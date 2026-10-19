//! Exact event-card identity uses the captured card, never the current slot/top.
use digimon_dsl::compiled::{CompiledClause, CompiledPredicate};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::predicate::{eval_predicate_with_bindings, PredicateSubject};
use digimon_engine::effect_context::EffectReadContext;
use digimon_engine::trigger_context::TriggerContext;

const OBSERVER: &str = r#"
card: EXACT-OBSERVER
name: Exact Observer
kind: digimon
level: 3
color: [green]
cost: 0
dp: 2000
effects:
  - when: on_ally_played
    condition: { event_card_name_is: Groundramon }
    process:
      - gain_memory: 1
"#;

fn predicate() -> CompiledPredicate {
    let spec: digimon_dsl::CardSpec =
        serde_yml::from_str(OBSERVER).expect("exact-name YAML parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("exact-name YAML compiles");
    let CompiledClause::Triggered(clause) = &compiled.effects[0] else {
        panic!("triggered clause")
    };
    clause.condition.clone().expect("condition")
}

#[test]
fn exact_event_name_accepts_any_printing_and_rule_alias_but_rejects_substrings() {
    let pred = predicate();
    let mut alias = make_test_card("ALIAS", "Different Printed Name");
    alias.also_treated_as = vec!["Groundramon".into()];
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SOURCE", "Source"))
        .add_card(make_test_card("FIRST", "Groundramon"))
        .add_card(make_test_card("NEW-PRINTING", "Groundramon"))
        .add_card(make_test_card("VARIANT", "Groundramon Variant"))
        .add_card(alias)
        .hand(0, &["SOURCE", "FIRST", "NEW-PRINTING", "VARIANT", "ALIAS"])
        .build();
    let source = runner.game.players[0].hand[0].handle();
    for (index, expected) in [(1, true), (2, true), (3, false), (4, true)] {
        runner.game.current_trigger_context = Some({ let mut __fixture_trigger_context = TriggerContext::default(); __fixture_trigger_context.event_card = Some(runner.game.players[0].hand[index].handle()); __fixture_trigger_context });
        let ctx = EffectReadContext::new(&runner.game, source, None, 0);
        assert_eq!(
            eval_predicate_with_bindings(&pred, &ctx, PredicateSubject::None, None),
            expected
        );
    }
    runner.game.current_trigger_context = None;
    let ctx = EffectReadContext::new(&runner.game, source, None, 0);
    assert!(!eval_predicate_with_bindings(
        &pred,
        &ctx,
        PredicateSubject::None,
        None
    ));
}

#[test]
fn captured_event_name_survives_new_top_and_departed_permanent() {
    let pred = predicate();
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SOURCE", "Source"))
        .add_card(make_test_card("EVENT", "Groundramon"))
        .add_card(make_test_card("EVOLVED", "Breakdramon"))
        .hand(0, &["SOURCE", "EVOLVED"])
        .build();
    let source = runner.game.players[0].hand[0].handle();
    let event_perm = runner.place_on_field(0, "EVENT", None);
    let event_card = runner.game.players[0].battle_area[event_perm.index as usize]
        .top_card()
        .handle();
    runner.game.current_trigger_context = Some({ let mut __fixture_trigger_context = TriggerContext::default(); __fixture_trigger_context.event_card = Some(event_card); __fixture_trigger_context.event_permanent = Some(event_perm); __fixture_trigger_context });
    // Ordinary evolution changes only the visible top; captured identity remains.
    let evolved = runner.game.players[0].hand.pop().unwrap();
    runner.game.players[0].battle_area[event_perm.index as usize]
        .card_sources
        .push(evolved);
    let ctx = EffectReadContext::new(&runner.game, source, None, 0);
    assert!(eval_predicate_with_bindings(
        &pred,
        &ctx,
        PredicateSubject::None,
        None
    ));
    let departed = runner.game.players[0]
        .battle_area
        .remove(event_perm.index as usize);
    runner.game.players[0].trash.extend(departed.card_sources);
    runner.place_on_field(0, "EVOLVED", None);
    let ctx = EffectReadContext::new(&runner.game, source, None, 0);
    assert!(eval_predicate_with_bindings(
        &pred,
        &ctx,
        PredicateSubject::None,
        None
    ));
}

#[test]
fn ally_play_observer_uses_exact_event_name() {
    for (name, expected_gain) in [("Groundramon", 1), ("Groundramon Variant", 0)] {
        let mut card = make_test_card("PLAYED", name);
        card.play_cost = 0;
        let mut runner = DebugRunner::builder()
            .from_dsl_yaml(OBSERVER)
            .expect("observer YAML")
            .add_card(card)
            .hand(0, &["PLAYED"])
            .memory(5)
            .start();
        runner.place_on_field(0, "EXACT-OBSERVER", None);
        assert_eq!(runner.memory(), 5, "started fixture applies initial memory");
        assert!(runner.play(0, 0).is_some());
        assert_eq!(runner.memory(), 5 + expected_gain);
    }
}
