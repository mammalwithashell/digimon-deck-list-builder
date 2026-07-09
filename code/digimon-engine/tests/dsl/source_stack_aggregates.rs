use digimon_dsl::compiled::{
    CompiledCountBound, CompiledFormula, CompiledModifierValue, CompiledPerSelector,
    CompiledStep,
};
use digimon_dsl::{compile::compile, spec::CardSpec};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::formula_eval;
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::{run_steps, RunOutcome};
use digimon_engine::effect_context::{EffectContext, SourceSelectionRef};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;

fn compile_steps(yaml: &str) -> Vec<CompiledStep> {
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("yaml compiles");
    let digimon_dsl::compiled::CompiledClause::Triggered(clause) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    clause.process.clone()
}

fn digimon_with_colors(id: &str, colors: &[CardColor]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = colors.to_vec();
    card
}

#[test]
fn source_stack_aggregate_formula_reads_source_levels() {
    let mut runner = DebugRunner::builder()
        .add_card({
            let mut card = make_test_card("SRC-4A", "Source 4A");
            card.level = Some(4);
            card
        })
        .add_card({
            let mut card = make_test_card("SRC-4B", "Source 4B");
            card.level = Some(4);
            card
        })
        .add_card({
            let mut card = make_test_card("SRC-5", "Source 5");
            card.level = Some(5);
            card
        })
        .add_card({
            let mut card = make_test_card("TOP", "Top");
            card.level = Some(6);
            card
        })
        .start();

    let target = runner.place_stack(0, &["SRC-4A", "SRC-4B", "SRC-5", "TOP"]);
    let source_card = runner.top_card(target);
    let ctx = EffectContext::new(&mut runner.game, source_card, Some(target), 0);
    let formula = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::SameLevelPairsInSources,
        delta: 1,
    };

    assert_eq!(formula_eval::evaluate(&formula, &ctx, target), 1);
}

#[test]
fn self_same_level_source_pairs_gte_predicate_reads_source_stack_levels() {
    let yaml = r#"
card: TEST-SAME-LEVEL-SOURCE-PAIR-GATE
name: Same Level Source Pair Gate
kind: digimon
level: 5
color: [blue]
cost: 7
dp: 7000
effects:
  - kind: aura
    target: {}
    active_when:
      self_same_level_source_pairs_gte: 1
    dp_modifier: 3000
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card({
            let mut card = make_test_card("SRC-4A", "Source 4A");
            card.level = Some(4);
            card.card_kind = CardKind::Digimon;
            card
        })
        .add_card({
            let mut card = make_test_card("SRC-4B", "Source 4B");
            card.level = Some(4);
            card.card_kind = CardKind::Digimon;
            card
        })
        .add_card({
            let mut card = make_test_card("SRC-5", "Source 5");
            card.level = Some(5);
            card.card_kind = CardKind::Digimon;
            card
        })
        .start();

    let with_pair = runner.place_stack(
        0,
        &[
            "SRC-4A",
            "SRC-4B",
            "SRC-5",
            "TEST-SAME-LEVEL-SOURCE-PAIR-GATE",
        ],
    );
    assert_eq!(
        runner.game.effective_dp(with_pair),
        Some(10000),
        "one same-level source pair should satisfy the predicate"
    );

    let without_pair = runner.place_stack(
        0,
        &[
            "SRC-4A",
            "SRC-5",
            "TEST-SAME-LEVEL-SOURCE-PAIR-GATE",
        ],
    );
    assert_eq!(
        runner.game.effective_dp(without_pair),
        Some(7000),
        "different-level sources should not satisfy the predicate"
    );
}

#[test]
fn source_color_count_formula_reads_distinct_live_source_stack_colors() {
    let yaml = r#"
card: TEST-SOURCE-COLOR-COUNT
name: Source Color Count
kind: digimon
level: 6
color: [red]
cost: 11
dp: 11000
effects:
  - when: when_digivolving
    process:
      - add_dp_modifier:
          target: source
          value:
            formula:
              source_color_count: {}
          expiry: end_of_turn
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("source_color_count formula parses");
    let compiled = compile(&spec).expect("source_color_count formula compiles");
    assert!(
        compiled.effects.len() == 1,
        "card should compile with the source_color_count formula"
    );

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(digimon_with_colors("RED-SOURCE", &[CardColor::Red]))
        .add_card(digimon_with_colors(
            "RED-YELLOW-SOURCE",
            &[CardColor::Red, CardColor::Yellow],
        ))
        .add_card(digimon_with_colors("BLUE-TOP", &[CardColor::Blue]))
        .memory(10)
        .start();
    let stack = runner.place_stack(
        0,
        &[
            "RED-SOURCE",
            "RED-YELLOW-SOURCE",
            "BLUE-TOP",
            "TEST-SOURCE-COLOR-COUNT",
        ],
    );

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(stack),
    );
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.game.effective_dp(stack),
        Some(11003),
        "source_color_count counts distinct source-card colors once, including multi-color sources"
    );
}

#[test]
fn source_color_count_returns_zero_without_source_cards() {
    let yaml = r#"
card: TEST-SOURCE-COLOR-EMPTY
name: Source Color Empty
kind: digimon
level: 6
color: [red]
cost: 11
dp: 11000
effects:
  - when: when_digivolving
    process:
      - add_dp_modifier:
          target: source
          value:
            formula:
              source_color_count: {}
          expiry: end_of_turn
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .memory(10)
        .start();
    let stack = runner.place_on_field(0, "TEST-SOURCE-COLOR-EMPTY", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(stack),
    );
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.game.effective_dp(stack),
        Some(11000),
        "source_color_count ignores the top card and returns zero with no source cards"
    );
}

