//! Phase 2e Task 2: SelectEffectChoice installs a parking selection,
//! its callback writes the chosen branch index into Bindings as
//! `BindingValue::Literal`, and the post-selection step runs.

use digimon_dsl::compiled::{
    CompiledBindingCompare, CompiledBindingRef, CompiledFormula, CompiledModifierValue,
    CompiledPredicate, CompiledStep,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

fn resolve_pending_index(runner: &mut DebugRunner, index: usize) {
    let (action_id, selecting_player) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("pending selection");
        (pending.valid_action_ids[index], pending.selecting_player)
    };
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve selection");
}

#[test]
fn select_effect_choice_binds_picked_index() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .hand(0, &["SRC"])
        .build();

    let src_card = runner.game.players[0].hand[0].handle();

    let steps = vec![
        CompiledStep::SelectEffectChoice {
            labels: vec!["A".to_string(), "B".to_string()],
            legal_when: None,
            bind_as: Some("branch".to_string()),
            prompt: "Pick A or B".to_string(),
            prompt_key: None,
        },
        // Sentinel: gain memory so the test can confirm the post-select
        // tail ran. Branch-specific behavior is exercised by the end-to-end
        // test in Task 9 once the equals predicate lands; until then we
        // just confirm the callback fires and the tail executes.
        CompiledStep::GainMemory(1),
    ];

    let memory_before = runner.game.memory;

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // SelectEffectChoice parked — the GainMemory tail should not have run yet.
    assert!(
        runner.game.pending_selection.is_some(),
        "select_effect_choice must install a pending selection"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "tail must not run before the choice is resolved"
    );

    // Resolve by picking branch 1 ("B").
    let (action_id, selecting_player) = {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        // labels[1] is the second action_id in the list; the engine
        // guarantees `valid_action_ids[i]` corresponds to label index i.
        (pending.valid_action_ids[1], pending.selecting_player)
    };
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve");

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "tail must run after resolution"
    );
}

fn mode_is(n: i64) -> CompiledPredicate {
    CompiledPredicate {
        equals: Some(vec![
            CompiledBindingCompare::Binding("mode".to_string()),
            CompiledBindingCompare::Literal(n),
        ]),
        ..CompiledPredicate::default()
    }
}

#[test]
fn repeat_effect_choice_repeats_count_and_resumes_after_nested_selection() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("ALLY", "ALLY"))
        .hand(0, &["SRC"])
        .memory(5)
        .start();

    let ally = runner.place_on_field(0, "ALLY", None);
    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let steps = vec![CompiledStep::RepeatEffectChoice {
        count: CompiledFormula::Literal(2),
        labels: vec!["Buff ally".to_string(), "Gain memory".to_string()],
        bind_as: Some("mode".to_string()),
        prompt: "Choose an effect to activate".to_string(),
        prompt_key: None,
        body: vec![
            CompiledStep::If {
                condition: mode_is(0),
                then: vec![
                    CompiledStep::SelectOwnPermanent {
                        filter: CompiledPredicate {
                            name_is: Some("ALLY".to_string()),
                            ..CompiledPredicate::default()
                        },
                        bind_as: Some("target".to_string()),
                        selector: None,
                        prompt: "Choose ALLY".to_string(),
                        prompt_key: None,
                        optional: false,
                        continue_on_decline: false,
                        then: vec![],
                    },
                    CompiledStep::AddDpModifier {
                        target: CompiledBindingRef::Named("target".to_string()),
                        value: CompiledModifierValue::Literal(1000),
                        expiry: "end_of_turn".to_string(),
                    },
                ],
                else_branch: vec![],
            },
            CompiledStep::If {
                condition: mode_is(1),
                then: vec![CompiledStep::GainMemory(3)],
                else_branch: vec![],
            },
        ],
    }];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    assert!(
        runner.game.pending_selection.is_some(),
        "first repeated effect-choice prompt should park"
    );

    resolve_pending_index(&mut runner, 0);
    assert!(
        runner.game.pending_selection.is_some(),
        "first choice body should park on its nested target prompt"
    );

    resolve_pending_index(&mut runner, 0);
    assert_eq!(
        runner.game.effective_dp(ally),
        Some(3000),
        "first mode should resolve before the next repeated prompt"
    );
    assert!(
        runner.game.pending_selection.is_some(),
        "second repeated effect-choice prompt should be installed"
    );

    resolve_pending_index(&mut runner, 1);
    assert!(
        runner.game.pending_selection.is_none(),
        "repeat loop should finish after the second choice"
    );
    assert_eq!(
        runner.game.memory,
        memory_before + 3,
        "second mode should resolve after the repeated prompt"
    );
}

