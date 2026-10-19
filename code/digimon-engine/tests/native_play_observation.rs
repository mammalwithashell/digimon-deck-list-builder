//! Observational identity must never alter gameplay or match a re-played copy.
use digimon_engine::{
    debug_runner::{make_test_card, DebugRunner},
    enums::{CostDelta, GamePhase, PlaySource},
};

fn setup() -> DebugRunner {
    let mut card = make_test_card("OBS-NATIVE", "Observed Native");
    card.play_cost = 0;
    let mut r = DebugRunner::builder()
        .add_card(card)
        .hand(0, &["OBS-NATIVE", "OBS-NATIVE", "OBS-NATIVE"])
        .memory(5)
        .start();
    r.game.current_phase = GamePhase::Main;
    r
}

#[test]
fn native_entries_distinguish_identical_copies_and_survive_index_shifts_and_clones() {
    let mut r = setup();
    r.game.enable_native_play_observation(true);
    let first = r.game.players[0].hand[0].handle();
    r.game.play_from_hand(0, 0).unwrap();
    let one = r.game.take_native_play_entries().pop().unwrap();
    assert_eq!(one.player(), 0);
    assert_eq!(one.card(), first);
    r.game.play_from_hand(0, 0).unwrap();
    let two = r.game.take_native_play_entries().pop().unwrap();
    assert_ne!(one.card(), two.card());
    let mut cloned = r.game.clone();
    cloned.players[0].battle_area.remove(0);
    assert!(cloned.resolve_native_play_entry(&one).is_none());
    assert_eq!(cloned.resolve_native_play_entry(&two).unwrap().index, 0);
    assert_eq!(r.game.resolve_native_play_entry(&two).unwrap().index, 1);
    let moved = cloned.players[0].battle_area.remove(0);
    cloned.players[1].battle_area.push(moved);
    let resolved = cloned.resolve_native_play_entry(&two).unwrap();
    assert_eq!((resolved.player, resolved.index), (1, 0));
}

#[test]
fn leaving_then_replaying_the_exact_card_never_revives_an_old_entry() {
    let mut r = setup();
    r.game.enable_native_play_observation(true);
    r.game.play_from_hand(0, 0).unwrap();
    let old = r.game.take_native_play_entries().pop().unwrap();
    let gone = r.game.players[0].battle_area.remove(0);
    r.game.players[0].hand.push(gone.top_card().clone());
    r.game.play_from_hand(0, 2).unwrap();
    let new = r.game.take_native_play_entries().pop().unwrap();
    assert_eq!(old.card(), new.card());
    assert!(r.game.resolve_native_play_entry(&old).is_none());
    assert!(r.game.resolve_native_play_entry(&new).is_some());
}

#[test]
fn an_ambiguous_duplicate_incarnation_is_refused() {
    let mut r = setup();
    r.game.enable_native_play_observation(true);
    r.game.play_from_hand(0, 0).unwrap();
    let entry = r.game.take_native_play_entries().pop().unwrap();
    // Deliberately malformed fixture: even duplicate identity cannot choose
    // whichever vector slot happens to occur first.
    let duplicate = r.game.players[0].battle_area[0].clone();
    r.game.players[0].battle_area.push(duplicate);
    assert!(r.game.resolve_native_play_entry(&entry).is_none());
}

#[test]
fn same_body_requires_original_top_but_restoring_that_top_preserves_identity() {
    let mut r = setup();
    r.game.enable_native_play_observation(true);
    r.game.play_from_hand(0, 0).unwrap();
    let entry = r.game.take_native_play_entries().pop().unwrap();
    let top = r.game.players[0].hand.remove(0);
    r.game.players[0].battle_area[0].card_sources.push(top);
    assert!(r.game.resolve_native_play_entry(&entry).is_none());
    r.game.players[0].battle_area[0].card_sources.pop();
    assert!(r.game.resolve_native_play_entry(&entry).is_some());
}

#[test]
fn failed_and_effect_initiated_plays_do_not_emit_native_entries() {
    let mut r = setup();
    r.game.enable_native_play_observation(true);
    assert!(r.game.play_from_hand(0, 99).is_none());
    r.game
        .play_from_hand_with_cost(0, 0, CostDelta::Free, PlaySource::ByEffect)
        .unwrap();
    assert!(r.game.take_native_play_entries().is_empty());
}

#[test]
fn observation_is_digest_and_debug_neutral_and_reset_revokes_entries() {
    let mut observed = setup();
    let mut control = observed.game.clone();
    observed.game.enable_native_play_observation(true);
    observed.game.play_from_hand(0, 0).unwrap();
    control.play_from_hand(0, 0).unwrap();
    assert_eq!(
        observed.game.verification_digest(),
        control.verification_digest()
    );
    assert_eq!(
        format!("{:?}", observed.game.players[0].battle_area),
        format!("{:?}", control.players[0].battle_area)
    );
    let entry = observed.game.take_native_play_entries().pop().unwrap();
    observed.game.enable_native_play_observation(false);
    assert!(observed.game.resolve_native_play_entry(&entry).is_none());
    observed.game.enable_native_play_observation(true);
    assert!(observed.game.resolve_native_play_entry(&entry).is_none());
    observed.game.reset_for_replay();
    assert!(observed.game.take_native_play_entries().is_empty());
    assert!(observed.game.resolve_native_play_entry(&entry).is_none());
}

#[test]
fn entry_is_captured_before_on_play_removes_its_body() {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(
            r#"
card: OBS-SELF-DELETE
name: Observed Self Delete
kind: digimon
color: [red]
level: 3
dp: 2000
cost: 0
effects:
  - when: on_play
    process:
      - delete_permanent: {target: source}
"#,
        )
        .unwrap()
        .hand(0, &["OBS-SELF-DELETE"])
        .memory(5)
        .start();
    r.game.current_phase = GamePhase::Main;
    let card = r.game.players[0].hand[0].handle();
    r.game.enable_native_play_observation(true);
    r.game.play_from_hand(0, 0);
    let entries = r.game.take_native_play_entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].card(), card);
    assert!(r.game.players[0].battle_area.is_empty());
    assert!(r.game.resolve_native_play_entry(&entries[0]).is_none());
}
