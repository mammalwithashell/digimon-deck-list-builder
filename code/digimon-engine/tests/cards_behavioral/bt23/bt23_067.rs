//! BT23-067 LadyDevimon.

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::encode_attack;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const YAML: &str = include_str!("../../../cards/bt23/BT23-067.yaml");

#[test]
fn bt23_067_cost_reduces_and_grants_blocker() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT23-067 YAML loads")
        .add_card(digimon("ANGE", "Angewomon"))
        .hand(0, &["BT23-067"])
        .memory(10)
        .start();
    runner.place_on_field(0, "ANGE", Some(0));

    let before = runner.memory();
    runner.play(0, 0).expect("play BT23-067");
    assert_eq!(before - runner.memory(), 4, "7 printed cost - 3");
    let lady = runner.game.players[0]
        .battle_area
        .iter()
        .position(|perm| perm.top_card().card_id(&runner.game.card_data) == "BT23-067")
        .map(|index| digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: index as u8,
        })
        .expect("LadyDevimon on field");
    assert!(runner.game.has_keyword(lady, Keyword::Blocker));
}

#[test]
fn bt23_067_when_digivolving_deletes_opponent_level_4_or_lower() {
    let low = make_test_card_with_level("LOW", "Low", 4);
    let high = make_test_card_with_level("HIGH", "High", 5);
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT23-067 YAML loads")
        .add_card(make_test_card("BASE", "Base"))
        .add_card(low)
        .add_card(high)
        .memory(0)
        .start();
    let lady = runner.place_stack(0, &["BASE", "BT23-067"]);
    runner.place_on_field(1, "LOW", Some(0));
    runner.place_on_field(1, "HIGH", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(lady),
    );
    runner.game.drain_effect_queue();
    let view = runner
        .pending_selection_view()
        .expect("level 4 delete prompt");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the level 4 Digimon should be offered"
    );
    assert_mask_allows(&runner, view.selecting_player, view.valid_action_ids[0]);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("delete LOW");

    assert!(!permanent_exists(&runner, 1, "LOW"));
    assert!(permanent_exists(&runner, 1, "HIGH"));
}

#[test]
fn bt23_067_inherited_scapegoat_substitutes_other_digimon_for_deletion() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT23-067 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("COST", "Cost"))
        .start();
    let carrier = runner.place_stack(0, &["BT23-067", "CARRIER"]);
    let cost = runner.place_on_field(0, "COST", Some(0));

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    let accept = runner
        .pending_selection_view()
        .expect("Scapegoat accept prompt");
    assert_eq!(accept.kind, SelectionKind::Replacement);
    assert_mask_allows(&runner, accept.selecting_player, accept.valid_action_ids[0]);
    runner
        .execute_action(0, accept.valid_action_ids[0])
        .expect("accept Scapegoat");
    let _cost_prompt = runner
        .pending_selection_view()
        .expect("Scapegoat cost prompt");
    let cost_action = encode_attack(cost.player as u16, cost.index as u16);
    assert_mask_allows(&runner, 0, cost_action);
    runner
        .execute_action(0, cost_action)
        .expect("select other Digimon");

    assert!(permanent_exists(&runner, 0, "CARRIER"));
    assert!(!permanent_exists(&runner, 0, "COST"));
}

fn digimon(id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card
}

fn permanent_exists(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|permanent| permanent.top_card().card_id(&runner.game.card_data) == card_id)
}

fn assert_mask_allows(runner: &DebugRunner, player: u8, action: u16) {
    assert_eq!(
        build_action_mask(&runner.game, player)[action as usize],
        1.0,
        "pending action {action} must be legal in the action mask"
    );
}
