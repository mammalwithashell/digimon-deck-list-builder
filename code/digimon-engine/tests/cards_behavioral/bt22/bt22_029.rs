//! BT22-029 Shoemon.
//! Printed text: [On Play] [On Deletion] 1 of your Digimon with the [Puppet] trait
//! gains <Blocker> until your opponent's turn ends. Inherited: [When Attacking]
//! [Once Per Turn] 1 of your opponent's Digimon gets -2000 DP for the turn.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::selection::SelectionKind;

#[test]
fn bt22_029_on_play_grants_blocker_to_selected_puppet() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-029")
        .expect("BT22-029 YAML loads")
        .add_card(make_trait_card("PUPPET", &["Puppet"]))
        .hand(0, &["BT22-029"])
        .memory(10)
        .start();
    let puppet = runner.place_on_field(0, "PUPPET", Some(0));

    runner.play(0, 0).expect("play Shoemon");

    let view = runner
        .pending_selection_view()
        .expect("Puppet target prompt");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "self and the ally Puppet are legal targets"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose Puppet");
    runner.auto_resolve().expect("finish grant");

    assert!(
        runner
            .game
            .modifiers
            .has_keyword(puppet, digimon_engine::Keyword::Blocker),
        "selected Puppet gains Blocker"
    );
}

#[test]
fn bt22_029_inherited_when_attacking_debuffs_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-029")
        .expect("BT22-029 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_digimon_dp("OPP", 3000))
        .memory(0)
        .start();
    let carrier = runner.place_stack(0, &["BT22-029", "CARRIER"]);
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.game.enqueue_triggered(
        digimon_engine::EffectTiming::WhenAttacking,
        digimon_engine::TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("opponent Digimon target prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose opponent Digimon");
    runner.auto_resolve().expect("finish inherited effect");

    assert_eq!(
        runner.effective_dp(opp),
        Some(1000),
        "3000 DP test card gets -2000 DP"
    );
}

fn make_trait_card(id: &str, traits: &[&str]) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn make_digimon_dp(id: &str, dp: i32) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.dp = Some(dp);
    card
}
