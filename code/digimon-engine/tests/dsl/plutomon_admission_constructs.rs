//! Independent lowering proofs for the TS Plutomon admission gaps.
//! CR 4.3 15-16-2-1: On Play occurs after a completed play;
//! 15-16-7-1: Main is an activation-type effect. Card-text idioms:
//! BT24-007 (discard-event identity), BT24-023 / BT25-080 (effect play),
//! BT26-079 (trash-only Main). Fixtures deliberately omit unrelated clauses.
use digimon_engine::{
    action::{
        build_action_mask,
        space::{PLAY_HAND_START, TRASH_EFFECT_START},
    },
    card_source::CardSource,
    debug_runner::{make_test_card, DebugRunner},
    owned_card_choice::CardChoiceOperation,
};

const DISCARD: &str = r#"
card: PROOF-DISCARD
name: Discard two
kind: tamer
color: [white]
cost: 0
effects:
  - when: on_play
    process:
      - trash_opponent_hand_to_count: { opponent: you, target_count: 0 }
"#;

const OBSERVER: &str = r#"
card: PROOF-OBSERVER
name: Discard observer
kind: tamer
color: [white]
cost: 0
effects:
  - when: on_discard_hand
    trigger_if:
      event_discarded_card_any: { trait_has: Titan }
    process:
      - select_union_zone:
          of: you
          zones: [trash]
          bind_as: selected
          filter: { in_event_discarded_cards: true }
          prompt: Choose one of the physical cards just discarded
      - play_union_bound_free: { binding: selected, cost_delta: free }
"#;

const EFFECT_SUBJECT: &str = r#"
card: PROOF-SUBJECT
name: Effect play subject
kind: digimon
color: [purple]
level: 3
cost: 2
dp: 1000
effects:
  - when: on_play
    process:
      - if:
          condition: { played_by_effect: true }
          then: [{ gain_memory: 3 }]
          else: [{ gain_memory: 1 }]
"#;

const EFFECT_PLAYER: &str = r#"
card: PROOF-PLAYER
name: Effect player
kind: tamer
color: [white]
cost: 0
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: selected
          filter: { kind: digimon }
          prompt: Choose a Digimon to play
      - play_from_hand_free: { of: you, hand_index: selected }
"#;

const TRASH_MAIN: &str = r#"
card: PROOF-TRASH
name: Trash Main subject
kind: digimon
color: [purple]
level: 3
cost: 4
dp: 1000
effects:
  - scope: trash
    when: main_from_trash
    condition:
      all_of:
        - your_turn: true
        - count_lte: { filter: { of: you, zone: [hand] }, n: 1 }
    process:
      - play_from_trash: { of: you, hand_index: self, cost_delta: { reduce: 2 } }
"#;

fn setup(yamls: &[&str]) -> DebugRunner {
    let mut titan = make_test_card("PROOF-TITAN", "Titan");
    titan.traits = vec!["Titan".into()];
    let mut builder = DebugRunner::builder()
        .add_card(titan)
        .add_card(make_test_card("PROOF-PLAIN", "Plain"));
    for yaml in yamls {
        builder = builder.from_dsl_yaml(yaml).expect("valid independent YAML");
    }
    let mut r = builder
        .deck(0, &["PROOF-PLAIN"; 10])
        .deck(1, &["PROOF-PLAIN"; 10])
        .memory(5)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.game.enter_main_phase();
    r
}

fn put(r: &mut DebugRunner, id: &str, trash: bool) {
    let data = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == id)
        .unwrap();
    let card = CardSource::new(data, 0, r.game.next_card_index());
    if trash {
        r.game.players[0].trash.push(card);
    } else {
        r.game.players[0].hand.push(card);
    }
}

fn action(r: &mut DebugRunner, id: u16) {
    assert_eq!(build_action_mask(&r.game, 0)[id as usize], 1.0);
    if r.game.pending_selection.is_some() {
        r.execute_action(0, id).unwrap();
    } else {
        r.game.decode_action(id, 0);
    }
}

fn discard_batch(r: &mut DebugRunner) {
    action(r, PLAY_HAND_START);
    for remaining in [2, 1] {
        let pending = r.pending_selection_view().expect("hand-discard choice");
        let choices: Vec<_> = pending
            .valid_action_ids
            .into_iter()
            .filter(|id| *id != digimon_engine::action::space::PASS)
            .collect();
        assert_eq!(choices.len(), remaining);
        action(r, choices[0]);
    }
    assert!(r.game.players[0].hand.is_empty());
}

#[test]
fn discarded_event_membership_keeps_two_new_copies_and_excludes_old_identical_copy() {
    for choice in [0, 1] {
        let mut r = setup(&[DISCARD, OBSERVER]);
        r.place_on_field(0, "PROOF-OBSERVER", None);
        put(&mut r, "PROOF-TITAN", true);
        let old = r.game.players[0].trash[0].handle();
        put(&mut r, "PROOF-DISCARD", false);
        put(&mut r, "PROOF-TITAN", false);
        put(&mut r, "PROOF-TITAN", false);
        let fresh = [
            r.game.players[0].hand[1].handle(),
            r.game.players[0].hand[2].handle(),
        ];
        discard_batch(&mut r);
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("membership choice");
        let page = pending.card_choice.as_ref().unwrap().view(&r.game, 0);
        let rows: Vec<_> = page
            .rows
            .iter()
            .filter(|row| matches!(row.operation, CardChoiceOperation::CommitCard))
            .collect();
        assert_eq!(rows.len(), 2, "both fresh copies must be offered");
        assert_eq!(
            rows.iter()
                .map(|row| row.position.as_deref().unwrap())
                .collect::<Vec<_>>(),
            vec!["1", "2"]
        );
        let selected = rows[choice].action_id;
        action(&mut r, selected);
        assert!(r.game.pending_selection.is_none());
        assert!(r.game.players[0].trash.iter().any(|c| c.handle() == old));
        assert!(!r.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == fresh[choice]));
        assert!(r.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == fresh[1 - choice]));
        assert!(r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().handle() == fresh[choice]));
    }
}

