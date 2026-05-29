//! ST5-06 Greymon — Lv.4, Black.
//!
//! Printed inherited text:
//! [End of Opponent's Turn] If your opponent didn't attack with a Digimon this
//! turn, <Draw 1>.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::TriggerSource;

fn runner() -> DebugRunner {
    let mut host = make_test_card("ST5-HOST-06", "ST5 Host 06");
    host.dp = Some(5000);
    host.level = Some(5);
    let mut attacker = make_test_card("ST5-ATTACKER-06", "ST5 Attacker 06");
    attacker.dp = Some(4000);
    attacker.level = Some(3);

    DebugRunner::builder()
        .dsl_card("ST5-06")
        .expect("ST5-06 YAML parses and compiles")
        .add_card(host)
        .add_card(attacker)
        .deck(0, &["ST5-HOST-06", "ST5-HOST-06", "ST5-HOST-06"])
        .build()
}

#[test]
fn st5_06_inherited_draws_when_opponent_did_not_attack_with_digimon() {
    let mut runner = runner();
    runner.place_stack(0, &["ST5-06", "ST5-HOST-06"]);

    let hand_before = runner.hand_size(0);
    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfOpponentsTurn, TriggerSource::PlayerBattleArea(0));
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "ST5-06 inherited effect should draw 1 when opponent did not attack"
    );
}

#[test]
fn st5_06_inherited_does_not_draw_after_opponent_attacked_with_digimon() {
    let mut runner = runner();
    let defender = runner.place_stack(0, &["ST5-06", "ST5-HOST-06"]);
    runner.end_turn(); // P0 -> P1

    let attacker = runner.place_on_field(1, "ST5-ATTACKER-06", Some(0));
    let _ = runner.attack_digimon(attacker, defender, false);
    runner.auto_resolve().expect("attack resolves");

    let hand_before = runner.hand_size(0);
    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfOpponentsTurn, TriggerSource::PlayerBattleArea(0));
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "ST5-06 inherited effect must not draw after opponent attacked"
    );
}
