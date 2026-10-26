use digimon_engine::debug_runner::make_test_dna_card;
use digimon_engine::enums::CardColor;
#[path = "../cards_behavioral/ex12/support.rs"]
mod support;
use support::{select_first_non_pass, select_hand_card, vb_digimon, DebugRunner};
const CARD_ID: &str = "EX12-001";

fn telemetry_fixture(two_eggs: bool, legal_target: bool) -> DebugRunner {
    let mut target = make_test_dna_card("VB-DNA", "VB DNA Result", 3, 3, 0);
    target.traits = vec![if legal_target { "VB" } else { "Other" }.to_string()];
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .unwrap()
        .add_card(vb_digimon("CARRIER", CardColor::Yellow, 3, 3000))
        .add_card(vb_digimon("PARTNER", CardColor::Red, 3, 3000))
        .add_card(target)
        .hand(0, &["VB-DNA"])
        .memory(3)
        .start();
    runner.game.turn_count = 1;
    runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    if two_eggs {
        runner.place_stack(0, &[CARD_ID, "PARTNER"]);
    } else {
        runner.place_on_field(0, "PARTNER", Some(0));
    }
    runner
}

#[test]
fn nyaromon_evaluation_metadata_tracks_legal_prompts_without_mutation() {
    let mut runner = telemetry_fixture(false, true);
    runner.game.fire_end_of_your_turn(0);
    let before = digimon_engine::serialization::to_ui_json(&runner.game);
    let context = digimon_engine::evaluation_context::decision_context(&runner.game);
    assert_eq!(
        before,
        digimon_engine::serialization::to_ui_json(&runner.game)
    );
    assert_eq!(context["effects"][0]["stage"], "dna_partner");
    assert_eq!(context["effects"][0]["card_id"], CARD_ID);
    assert_eq!(context["effects"][0]["player"], 1);
    let sequence = context["effects"][0]["sequence"].clone();
    select_first_non_pass(&mut runner);
    let result = digimon_engine::evaluation_context::decision_context(&runner.game);
    assert_eq!(result["effects"][0]["stage"], "dna_result");
    assert_eq!(result["effects"][0]["sequence"], sequence);
    select_hand_card(&mut runner, 0, "VB-DNA");
    let attack = digimon_engine::evaluation_context::decision_context(&runner.game);
    assert_eq!(
        attack["effects"][0]["stage"],
        "followup_attack",
        "pending={:?}, resume={:?}",
        runner.pending_selection_view(),
        runner.game.pending_selection_resume
    );
    assert_eq!(attack["effects"][0]["sequence"], sequence);
    let clone = runner.game.clone();
    assert_eq!(
        attack,
        digimon_engine::evaluation_context::decision_context(&clone)
    );
}

#[test]
fn nyaromon_evaluation_metadata_covers_order_declines_and_separate_eggs() {
    let mut runner = telemetry_fixture(true, true);
    runner.game.fire_end_of_your_turn(0);
    let context = digimon_engine::evaluation_context::decision_context(&runner.game);
    let effects = context["effects"].as_array().unwrap();
    assert_eq!(effects.len(), 2);
    assert!(effects.iter().all(|e| e["stage"] == "trigger_order"));
    assert_ne!(effects[0]["sequence"], effects[1]["sequence"]);
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .unwrap();
    assert!(
        digimon_engine::evaluation_context::decision_context(&runner.game)["effects"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn nyaromon_evaluation_metadata_excludes_unusable_dna_targets() {
    let mut runner = telemetry_fixture(true, false);
    runner.game.fire_end_of_your_turn(0);
    assert!(
        digimon_engine::evaluation_context::decision_context(&runner.game)["effects"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
