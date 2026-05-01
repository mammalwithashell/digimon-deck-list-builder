//! Phase 2g: source selections can bind stable cross-permanent source refs and
//! consume them from later DSL steps.

use digimon_dsl::compiled::CompiledStep;
use digimon_engine::action::space::encode_source_select;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::{run_steps, RunOutcome};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::selection::SelectionKind;

#[test]
fn select_own_sources_binds_source_refs_for_trashing() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC-A", "Source A"))
        .add_card(make_test_card("TOP-A", "Top A"))
        .add_card(make_test_card("SRC-B", "Source B"))
        .add_card(make_test_card("TOP-B", "Top B"))
        .add_card(make_test_card("EFFECT", "Effect"))
        .hand(0, &["EFFECT"])
        .start();

    let stack_a = runner.place_stack(0, &["SRC-A", "TOP-A"]);
    let stack_b = runner.place_stack(0, &["SRC-B", "TOP-B"]);
    let source_card = runner.game.players[0].hand[0].handle();

    let steps = vec![CompiledStep::SelectOwnSources {
        min: 2,
        max: 2,
        bind_as: Some("chosen_sources".to_string()),
        prompt: "Choose two sources".to_string(),
        then: vec![CompiledStep::TrashSelectedSources {
            source_refs: "chosen_sources".to_string(),
        }],
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };

    assert_eq!(outcome, RunOutcome::Parked);
    assert_eq!(
        runner.game.pending_selection.as_ref().map(|s| s.kind),
        Some(SelectionKind::SourceMulti {
            min: 2,
            max: 2,
            picked: 0,
        })
    );

    let action_a = encode_source_select(stack_a.index as u16, 0).expect("stack A source action");
    let action_b = encode_source_select(stack_b.index as u16, 0).expect("stack B source action");
    runner.execute_action(0, action_a).expect("pick source A");
    runner.execute_action(0, action_b).expect("pick source B");

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(
        runner.game.players[0].battle_area[stack_a.index as usize]
            .card_sources
            .len(),
        1
    );
    assert_eq!(
        runner.game.players[0].battle_area[stack_b.index as usize]
            .card_sources
            .len(),
        1
    );
    assert_eq!(runner.game.players[0].trash.len(), 2);
}

#[test]
fn empty_select_own_sources_runs_outer_tail_synchronously() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("EFFECT", "Effect"))
        .hand(0, &["EFFECT"])
        .start();
    let source_card = runner.game.players[0].hand[0].handle();

    let steps = vec![
        CompiledStep::SelectOwnSources {
            min: 1,
            max: 1,
            bind_as: Some("chosen_sources".to_string()),
            prompt: "Choose source".to_string(),
            then: vec![CompiledStep::GainMemory(7)],
        },
        CompiledStep::GainMemory(3),
    ];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };

    assert_eq!(outcome, RunOutcome::Synchronous);
    assert!(runner.game.pending_selection.is_none());
    assert_eq!(runner.game.memory, 3);
}
