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

/// "[All Turns] When any Digimon are deleted, gain memory equal to that
/// Digimon's level." — the body reads the singular event subject.
const OBS_LEVEL: &str = r#"
card: T-OBS-LEVEL
name: LevelObserver
kind: tamer
color: [red]
cost: 3
traits: []
effects:
  - when: on_any_deletion
    active_when: { all_turns: true }
    trigger_if: { event_target_kind: digimon }
    summary: "[All Turns] When any Digimon are deleted, gain memory equal to that Digimon's level"
    process:
      - gain_memory: { event_target_level: {} }
"#;

fn runner(observer: &str) -> DebugRunner {
    let mut d = make_test_card("D", "D");
    d.card_kind = CardKind::Digimon;
    d.level = Some(4);
    d.dp = Some(4000);
    d.colors = vec![CardColor::Red];
    let mut e = make_test_card("E", "E");
    e.card_kind = CardKind::Digimon;
    e.level = Some(6);
    e.dp = Some(9000);
    e.colors = vec![CardColor::Red];
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(observer)
        .expect("observer")
        .add_card(d)
        .add_card(e)
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

#[test]
fn an_observer_reading_that_digimon_chooses_which_deleted_digimon_it_means() {
    // One trigger for the batch (§15-5-2), but "that Digimon" is singular:
    // with two qualifying deleted Digimon the player chooses which one the
    // effect refers to (no auto-selection).
    let mut r = runner(OBS_LEVEL);
    r.place_on_field(0, "T-OBS-LEVEL", Some(0));
    let d = r.place_on_field(1, "D", Some(0));
    let e = r.place_on_field(1, "E", Some(0));
    let mem = r.memory();
    r.game
        .delete_permanents_batch(vec![d, e], ReplacementCause::OpponentEffect);
    // No auto_resolve here: it would answer the subject choice itself.
    let v = r
        .pending_selection_view()
        .expect("a choice of the deleted Digimon the effect refers to");
    assert_eq!(v.prompt, "Choose the card referred to by this effect");
    assert!(!v.is_optional, "the trigger is mandatory");
    let choices = v.effect_choices.clone().expect("one choice per deleted Digimon");
    assert_eq!(choices.len(), 2, "{choices:?}");
    assert!(
        choices.iter().all(|c| !c.label.contains("play")),
        "labels are not play-specific: {choices:?}"
    );
    let pick_e = choices
        .iter()
        .find(|c| c.label.ends_with(": E"))
        .unwrap_or_else(|| panic!("a label for E: {choices:?}"))
        .action_id;
    r.execute_action(v.selecting_player, pick_e).unwrap();
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.memory(), mem + 6, "the chosen Digimon's level, once");
}

