use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::EffectTiming;
use digimon_engine::replacement::ReplacementCause;

#[test]
fn partition_declarative_runs_process_body_on_deletion() {
    let yaml = r#"
card: DSL-3C-PARTITION
name: Partition Body
kind: digimon
level: 5
color: [red]
cost: 7
dp: 7000
effects:
  - kind: partition
    sources: []
    exclude_cause: []
    process:
      - gain_memory: 2
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .memory(0)
        .start();
    let handle = runner.place_on_field(0, "DSL-3C-PARTITION", Some(0));

    runner
        .game
        .delete_permanent_with_cause(handle, ReplacementCause::OpponentEffect);

    assert_eq!(runner.memory(), 2);
}

#[test]
fn delay_declarative_uses_continuation_aware_process_body() {
    let yaml = r#"
card: DSL-3C-DELAY
name: Delay Body
kind: option
color: [red]
cost: 0
effects:
  - kind: delay
    trigger: end_of_your_turn
    process:
      - select_effect_choice:
          labels: ["first", "second"]
          bind_as: branch
          prompt: "Pick delay branch"
      - gain_memory: 3
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .hand(0, &["DSL-3C-DELAY"])
        .start();
    let source = runner.game.players[0].hand[0].handle();
    let effects = runner
        .game
        .effects_for_card("DSL-3C-DELAY", source)
        .expect("delay effect");
    let delay = effects
        .iter()
        .find(|effect| effect.timing == EffectTiming::DelayEffect)
        .expect("DelayEffect");
    let process = delay.process.as_ref().expect("delay process");

    let before = runner.memory();
    {
        let mut ctx = EffectContext::new(&mut runner.game, source, None, 0);
        process(&mut ctx);
    }

    assert!(runner.pending_selection().is_some());
    assert_eq!(runner.memory(), before, "tail must wait for selection");

    runner.execute_branch(0).expect("delay branch resolves");
    assert_eq!(runner.memory(), before + 3);
}
