use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl::{compiled::CompiledPlayerRef, predicate::PredicateSpec, PlayerRef};
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::TriggerSource;

fn parse_predicate(yaml: &str) -> PredicateSpec {
    serde_yml::from_str(yaml).expect("predicate YAML parses")
}

#[test]
fn parses_digimon_attack_history_predicate_with_player_ref() {
    let pred = parse_predicate("digimon_attacked_this_turn: opponent");

    assert_eq!(
        pred.digimon_attacked_this_turn,
        Some(PlayerRef::Opponent),
        "DSL predicate must preserve the referenced player"
    );
}

#[test]
fn lowers_digimon_attack_history_predicate_into_active_when() {
    let yaml = r#"
card: TEST-ATTACK-HISTORY
name: Attack History Probe
kind: digimon
level: 3
color: [black]
cost: 3
dp: 1000
effects:
  - when: end_of_opponents_turn
    scope: inherited
    active_when:
      none_of:
        - digimon_attacked_this_turn: opponent
    process:
      - draw: { of: you, count: 1 }
"#;

    let runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("attack-history fixture YAML parses and compiles")
        .build();
    let card = runner
        .compiled_card("TEST-ATTACK-HISTORY")
        .expect("compiled card exists");
    let active_when = card.effects.iter().find_map(|clause| match clause {
        digimon_engine::dsl::compiled::CompiledClause::Triggered(triggered) => {
            triggered.active_when.as_ref()
        }
        _ => None,
    });

    let active_when = active_when.expect("inherited clause has active_when");
    assert_eq!(active_when.none_of.len(), 1);
    assert_eq!(
        active_when.none_of[0].digimon_attacked_this_turn,
        Some(CompiledPlayerRef::Opponent)
    );
}

#[test]
fn attack_history_predicate_is_false_until_referenced_player_attacks() {
    let mut attacker = make_test_card("ATTACKER-HISTORY", "Attacker History");
    attacker.dp = Some(3000);
    attacker.level = Some(3);
    let mut target = make_test_card("TARGET-HISTORY", "Target History");
    target.dp = Some(9000);
    target.level = Some(3);

    let yaml = r#"
card: TEST-ATTACK-HISTORY-RUNTIME
name: Attack History Runtime
kind: digimon
level: 3
color: [black]
cost: 3
dp: 1000
effects:
  - when: end_of_opponents_turn
    scope: inherited
    active_when:
      digimon_attacked_this_turn: opponent
    process:
      - draw: { of: you, count: 1 }
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("runtime fixture YAML parses")
        .add_card(attacker)
        .add_card(target)
        .deck(0, &["TARGET-HISTORY", "TARGET-HISTORY"])
        .build();

    let carrier = runner.place_stack(0, &["TEST-ATTACK-HISTORY-RUNTIME", "TARGET-HISTORY"]);
    let hand_before = runner.hand_size(0);
    runner.game.enqueue_triggered(
        EffectTiming::EndOfOpponentsTurn,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "without an opponent Digimon attack, positive attack-history predicate must be false"
    );

    runner.end_turn(); // move to player 1's turn for a real opponent attack
    let attacker = runner.place_on_field(1, "ATTACKER-HISTORY", Some(0));
    let defender = carrier;
    let _ = runner.attack_digimon(attacker, defender, false);
    runner.auto_resolve().expect("attack resolves");
    let hand_before = runner.hand_size(0);
    runner.game.enqueue_triggered(
        EffectTiming::EndOfOpponentsTurn,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "after the referenced opponent attacks with a Digimon, predicate-gated draw should fire"
    );
}

#[test]
fn attack_history_predicate_resets_on_next_turn() {
    let mut attacker = make_test_card("ATTACKER-HISTORY-RESET", "Attacker History Reset");
    attacker.dp = Some(3000);
    attacker.level = Some(3);
    let mut target = make_test_card("TARGET-HISTORY-RESET", "Target History Reset");
    target.dp = Some(9000);
    target.level = Some(3);

    let yaml = r#"
card: TEST-ATTACK-HISTORY-RESET
name: Attack History Reset
kind: digimon
level: 3
color: [black]
cost: 3
dp: 1000
effects:
  - when: end_of_opponents_turn
    scope: inherited
    active_when:
      digimon_attacked_this_turn: opponent
    process:
      - draw: { of: you, count: 1 }
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("reset fixture YAML parses")
        .add_card(attacker)
        .add_card(target)
        .deck(0, &["TARGET-HISTORY-RESET", "TARGET-HISTORY-RESET"])
        .build();
    let carrier = runner.place_stack(0, &["TEST-ATTACK-HISTORY-RESET", "TARGET-HISTORY-RESET"]);

    runner.end_turn(); // move to player 1's turn for a real opponent attack
    let attacker = runner.place_on_field(1, "ATTACKER-HISTORY-RESET", Some(0));
    let _ = runner.attack_digimon(attacker, carrier, false);
    runner.auto_resolve().expect("attack resolves");
    let hand_before = runner.hand_size(0);
    runner.game.enqueue_triggered(
        EffectTiming::EndOfOpponentsTurn,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();
    assert_eq!(runner.hand_size(0), hand_before + 1);

    runner.end_turn(); // finish player 1 turn -> player 0
    runner.end_turn(); // finish player 0 turn -> player 1; player 1 attack history resets
    let hand_after_reset = runner.hand_size(0);
    runner.game.enqueue_triggered(
        EffectTiming::EndOfOpponentsTurn,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();
    assert_eq!(
        runner.hand_size(0),
        hand_after_reset,
        "attack history must not leak into later turns"
    );
}
