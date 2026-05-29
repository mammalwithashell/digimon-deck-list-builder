//! BT15-042 Magnadramon.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;
use digimon_engine::trigger_context::EventCause;

const YAML: &str = include_str!("../../../cards/bt15/BT15-042.yaml");

#[test]
fn bt15_042_top_or_bottom_security_cost_applies_minus_9000_dp() {
    let mut runner = angel_runner(11000)
        .security(0, &["SEC1", "SEC2", "SEC3", "BOTTOM", "TOP"])
        .start();
    let source = runner.place_on_field(0, "BT15-042", Some(0));
    let target = runner.place_on_field(1, "TARGET", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
    runner
        .execute_branch(1)
        .expect("choose bottom security cost");

    let pick = runner
        .pending_selection_view()
        .expect("opponent Digimon DP-minus target");
    runner
        .execute_action(pick.selecting_player, pick.valid_action_ids[0])
        .expect("choose opponent target");

    assert_eq!(runner.security_count(0), 4);
    assert_eq!(
        runner.game.players[0]
            .security
            .first()
            .unwrap()
            .card_id(&runner.game.card_data),
        "SEC2",
        "choosing bottom should trash index 0"
    );
    assert!(
        zone_ids(&runner.game.players[0].trash, &runner.game.card_data)
            .contains(&"SEC1".to_string())
    );
    assert_eq!(runner.dp_of(target), Some(2000));
}

#[test]
fn bt15_042_security_removed_may_place_yellow_card_top_or_bottom_security_once() {
    let mut runner = angel_runner(11000)
        .hand(0, &["YELLOW-HAND", "RED-HAND"])
        .security(0, &["BOTTOM", "TOP"])
        .memory(0)
        .start();
    runner.place_on_field(0, "BT15-042", Some(0));

    fire_own_security_removed(&mut runner);
    assert!(
        runner.pending_is_optional(),
        "the security-placement observer is a may effect"
    );
    let hand_pick = runner
        .pending_selection_view()
        .expect("yellow hand-card selection");
    assert_eq!(
        hand_pick.valid_action_ids.len(),
        1,
        "only the yellow hand card should be legal"
    );
    runner
        .execute_action(hand_pick.selecting_player, hand_pick.valid_action_ids[0])
        .expect("select yellow card");
    runner
        .execute_branch(1)
        .expect("choose bottom security placement");

    assert_eq!(runner.security_count(0), 3);
    assert_eq!(
        runner.game.players[0]
            .security
            .first()
            .unwrap()
            .card_id(&runner.game.card_data),
        "YELLOW-HAND"
    );
    assert!(
        zone_ids(&runner.game.players[0].hand, &runner.game.card_data)
            .contains(&"RED-HAND".to_string())
    );

    fire_own_security_removed(&mut runner);
    assert!(
        runner.pending_selection().is_none(),
        "once-per-turn observer must not prompt a second time"
    );
}

fn angel_runner(target_dp: i32) -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT15-042 YAML loads")
        .add_card(make_test_card("SEC1", "Security 1"))
        .add_card(make_test_card("SEC2", "Security 2"))
        .add_card(make_test_card("SEC3", "Security 3"))
        .add_card(make_test_card("BOTTOM", "Bottom"))
        .add_card(make_test_card("TOP", "Top"))
        .add_card(digimon("TARGET", "Target", target_dp, CardColor::Purple))
        .add_card(digimon("BT15-042", "Magnadramon", 11000, CardColor::Yellow))
        .add_card(digimon(
            "YELLOW-HAND",
            "Yellow Hand",
            1000,
            CardColor::Yellow,
        ))
        .add_card(digimon("RED-HAND", "Red Hand", 1000, CardColor::Red))
}

fn fire_own_security_removed(runner: &mut DebugRunner) {
    runner.game.enqueue_triggered(
        EffectTiming::OnOwnSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player: 0,
            observer_player: 0,
            source_player: 1,
            card: digimon_engine::card_source::CardHandle(0),
            cause: EventCause::SecurityRemoval,
        },
    );
    runner.game.drain_effect_queue();
}

fn digimon(id: &str, name: &str, dp: i32, color: CardColor) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.dp = Some(dp);
    card
}

fn zone_ids(zone: &[digimon_engine::card_source::CardSource], data: &[CardData]) -> Vec<String> {
    zone.iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}
