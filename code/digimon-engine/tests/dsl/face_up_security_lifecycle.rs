use digimon_dsl::compiled::{CompiledClause, CompiledStep};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::permanent::PermanentHandle;

fn compile_steps(step_yaml: &str) -> Vec<CompiledStep> {
    let yaml = format!(
        r#"
card: FACE-UP-FLIPPER
name: Face-Up Flipper
kind: digimon
level: 3
color: [black]
cost: 3
dp: 1000
effects:
  - when: on_play
    process:
{step_yaml}
"#
    );
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(&yaml).expect("YAML parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("YAML compiles");
    match &compiled.effects[0] {
        CompiledClause::Triggered(t) => t.process.clone(),
        other => panic!("expected triggered clause, got {other:?}"),
    }
}

fn run_compiled_steps(
    runner: &mut DebugRunner,
    source_card: CardHandle,
    source_permanent: Option<PermanentHandle>,
    steps: Vec<CompiledStep>,
) {
    let mut ctx = EffectContext::new(&mut runner.game, source_card, source_permanent, 0);
    let mut bindings = Bindings::new();
    run_steps(&steps, &mut ctx, &mut bindings);
}

#[test]
fn flip_security_face_up_marks_topmost_face_down_security_without_choice() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SOURCE", "Source"))
        .add_card(make_test_card("BOTTOM", "Bottom Security"))
        .add_card(make_test_card("MIDDLE", "Middle Security"))
        .add_card(make_test_card("TOP", "Top Security"))
        .security(1, &["BOTTOM", "MIDDLE", "TOP"])
        .start();
    let source = runner.place_on_field(0, "SOURCE", Some(0));
    let source_card = runner.game.players[0].battle_area[source.index as usize]
        .top_card()
        .handle();
    let top = runner.game.players[1].security[2].handle();
    let middle = runner.game.players[1].security[1].handle();

    runner.game.players[1].face_up_security.insert(top.0);

    run_compiled_steps(
        &mut runner,
        source_card,
        Some(source),
        compile_steps("      - flip_security_face_up:\n          of: opponent\n"),
    );

    assert!(
        runner.game.pending_selection.is_none(),
        "flipping security face-up must not surface a player choice"
    );
    assert!(
        runner.game.players[1].face_up_security.contains(&top.0),
        "already-face-up top security remains face-up"
    );
    assert!(
        runner.game.players[1].face_up_security.contains(&middle.0),
        "the topmost still-face-down security card should be flipped"
    );
}

#[test]
fn when_your_digimon_checks_face_up_security_fires_from_attacker_side() {
    let observer_yaml = r#"
card: FACE-UP-OBSERVER
name: Face-Up Observer
kind: digimon
level: 3
color: [black]
cost: 3
dp: 1000
effects:
  - when: on_check_face_up_security
    process:
      - gain_memory: 2
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(observer_yaml)
        .unwrap()
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("SECURITY", "Security"))
        .security(1, &["SECURITY"])
        .memory(0)
        .start();
    runner.place_on_field(0, "FACE-UP-OBSERVER", Some(0));
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let security = runner.game.players[1].security[0].handle();
    runner.game.players[1].face_up_security.insert(security.0);

    let _ = runner.attack_player(attacker, 1, true);
    runner
        .auto_resolve()
        .expect("face-up security observer should drain");

    assert_eq!(
        runner.memory(),
        2,
        "attacker-side observer should gain memory when its Digimon checks face-up security"
    );
}
