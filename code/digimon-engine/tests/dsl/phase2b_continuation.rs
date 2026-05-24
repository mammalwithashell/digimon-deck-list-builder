//! Continuation dispatcher: a step slice with no selection steps runs
//! straight through. (Selection-step parking test lands in Task 4 once
//! the first selection handler exists.)

use digimon_dsl::compiled::CompiledStep;
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::TriggerSource;

#[test]
fn run_steps_with_no_selections_executes_all_steps_inline() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F"])
        .build();
    let card = runner.game.players[0].hand[0].handle();
    let before = runner.game.memory;
    {
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        let mut b = Bindings::new();
        run_steps(
            &[CompiledStep::GainMemory(1), CompiledStep::GainMemory(2)],
            &mut ctx,
            &mut b,
        );
    }
    assert_eq!(runner.game.memory, before + 3);
}

#[test]
fn optional_select_hand_decline_runs_outer_tail() {
    let yaml = r#"
card: TST-HAND-TAIL
name: Hand Tail
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: picked
          optional: true
          filter: { trait_has: TailCandidate }
          prompt: "You may pick a card"
      - gain_memory: 1
"#;
    let mut candidate = make_test_card("TAIL-CANDIDATE", "Tail Candidate");
    candidate.traits = vec!["TailCandidate".to_string()];
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("test YAML loads")
        .add_card(candidate)
        .hand(0, &["TAIL-CANDIDATE"])
        .memory(0)
        .start();
    let source = runner.place_on_field(0, "TST-HAND-TAIL", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_is_optional(),
        "optional select_hand must expose a decline path"
    );
    runner
        .execute_action(0, PASS)
        .expect("decline optional select_hand");

    assert_eq!(
        runner.memory(),
        1,
        "declining the optional selection skips its binding but still resumes the mandatory tail"
    );
}