#[test]
fn source_stack_count_formula_counts_matching_source_cards() {
    let yaml = r#"
card: TEST-SOURCE-STACK-COUNT
name: Source Stack Count
kind: digimon
level: 7
color: [yellow, green]
cost: 15
dp: 15000
effects:
  - when: when_digivolving
    process:
      - gain_memory_fn:
          formula:
            source_stack_count:
              target: source
              filter: { level_eq: 6 }
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("source_stack_count formula parses");
    let compiled = compile(&spec).expect("source_stack_count formula compiles");
    assert_eq!(compiled.effects.len(), 1);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card({
            let mut card = make_test_card("SRC-6A", "Source 6A");
            card.card_kind = CardKind::Digimon;
            card.level = Some(6);
            card
        })
        .add_card({
            let mut card = make_test_card("SRC-6B", "Source 6B");
            card.card_kind = CardKind::Digimon;
            card.level = Some(6);
            card
        })
        .add_card({
            let mut card = make_test_card("SRC-5", "Source 5");
            card.card_kind = CardKind::Digimon;
            card.level = Some(5);
            card
        })
        .memory(0)
        .start();
    let stack = runner.place_stack(0, &["SRC-6A", "SRC-5", "SRC-6B", "TEST-SOURCE-STACK-COUNT"]);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(stack),
    );
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.game.memory, 2,
        "source_stack_count counts only matching source cards beneath the top"
    );
}

#[test]
fn source_stack_count_formula_lowers_inside_count_capped_multi_bound() {
    let steps = compile_steps(
        r#"
card: TEST-SOURCE-COUNT-BOUND
name: Source Count Bound
kind: digimon
level: 7
color: [yellow]
cost: 15
dp: 15000
effects:
  - when: when_digivolving
    process:
      - select_count_capped_multi:
          of: opponent
          zone: battle_area
          max:
            formula:
              source_stack_count:
                target: source
                filter: { level_eq: 6 }
          filter: { kind: digimon }
          bind_as: picked
          prompt: Pick per source count
"#,
    );

    let CompiledStep::SelectCountCappedMulti { max, .. } = &steps[0] else {
        panic!("expected select_count_capped_multi");
    };
    let CompiledCountBound::Formula(CompiledFormula::SourceStackCount { target, filter }) = max
    else {
        panic!("expected source_stack_count formula bound");
    };

    assert_eq!(target, "source");
    assert!(
        filter
            .as_ref()
            .is_some_and(|predicate| predicate.level_eq == Some(6)),
        "source_stack_count filter should lower through compile"
    );
}

#[test]
fn source_color_count_per_selector_lowers_for_modifier_formulas() {
    let steps = compile_steps(
        r#"
card: TEST-SOURCE-COLOR-PER
name: Source Color Per
kind: digimon
level: 6
color: [red]
cost: 11
dp: 11000
effects:
  - when: when_digivolving
    process:
      - add_dp_modifier:
          target: source
          value:
            formula:
              base: 0
              per: source_color_count
              delta: -1000
          expiry: end_of_turn
"#,
    );

    let CompiledStep::AddDpModifier { value, .. } = &steps[0] else {
        panic!("expected add_dp_modifier");
    };
    let CompiledModifierValue::Formula(CompiledFormula::BasePerDelta { base, per, delta }) = value
    else {
        panic!("expected base/per/delta formula");
    };

    assert_eq!(
        (*base, per, *delta),
        (0, &CompiledPerSelector::SourceColorCount, -1000)
    );
}

#[test]
fn source_stack_steps_compile() {
    let steps = compile_steps(
        r#"
card: TEST-SOURCE-STACK
name: Source Stack Test
kind: digimon
color: [green]
level: 6
cost: 11
dp: 11000
effects:
  - when: main_from_hand
    process:
      - select_opponent_permanent:
          filter: { kind: digimon }
          bind_as: target
          prompt: Choose target
      - trash_all_sources: { target: target }
"#,
    );

    assert!(matches!(steps[1], CompiledStep::TrashAllSources { .. }));
}

#[test]
fn play_selected_sources_free_step_compiles() {
    let steps = compile_steps(
        r#"
card: TEST-PLAY-SOURCES
name: Play Sources Test
kind: option
color: [green]
cost: 3
effects:
  - when: main_from_hand
    process:
      - select_own_sources:
          min: 1
          max: 1
          bind_as: chosen_sources
          prompt: Choose source
          then:
            - play_selected_sources_free: { source_refs: chosen_sources }
"#,
    );

    let CompiledStep::SelectOwnSources { then, .. } = &steps[0] else {
        panic!("expected select_own_sources");
    };
    assert!(matches!(
        then[0],
        CompiledStep::PlaySelectedSourcesFree { .. }
    ));
}

#[test]
fn source_stack_steps_run_against_bound_refs() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("TOP", "Top"))
        .add_card(make_test_card("EFFECT", "Effect"))
        .start();
    let stack = runner.place_stack(0, &["SRC", "TOP"]);
    let source = runner.game.players[0].battle_area[stack.index as usize].card_sources[0].handle();
    let effect_perm = runner.place_on_field(0, "EFFECT", None);
    let effect_card = runner.top_card(effect_perm);

    let steps = vec![CompiledStep::PlaySelectedSourcesFree {
        source_refs: "chosen".to_string(),
    }];
    let mut bindings = Bindings::new();
    bindings.insert_source_refs(
        "chosen",
        vec![SourceSelectionRef {
            permanent: stack,
            field_index: stack.index,
            source_index: 0,
            card: source,
        }],
    );

    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, effect_card, Some(effect_perm), 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };

    assert_eq!(outcome, RunOutcome::Synchronous);
    assert!(runner.game.players[0]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().handle() == source));
}
