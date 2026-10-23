//! Event-color card-text idiom: BT20-023 Coredramon (green played Digimon),
//! EX13-002 DemiVeemon (blue played Tamer). Images inspected 2026-10-22.
//! CR 4.3 §§15-5-2/3: the event subject is distinct from the observing source.
//! Also exercise the deleted-object snapshot after battle-area compaction.

use digimon_engine::action::{build_action_mask, space::PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const PLAYED_COLOR: &str = r#"
card: TST-EVENT-COLOR
name: Blue observer
kind: tamer
color: [blue]
cost: 1
effects:
  - when: on_ally_played
    trigger_if: { event_target_color_any_of: [blue, green] }
    process:
      - select_own_permanent:
          bind_as: selected
          filter: { kind: digimon, trait_has: Target }
          optional: true
          prompt: Choose a target
      - suspend: { target: selected }
"#;

const DELETED_COLOR: &str = r#"
card: TST-EVENT-COLOR
name: Blue observer
kind: tamer
color: [blue]
cost: 1
effects:
  - when: on_any_deletion
    trigger_if: { event_target_color_any_of: [blue, green] }
    process:
      - gain_memory: 1
"#;

fn runner(yaml: &str, event_colors: Vec<CardColor>, kind: CardKind) -> DebugRunner {
    let mut event = make_test_card("EVENT", "Event subject");
    event.colors = event_colors;
    event.card_kind = kind;
    event.play_cost = 1;
    if kind == CardKind::Tamer {
        event.level = None;
        event.dp = None;
    }
    let mut target = make_test_card("TARGET", "Target");
    target.traits = vec!["Target".into()];
    target.dp = Some(4000);
    target.colors = vec![CardColor::Red];
    let mut red = make_test_card("RED", "Red neighbor");
    red.colors = vec![CardColor::Red];
    let mut blue = make_test_card("BLUE", "Blue neighbor");
    blue.colors = vec![CardColor::Blue];
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(event)
        .add_card(target)
        .add_card(red)
        .add_card(blue)
        .hand(0, &["EVENT"])
        .deck(0, &["RED"; 15])
        .deck(1, &["RED"; 15])
        .memory(8)
        .start();
    r.skip_mulligan();
    r.game.enter_main_phase();
    r.place_on_field(0, "TST-EVENT-COLOR", Some(0));
    r
}

fn assert_choice(r: &DebugRunner) {
    let s = r
        .game
        .pending_selection
        .as_ref()
        .expect("matching event offers choice");
    assert_eq!(s.kind, SelectionKind::OwnField);
    assert_eq!(s.selecting_player, 0);
    assert!(s.is_optional);
    let mut ids = s.valid_action_ids.clone();
    ids.sort();
    assert_eq!(ids, vec![101, 102]);
    let mask = build_action_mask(&r.game, 0);
    for (i, value) in mask.iter().enumerate() {
        assert_eq!(
            *value,
            if [101, 102, PASS].contains(&(i as u16)) {
                1.0
            } else {
                0.0
            },
            "action {i}"
        );
    }
    assert!(build_action_mask(&r.game, 1).iter().all(|v| *v == 0.0));
}

#[test]
fn played_event_color_intersection_uses_event_subject_and_exposes_choices() {
    for colors in [
        vec![CardColor::Blue],
        vec![CardColor::Green],
        vec![CardColor::Red, CardColor::Blue],
    ] {
        for kind in [CardKind::Digimon, CardKind::Tamer] {
            let mut r = runner(PLAYED_COLOR, colors.clone(), kind);
            let first = r.place_on_field(0, "TARGET", Some(0));
            let second = r.place_on_field(0, "TARGET", Some(0));
            let opponent = r.place_on_field(1, "TARGET", Some(0));
            assert_eq!(build_action_mask(&r.game, 0)[0], 1.0);
            r.game.decode_action(0, 0);
            assert_choice(&r);
            r.execute_action(0, 102).unwrap();
            assert!(
                !r.game.players[first.player as usize].battle_area[first.index as usize]
                    .is_suspended
            );
            assert!(
                r.game.players[second.player as usize].battle_area[second.index as usize]
                    .is_suspended
            );
            assert!(
                !r.game.players[opponent.player as usize].battle_area[opponent.index as usize]
                    .is_suspended
            );
            assert_eq!(r.memory(), 7);
            assert!(r.game.pending_selection.is_none());
        }
    }
}

#[test]
fn played_event_without_requested_color_does_not_borrow_observer_color() {
    for colors in [
        vec![CardColor::Red],
        vec![CardColor::Red, CardColor::Yellow],
        vec![],
    ] {
        let mut r = runner(PLAYED_COLOR, colors, CardKind::Digimon);
        let first = r.place_on_field(0, "TARGET", Some(0));
        let second = r.place_on_field(0, "TARGET", Some(0));
        r.game.decode_action(0, 0);
        assert!(r.game.pending_selection.is_none());
        assert!(
            !r.game.players[first.player as usize].battle_area[first.index as usize].is_suspended
        );
        assert!(
            !r.game.players[second.player as usize].battle_area[second.index as usize].is_suspended
        );
        assert_eq!(r.memory(), 7);
    }
}

#[test]
fn matching_played_event_choice_can_be_declined() {
    let mut r = runner(PLAYED_COLOR, vec![CardColor::Blue], CardKind::Digimon);
    let first = r.place_on_field(0, "TARGET", Some(0));
    let second = r.place_on_field(0, "TARGET", Some(0));
    r.game.decode_action(0, 0);
    assert_choice(&r);
    r.execute_action(0, PASS).unwrap();
    assert!(!r.game.players[first.player as usize].battle_area[first.index as usize].is_suspended);
    assert!(
        !r.game.players[second.player as usize].battle_area[second.index as usize].is_suspended
    );
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn deleted_event_color_reads_snapshot_not_compacted_neighbor() {
    for (colors, neighbor, expected) in [
        (vec![CardColor::Blue], "RED", 1),
        (vec![CardColor::Red, CardColor::Green], "RED", 1),
        (vec![CardColor::Red], "BLUE", 0),
    ] {
        for owner in [0, 1] {
            let mut r = runner(DELETED_COLOR, colors.clone(), CardKind::Digimon);
            let victim = r.place_on_field(owner, "EVENT", Some(0));
            let neighbor_handle = r.place_on_field(owner, neighbor, Some(0));
            let neighbor_card = r.top_card(neighbor_handle);
            r.game
                .delete_permanents_batch(vec![victim], ReplacementCause::OpponentEffect);
            r.game.drain_effect_queue();
            assert_eq!(r.memory(), 8 + expected);
            assert_eq!(
                r.game
                    .player(owner)
                    .battle_area
                    .last()
                    .unwrap()
                    .top_card()
                    .handle(),
                neighbor_card
            );
            assert!(r.game.pending_selection.is_none());
        }
    }
}

#[test]
fn event_color_observer_does_not_fire_at_adjacent_on_play_timing() {
    let mut r = runner(PLAYED_COLOR, vec![CardColor::Blue], CardKind::Digimon);
    r.place_on_field(0, "TARGET", Some(0));
    r.place_on_field(0, "TARGET", Some(0));
    r.fire_on_play(0, 0);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.memory(), 8);
}
