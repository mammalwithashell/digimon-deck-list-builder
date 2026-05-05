//! BT22-040 Cendrillmon.
//! Printed text covered here:
//! <Overclock ([Puppet] Trait)>.
//! [On Play] [When Digivolving] You may play 1 Familiar Token.
//! [All Turns] [Once Per Turn] When any of your other Digimon are deleted,
//! you may activate 1 of this Digimon's [When Digivolving] effects.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, Keyword};
use digimon_engine::selection::{SelectionKind, TriggerSource};

fn runner_with_bt22_040() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT22-040")
        .expect("BT22-040 YAML loads")
        .add_card(make_test_card("BASE", "Base"))
        .add_card(make_test_card("ALLY-1", "Ally 1"))
        .add_card(make_test_card("ALLY-2", "Ally 2"))
        .add_card(make_test_card("OPPONENT", "Opponent"))
        .hand(0, &["BT22-040"])
        .memory(20)
        .start()
}

fn familiar_count(runner: &DebugRunner, player: usize) -> usize {
    runner.game.players[player]
        .battle_area
        .iter()
        .filter(|perm| perm.top_card().card_name(&runner.game.card_data) == "Familiar Token")
        .count()
}

#[test]
fn bt22_040_has_puppet_overclock() {
    let mut runner = runner_with_bt22_040();
    let cendrill = runner.place_on_field(0, "BT22-040", Some(0));

    assert!(
        runner.game.has_keyword(cendrill, Keyword::Overclock),
        "BT22-040 grants Overclock while face up"
    );
}

#[test]
fn bt22_040_on_play_may_play_one_familiar_token() {
    let mut runner = runner_with_bt22_040();

    runner.play(0, 0).expect("BT22-040 plays from hand");
    let choice = runner
        .pending_selection_view()
        .expect("On Play token effect says 'you may'");
    assert_eq!(choice.kind, SelectionKind::EffectChoice);
    runner
        .execute_action(0, choice.valid_action_ids[0])
        .expect("choose to play Familiar Token");
    runner
        .auto_resolve()
        .expect("accept optional On Play token");

    assert_eq!(
        familiar_count(&runner, 0),
        1,
        "one Familiar Token is played"
    );
}

#[test]
fn bt22_040_on_play_can_decline_token() {
    let mut runner = runner_with_bt22_040();

    runner.play(0, 0).expect("BT22-040 plays from hand");
    let choice = runner
        .pending_selection_view()
        .expect("On Play token effect says 'you may'");
    assert_eq!(choice.kind, SelectionKind::EffectChoice);
    runner
        .execute_action(0, choice.valid_action_ids[1])
        .expect("decline optional On Play token");
    runner.auto_resolve().expect("finish declined token effect");

    assert_eq!(familiar_count(&runner, 0), 0, "declining plays no token");
}

#[test]
fn bt22_040_when_digivolving_may_play_one_familiar_token() {
    let mut runner = runner_with_bt22_040();
    let cendrill = runner.place_stack(0, &["BASE", "BT22-040"]);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(cendrill),
    );
    runner.game.drain_effect_queue();

    let choice = runner
        .pending_selection_view()
        .expect("When Digivolving token effect says 'you may'");
    assert_eq!(choice.kind, SelectionKind::EffectChoice);
    runner
        .execute_action(0, choice.valid_action_ids[0])
        .expect("choose to play Familiar Token");
    runner
        .auto_resolve()
        .expect("accept optional When Digivolving token");

    assert_eq!(
        familiar_count(&runner, 0),
        1,
        "one Familiar Token is played"
    );
}

#[test]
#[ignore = "pending: G-ON-ANY-DELETION-EVENT-CONTEXT — refire_effect exists, but 'your other Digimon deleted' needs deleted-permanent event context"]
fn bt22_040_refires_when_digivolving_after_other_own_digimon_deleted() {
    todo!("unignore once OnAnyDeletion carries deleted-permanent context for 'your other Digimon'")
}

#[test]
#[ignore = "pending: G-ON-ANY-DELETION-EVENT-CONTEXT — paired with positive refire coverage"]
fn bt22_040_refire_can_be_declined() {
    todo!("unignore once the optional refire prompt can be gated to the deleted object")
}

#[test]
#[ignore = "pending: G-ON-ANY-DELETION-EVENT-CONTEXT — current event_target predicates over-trigger on nonmatching deletions"]
fn bt22_040_refire_only_triggers_for_other_own_digimon() {
    todo!("unignore once deleted-object owner and other-than-source predicates are faithful")
}

#[test]
#[ignore = "pending: G-ON-ANY-DELETION-EVENT-CONTEXT — once-per-turn refire must be tested with faithful deleted-object gating"]
fn bt22_040_other_deletion_refire_is_once_per_turn() {
    todo!("unignore once the refire observer can be authored without over-triggering")
}
