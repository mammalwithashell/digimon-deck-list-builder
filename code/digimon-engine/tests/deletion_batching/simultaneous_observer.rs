//! §15-5-2: Digimon deleted by one batch are deleted at the same time — one
//! event — so a "when ... Digimon are deleted" observer triggers ONCE for the
//! batch, not once per deleted Digimon (DCGO stacks one `OnDestroyedAnyone`
//! carrying every deleted permanent; `CanTriggerOnPermanentDeleted` matches
//! when any of them qualifies). A later batch is a new event and triggers it
//! again. G-ENGINE-DELETION-OBSERVER-TRIGGERS-PER-DELETED-DIGIMON.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementCause;

/// "[All Turns] When any Digimon are deleted, gain 1 memory."
const OBS_ANY: &str = r#"
card: T-OBS-ANY
name: AnyObserver
kind: tamer
color: [red]
cost: 3
traits: []
effects:
  - when: on_any_deletion
    active_when: { all_turns: true }
    trigger_if: { event_target_kind: digimon }
    summary: "[All Turns] When any Digimon are deleted, gain 1 memory"
    process:
      - gain_memory: 1
"#;

/// "[All Turns] When one of your opponent's Digimon is deleted, gain 1
/// memory." — the qualifier names the deleted Digimon.
const OBS_OPP: &str = r#"
card: T-OBS-OPP
name: OppObserver
kind: tamer
color: [red]
cost: 3
traits: []
effects:
  - when: on_any_deletion
    active_when: { all_turns: true }
    trigger_if:
      all_of:
        - event_target_owner: opponent
        - event_target_kind: digimon
    summary: "[All Turns] When one of your opponent's Digimon is deleted, gain 1 memory"
    process:
      - gain_memory: 1
"#;

fn runner(observer: &str) -> DebugRunner {
    let mut d = make_test_card("D", "D");
    d.card_kind = CardKind::Digimon;
    d.level = Some(4);
    d.dp = Some(4000);
    d.colors = vec![CardColor::Red];
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(observer)
        .expect("observer")
        .add_card(d)
        .memory(0)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn two_digimon_deleted_at_once_trigger_an_any_deletion_observer_once() {
    let mut r = runner(OBS_ANY);
    r.place_on_field(0, "T-OBS-ANY", Some(0));
    let a = r.place_on_field(1, "D", Some(0));
    let b = r.place_on_field(1, "D", Some(0));
    let mem = r.memory();
    r.game
        .delete_permanents_batch(vec![a, b], ReplacementCause::OpponentEffect);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem + 1, "one event, one trigger (§15-5-2)");
}

#[test]
fn separate_deletion_batches_trigger_the_observer_again() {
    let mut r = runner(OBS_ANY);
    r.place_on_field(0, "T-OBS-ANY", Some(0));
    let a = r.place_on_field(1, "D", Some(0));
    let b = r.place_on_field(1, "D", Some(0));
    let mem = r.memory();
    r.game
        .delete_permanents_batch(vec![b], ReplacementCause::OpponentEffect);
    let _ = r.auto_resolve();
    r.game
        .delete_permanents_batch(vec![a], ReplacementCause::OpponentEffect);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem + 2, "each event triggers it once");
}

#[test]
fn a_mixed_batch_triggers_a_qualified_observer_once_when_any_member_qualifies() {
    let mut r = runner(OBS_OPP);
    r.place_on_field(0, "T-OBS-OPP", Some(0));
    let own = r.place_on_field(0, "D", Some(0));
    let opp = r.place_on_field(1, "D", Some(0));
    let mem = r.memory();
    r.game
        .delete_permanents_batch(vec![own, opp], ReplacementCause::OpponentEffect);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem + 1, "the opponent's Digimon qualifies");
}

#[test]
fn a_batch_with_no_qualifying_member_does_not_trigger() {
    let mut r = runner(OBS_OPP);
    r.place_on_field(0, "T-OBS-OPP", Some(0));
    let a = r.place_on_field(0, "D", Some(0));
    let b = r.place_on_field(0, "D", Some(0));
    let mem = r.memory();
    r.game
        .delete_permanents_batch(vec![a, b], ReplacementCause::OpponentEffect);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem, "only your own Digimon were deleted");
}
