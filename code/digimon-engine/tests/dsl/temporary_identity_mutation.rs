use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const ORIGINAL_NAME_MUTATION_YAML: &str = r#"
card: TEST-ORIGINAL-NAME-MUTATOR
name: Original Name Mutator
kind: digimon
level: 5
color: [yellow]
cost: 7
dp: 7000
traits: []
effects:
  - when: on_play
    process:
      - change_original_name:
          target: { of: opponent, zone: [battle_area], kind: digimon }
          name: Sukamon
          expiry: end_of_opponents_turn
"#;

const COLOR_MUTATION_YAML: &str = r#"
card: TEST-COLOR-MUTATOR
name: Color Mutator
kind: digimon
level: 5
color: [yellow]
cost: 7
dp: 7000
traits: []
effects:
  - when: on_play
    process:
      - change_color:
          target: { of: opponent, zone: [battle_area], kind: digimon }
          color: white
          expiry: end_of_opponents_turn
"#;

fn runner() -> DebugRunner {
    let mut target = make_test_card("TEST-NAME-TARGET", "Targetmon");
    target.dp = Some(6000);

    DebugRunner::builder()
        .from_dsl_yaml(ORIGINAL_NAME_MUTATION_YAML)
        .expect("register original-name mutator")
        .add_card(target)
        .build()
}

fn color_runner() -> DebugRunner {
    let mut target = make_test_card("TEST-COLOR-TARGET", "Color Target");
    target.dp = Some(6000);
    target.colors = vec![CardColor::Red, CardColor::Blue];

    DebugRunner::builder()
        .from_dsl_yaml(COLOR_MUTATION_YAML)
        .expect("register color mutator")
        .add_card(target)
        .build()
}

fn fire_on_play(runner: &mut DebugRunner, source: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

fn target_has_name(runner: &DebugRunner, target: PermanentHandle, name: &str) -> bool {
    runner.game.players[target.player as usize].battle_area[target.index as usize]
        .contains_card_name_for_rules(name, &runner.game.card_data, &runner.game.modifiers, target)
}

fn target_colors(runner: &DebugRunner, target: PermanentHandle) -> Vec<CardColor> {
    runner.game.players[target.player as usize].battle_area[target.index as usize].colors_for_rules(
        &runner.game.card_data,
        &runner.game.modifiers,
        target,
    )
}

#[test]
fn change_original_name_makes_name_predicates_match_until_expiry() {
    let mut runner = runner();
    let source = runner.place_on_field(0, "TEST-ORIGINAL-NAME-MUTATOR", Some(0));
    let target = runner.place_on_field(1, "TEST-NAME-TARGET", Some(0));

    assert!(!target_has_name(&runner, target, "Sukamon"));
    fire_on_play(&mut runner, source);

    assert!(
        target_has_name(&runner, target, "Sukamon"),
        "temporary original-name mutation must make name predicates see Sukamon"
    );
}

#[test]
fn change_original_name_expires_and_restores_printed_name_predicates() {
    let mut runner = runner();
    let source = runner.place_on_field(0, "TEST-ORIGINAL-NAME-MUTATOR", Some(0));
    let target = runner.place_on_field(1, "TEST-NAME-TARGET", Some(0));

    fire_on_play(&mut runner, source);
    assert!(target_has_name(&runner, target, "Sukamon"));
    runner.end_turn();
    assert!(
        target_has_name(&runner, target, "Sukamon"),
        "name mutation should remain active during the opponent's turn"
    );
    runner.end_turn();

    assert!(
        !target_has_name(&runner, target, "Sukamon"),
        "name mutation must expire at end of opponent's turn"
    );
    assert!(
        target_has_name(&runner, target, "Targetmon"),
        "printed name must be visible again after expiry"
    );
}

#[test]
fn change_color_makes_rules_visible_color_match_until_expiry() {
    let mut runner = color_runner();
    let source = runner.place_on_field(0, "TEST-COLOR-MUTATOR", Some(0));
    let target = runner.place_on_field(1, "TEST-COLOR-TARGET", Some(0));

    assert_eq!(
        target_colors(&runner, target),
        vec![CardColor::Red, CardColor::Blue]
    );
    fire_on_play(&mut runner, source);

    assert_eq!(
        target_colors(&runner, target),
        vec![CardColor::White],
        "temporary color mutation must replace rules-visible colors"
    );
}

#[test]
fn change_color_expires_and_restores_printed_color() {
    let mut runner = color_runner();
    let source = runner.place_on_field(0, "TEST-COLOR-MUTATOR", Some(0));
    let target = runner.place_on_field(1, "TEST-COLOR-TARGET", Some(0));

    fire_on_play(&mut runner, source);
    assert_eq!(target_colors(&runner, target), vec![CardColor::White]);
    runner.end_turn();
    assert_eq!(
        target_colors(&runner, target),
        vec![CardColor::White],
        "color mutation should remain active during the opponent's turn"
    );
    runner.end_turn();

    assert_eq!(
        target_colors(&runner, target),
        vec![CardColor::Red, CardColor::Blue],
        "printed colors must be visible again after expiry"
    );
}
