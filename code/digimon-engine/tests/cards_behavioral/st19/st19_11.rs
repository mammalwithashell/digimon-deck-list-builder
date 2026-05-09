//! ST19-11 Chaperomon.
//! Printed text covered here:
//! - [On Play] [When Digivolving] 1 opponent Digimon gets -3000 DP for the turn.
//!   If there are 3 or more Digimon, increase the DP reduction by -3000.
//! - Inherited: [All Turns] [Once Per Turn] when this Digimon would leave the
//!   battle area other than by your effects, by deleting 1 of your Tokens or
//!   other Puppet trait Digimon, prevent it from leaving.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

#[test]
fn st19_11_on_play_reduces_one_opponent_digimon_by_3000_with_two_total_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-11")
        .expect("ST19-11 YAML loads")
        .add_card(make_digimon("OPP", 7000, &[]))
        .hand(0, &["ST19-11"])
        .memory(10)
        .start();
    runner.place_on_field(1, "OPP", Some(0));

    runner.play(0, 0).expect("play Chaperomon");
    let view = runner.pending_selection_view().expect("DP target prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only opponent Digimon is legal"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose DP target");
    runner.auto_resolve().expect("finish DP reduction");

    let target = runner.game.players[1]
        .battle_area
        .iter()
        .position(|perm| perm.top_card().card_id(&runner.game.card_data) == "OPP")
        .expect("opponent target still present");
    let target = digimon_engine::permanent::PermanentHandle {
        player: 1,
        index: target as u8,
    };
    assert_eq!(
        runner.dp_of(target),
        Some(4000),
        "with only 2 total Digimon, target gets exactly -3000 DP"
    );
}

#[test]
fn st19_11_when_digivolving_reduces_one_opponent_digimon_by_3000_with_two_total_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-11")
        .expect("ST19-11 YAML loads")
        .add_card(make_digimon("BASE", 3000, &[]))
        .add_card(make_digimon("OPP", 7000, &[]))
        .start();
    let chaperomon = runner.place_stack(0, &["BASE", "ST19-11"]);
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(chaperomon),
    );
    runner.game.drain_effect_queue();

    let view = runner.pending_selection_view().expect("DP target prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose DP target");
    runner.auto_resolve().expect("finish DP reduction");

    assert_eq!(
        runner.dp_of(opp),
        Some(4000),
        "with only 2 total Digimon, target gets exactly -3000 DP"
    );
}

#[test]
#[ignore = "BLOCKED: subjectless DSL count_gte/threshold formula cannot faithfully express the 3+ Digimon extra -3000 branch"]
fn st19_11_dp_reduction_increases_to_6000_with_three_or_more_total_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-11")
        .expect("ST19-11 YAML loads")
        .add_card(make_digimon("BASE", 3000, &[]))
        .add_card(make_digimon("ALLY", 3000, &[]))
        .add_card(make_digimon("OPP", 7000, &[]))
        .start();
    let chaperomon = runner.place_stack(0, &["BASE", "ST19-11"]);
    runner.place_on_field(0, "ALLY", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(chaperomon),
    );
    runner.game.drain_effect_queue();
    let view = runner.pending_selection_view().expect("DP target prompt");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose DP target");
    runner.auto_resolve().expect("finish DP reduction");

    assert_eq!(
        runner.dp_of(opp),
        Some(1000),
        "3 total Digimon should increase the reduction to -6000"
    );
}

#[test]
fn st19_11_inherited_prevents_opponent_effect_leave_by_deleting_other_puppet() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-11")
        .expect("ST19-11 YAML loads")
        .add_card(make_digimon("CARRIER", 8000, &[]))
        .add_card(make_digimon("PUPPET-COST", 3000, &["Puppet"]))
        .start();
    let carrier = runner.place_stack(0, &["ST19-11", "CARRIER"]);
    runner.place_on_field(0, "PUPPET-COST", Some(0));

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    let accept = runner
        .pending_selection_view()
        .expect("inherited replacement accept prompt");
    assert_eq!(accept.kind, SelectionKind::Replacement);
    assert!(
        accept.is_optional,
        "prevention replacement must be a visible optional choice"
    );
    runner
        .execute_action(0, accept.valid_action_ids[0])
        .expect("accept inherited replacement");

    let view = runner
        .pending_selection_view()
        .expect("Puppet/token prevention cost prompt");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the other Puppet, not the carrier, can pay the cost"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("delete other Puppet");
    runner.auto_resolve().expect("finish prevention");

    assert_eq!(runner.battle_area_size(0), 1, "carrier survives");
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "CARRIER"),
        "inherited carrier remains in the battle area"
    );
}

#[test]
fn st19_11_inherited_does_not_prevent_own_effect_leave() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-11")
        .expect("ST19-11 YAML loads")
        .add_card(make_digimon("CARRIER", 8000, &[]))
        .add_card(make_digimon("PUPPET-COST", 3000, &["Puppet"]))
        .start();
    let carrier = runner.place_stack(0, &["ST19-11", "CARRIER"]);
    runner.place_on_field(0, "PUPPET-COST", Some(0));

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OwnEffect);

    assert!(
        runner.pending_selection_view().is_none(),
        "own effects must not offer the inherited prevention prompt"
    );
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "carrier leaves; only the untouched Puppet cost remains"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .all(|perm| perm.top_card().card_id(&runner.game.card_data) != "CARRIER"),
        "carrier is not prevented from leaving by your own effect"
    );
}

fn make_digimon(id: &str, dp: i32, traits: &[&str]) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.dp = Some(dp);
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}