// ---------------------------------------------------------------------------
// G-DSL-EFFECT-CHOICE-BRANCH-LEGALITY — `legal_when` per-branch gating.
// ---------------------------------------------------------------------------

fn always(holds: bool) -> CompiledPredicate {
    CompiledPredicate {
        equals: Some(vec![
            CompiledBindingCompare::Literal(1),
            CompiledBindingCompare::Literal(if holds { 1 } else { 0 }),
        ]),
        ..CompiledPredicate::default()
    }
}

fn choice_steps(legal: Vec<bool>, labels: usize) -> Vec<CompiledStep> {
    let mut steps = vec![CompiledStep::SelectEffectChoice {
        labels: (0..labels).map(|i| format!("L{i}")).collect(),
        legal_when: Some(legal.into_iter().map(always).collect()),
        bind_as: Some("branch".to_string()),
        prompt: "Pick".to_string(),
        prompt_key: None,
    }];
    // Branch k gains (k+1) memory, so the chosen branch is observable.
    for k in 0..labels as i64 {
        steps.push(CompiledStep::If {
            condition: CompiledPredicate {
                equals: Some(vec![
                    CompiledBindingCompare::Binding("branch".to_string()),
                    CompiledBindingCompare::Literal(k),
                ]),
                ..CompiledPredicate::default()
            },
            then: vec![CompiledStep::GainMemory((k + 1) as i32)],
            else_branch: vec![],
        });
    }
    // Unconditional sentinel tail.
    // (Memory caps at 10, so keep branch + tail sums below it.)
    steps.push(CompiledStep::GainMemory(4));
    steps
}

fn run_choice(steps: &[CompiledStep]) -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .hand(0, &["SRC"])
        .build();
    runner.game.memory = 0;
    let src_card = runner.game.players[0].hand[0].handle();
    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(steps, &mut ctx, &mut bindings);
    }
    runner
}

#[test]
fn legal_when_masks_illegal_branches_out_of_the_prompt() {
    let mut runner = run_choice(&choice_steps(vec![true, false, true], 3));
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("two legal branches still prompt");
    assert_eq!(pending.valid_action_ids.len(), 2, "branch 1 is masked");
    assert_eq!(pending.effect_choices.as_ref().unwrap().len(), 2);
    assert_eq!(pending.effect_choices.as_ref().unwrap()[1].label, "L2");
    // The second offered option is label index 2 → gains 3, then the tail 4.
    resolve_pending_index(&mut runner, 1);
    assert!(runner.game.pending_selection.is_none());
    assert_eq!(runner.game.memory, 7);
}

#[test]
fn legal_when_single_legal_branch_runs_without_a_prompt() {
    let runner = run_choice(&choice_steps(vec![false, true], 2));
    assert!(
        runner.game.pending_selection.is_none(),
        "a one-option effect choice is not a choice — no prompt"
    );
    assert_eq!(runner.game.memory, 6, "branch 1 (+2) then the tail (+4)");
}

#[test]
fn legal_when_no_legal_branch_binds_nothing_and_continues() {
    let runner = run_choice(&choice_steps(vec![false, false], 2));
    assert!(runner.game.pending_selection.is_none());
    assert_eq!(runner.game.memory, 4, "no branch runs; the tail still does");
}

#[test]
fn legal_when_parses_and_compiles_parallel_to_labels() {
    let yaml = r#"
card: TEST-LEGAL-WHEN
name: Legal When
kind: digimon
color: [red]
level: 4
cost: 4
dp: 4000
effects:
  - when: on_play
    process:
      - select_effect_choice:
          bind_as: branch
          labels: ["A", "B"]
          legal_when:
            - any_permanent: { of: opponent, kind: digimon }
            - {}
          prompt: "Pick"
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile yaml");
    let dump = format!("{compiled:?}");
    assert!(dump.contains("legal_when: Some(["), "legal_when lowered: {dump}");

    let bad = yaml.replace("            - {}\n", "");
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(&bad).expect("parse yaml");
    assert!(
        digimon_dsl::compile::compile(&spec).is_err(),
        "legal_when must be parallel to labels"
    );
}
