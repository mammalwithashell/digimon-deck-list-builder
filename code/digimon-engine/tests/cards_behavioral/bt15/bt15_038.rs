//! BT15-038 Angewomon.

use digimon_dsl::compiled::CompiledAltPathKind;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;
use digimon_engine::trigger_context::EventCause;

const YAML: &str = include_str!("../../../cards/bt15/BT15-038.yaml");

#[test]
fn bt15_038_has_blast_digivolve_and_ace_overflow_minus_3() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT15-038 YAML loads")
        .start();
    let card = runner.compiled_card("BT15-038").expect("compiled BT15-038");

    assert_eq!(card.ace_overflow, Some(-3));
    assert!(card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::BurstDigivolve
            && path.marker
            && path
                .cost
                .as_ref()
                .is_some_and(|cost| matches!(cost, digimon_dsl::compiled::CompiledCost::Literal(0)))
    }));
    assert!(card.effects.iter().any(|clause| {
        matches!(
            clause,
            digimon_dsl::compiled::CompiledClause::Declarative(
                digimon_dsl::compiled::CompiledDeclarativeClause::GrantKeyword {
                    keyword,
                    ..
                }
            ) if keyword == "BlastDigivolve"
        )
    }));
}

#[test]
fn bt15_038_top_or_bottom_security_cost_applies_minus_6000_dp() {
    let mut runner = angel_runner(YAML, "BT15-038", 7000)
        .security(0, &["SEC1", "SEC2", "SEC3", "BOTTOM", "TOP"])
        .start();
    let source = runner.place_on_field(0, "BT15-038", Some(0));
    let target = runner.place_on_field(1, "TARGET", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
    runner.execute_branch(0).expect("choose top security cost");

    let pick = runner
        .pending_selection_view()
        .expect("opponent Digimon DP-minus target");
    runner
        .execute_action(pick.selecting_player, pick.valid_action_ids[0])
        .expect("choose opponent target");

    assert_eq!(runner.security_count(0), 4);
    assert!(
        zone_ids(&runner.game.players[0].trash, &runner.game.card_data)
            .contains(&"TOP".to_string())
    );
    assert_eq!(runner.dp_of(target), Some(1000));
}

#[test]
fn bt15_038_security_removed_recovers_once_at_three_or_fewer_security() {
    let mut runner = angel_runner(YAML, "BT15-038", 7000)
        .security(0, &["SEC1", "SEC2", "SEC3"])
        .deck(0, &["RECOVER2", "RECOVER1"])
        .memory(0)
        .start();
    runner.place_on_field(0, "BT15-038", Some(0));

    fire_own_security_removed(&mut runner);
    fire_own_security_removed(&mut runner);

    assert_eq!(runner.security_count(0), 4);
    assert_eq!(
        runner.game.players[0]
            .security
            .last()
            .unwrap()
            .card_id(&runner.game.card_data),
        "RECOVER1"
    );
}

fn angel_runner(
    yaml: &str,
    card_id: &str,
    target_dp: i32,
) -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("card YAML loads")
        .add_card(make_test_card("SEC1", "Security 1"))
        .add_card(make_test_card("SEC2", "Security 2"))
        .add_card(make_test_card("SEC3", "Security 3"))
        .add_card(make_test_card("BOTTOM", "Bottom"))
        .add_card(make_test_card("TOP", "Top"))
        .add_card(make_test_card("RECOVER1", "Recover 1"))
        .add_card(make_test_card("RECOVER2", "Recover 2"))
        .add_card(digimon("TARGET", "Target", target_dp))
        .add_card(digimon(card_id, card_id, 7000))
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

fn digimon(id: &str, name: &str, dp: i32) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(dp);
    card
}

fn zone_ids(zone: &[digimon_engine::card_source::CardSource], data: &[CardData]) -> Vec<String> {
    zone.iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}