#[test]
fn discarded_any_does_not_match_an_old_qualifying_trash_card() {
    let mut r = setup(&[DISCARD, OBSERVER]);
    r.place_on_field(0, "PROOF-OBSERVER", None);
    put(&mut r, "PROOF-TITAN", true);
    put(&mut r, "PROOF-DISCARD", false);
    put(&mut r, "PROOF-PLAIN", false);
    put(&mut r, "PROOF-PLAIN", false);
    discard_batch(&mut r);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.players[0].trash.len(), 3);
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    assert_eq!(r.memory(), 5);
}

#[test]
fn discarded_any_checks_later_batch_members_and_membership_is_not_a_trait_filter() {
    let mut r = setup(&[DISCARD, OBSERVER]);
    r.place_on_field(0, "PROOF-OBSERVER", None);
    put(&mut r, "PROOF-DISCARD", false);
    put(&mut r, "PROOF-PLAIN", false);
    put(&mut r, "PROOF-TITAN", false);
    let plain = r.game.players[0].hand[1].handle();
    discard_batch(&mut r);
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("later Titan triggers the observer");
    let page = pending.card_choice.as_ref().unwrap().view(&r.game, 0);
    let rows: Vec<_> = page
        .rows
        .iter()
        .filter(|row| matches!(row.operation, CardChoiceOperation::CommitCard))
        .collect();
    assert_eq!(
        rows.len(),
        2,
        "membership includes the non-Titan from the same batch"
    );
    let selected = rows
        .iter()
        .find(|row| row.position.as_deref() == Some("0"))
        .unwrap()
        .action_id;
    action(&mut r, selected);
    assert!(r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().handle() == plain));
    assert_eq!(r.game.players[0].trash.len(), 1);
    assert_eq!(
        r.game.players[0].trash[0].card_id(&r.game.card_data),
        "PROOF-TITAN"
    );
}

#[test]
fn played_by_effect_uses_completed_effect_play_provenance() {
    let mut r = setup(&[EFFECT_SUBJECT, EFFECT_PLAYER]);
    put(&mut r, "PROOF-PLAYER", false);
    put(&mut r, "PROOF-SUBJECT", false);
    action(&mut r, PLAY_HAND_START);
    let pending = r.pending_selection_view().expect("effect-play hand choice");
    assert_eq!(pending.valid_action_ids.len(), 1);
    action(&mut r, pending.valid_action_ids[0]);
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    assert!(r.game.players[0].hand.is_empty());
    assert_eq!(r.memory(), 8, "free effect play takes the true branch");
}

#[test]
fn played_by_effect_is_false_for_normal_paid_hand_play() {
    let mut r = setup(&[EFFECT_SUBJECT]);
    put(&mut r, "PROOF-SUBJECT", false);
    action(&mut r, PLAY_HAND_START);
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(r.memory(), 4, "pay 2 then false branch gains 1");
}

#[test]
fn main_from_trash_is_offered_and_pays_its_reduced_cost() {
    let mut r = setup(&[TRASH_MAIN]);
    put(&mut r, "PROOF-TRASH", true);
    action(&mut r, TRASH_EFFECT_START);
    assert!(r.game.players[0].trash.is_empty());
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(r.memory(), 3);
    assert_eq!(
        build_action_mask(&r.game, 0)[TRASH_EFFECT_START as usize],
        0.0
    );
}

#[test]
fn main_from_trash_rejects_unsatisfied_hand_condition() {
    let mut r = setup(&[TRASH_MAIN]);
    put(&mut r, "PROOF-TRASH", true);
    put(&mut r, "PROOF-PLAIN", false);
    put(&mut r, "PROOF-PLAIN", false);
    assert_eq!(
        build_action_mask(&r.game, 0)[TRASH_EFFECT_START as usize],
        0.0
    );
    assert!(!r.game.activate_trash_main(0, 0));
    assert_eq!(r.game.players[0].trash.len(), 1);
    assert!(r.game.players[0].battle_area.is_empty());
    assert_eq!(r.memory(), 5);
}

#[test]
fn main_from_trash_rejects_opponents_turn() {
    let mut r = setup(&[TRASH_MAIN]);
    put(&mut r, "PROOF-TRASH", true);
    r.game.turn_player_idx = 1;
    assert!(!r.game.activate_trash_main(0, 0));
    assert_eq!(r.game.players[0].trash.len(), 1);
    assert!(r.game.players[0].battle_area.is_empty());
    assert_eq!(r.memory(), 5);
}

#[test]
fn main_from_trash_does_not_fire_on_hand_play_or_field_on_play() {
    let mut r = setup(&[TRASH_MAIN]);
    put(&mut r, "PROOF-TRASH", false);
    action(&mut r, PLAY_HAND_START);
    assert_eq!(r.memory(), 1, "ordinary play pays full printed cost");
    r.fire_on_play(0, 0);
    assert_eq!(
        r.memory(),
        1,
        "adjacent On Play timing cannot activate trash Main"
    );
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert!(r.game.pending_selection.is_none());
}
