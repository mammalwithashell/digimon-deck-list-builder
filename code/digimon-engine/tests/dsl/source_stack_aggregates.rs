use digimon_dsl::compiled::{CompiledFormula, CompiledPerSelector, CompiledStep};
use digimon_dsl::{compile::compile, spec::CardSpec};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::formula_eval;
use digimon_engine::dsl_cards::step::{run_steps, RunOutcome};
use digimon_engine::effect_context::{EffectContext, SourceSelectionRef};

fn compile_steps(yaml: &str) -> Vec<CompiledStep> {
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("yaml compiles");
    let digimon_dsl::compiled::CompiledClause::Triggered(clause) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    clause.process.clone()
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
