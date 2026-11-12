//! Mechanics tests for three DSL constructs added for MirageGaogamon: Burst
//! Mode (BT13-033) and Ravemon: Burst Mode (BT13-092), on synthetic cards:
//!
//! - `shuffle_hand`: reorders a hand without changing its contents (the
//!   "without looking" pick from an opponent's hand, BT13-033).
//! - `return_hand_list_to_deck`: the picked hand cards go to the bottom of
//!   their owner's deck (BT13-033).
//! - `name_matches_returned_card`: a filter matching permanents whose name is
//!   the name of the card the effect just returned (BT13-092).
use digimon_engine::{
    action::{build_action_mask, space::PLAY_HAND_START, PASS, TRASH_EFFECT_START},
    debug_runner::{make_test_card, DebugRunner},
    enums::GamePhase,
    selection::SelectionKind,
};

fn ids(r: &DebugRunner, cards: &[digimon_engine::card_source::CardSource]) -> Vec<String> {
    cards
        .iter()
        .map(|c| r.game.card_data[c.data_index].card_id.clone())
        .collect()
}

/// A Digimon whose [On Play] runs `process`, in player 0's hand on their main
/// phase, plus named filler cards.
fn fixture(process: &str, extra: &[(&str, &str)]) -> DebugRunner {
    let yaml = format!(
        r#"
card: CONSTRUCT-FIXTURE
name: Construct fixture
kind: digimon
color: [blue]
level: 3
cost: 0
dp: 2000
effects:
  - when: on_play
    process:
{process}
"#
    );
    let mut b = DebugRunner::builder().from_dsl_yaml(&yaml).unwrap();
    b = b.add_card(make_test_card("FILL", "Filler"));
    for (id, name) in extra {
        let mut c = make_test_card(id, name);
        c.dp = Some(3000);
        b = b.add_card(c);
    }
    let mut r = b
        .hand(0, &["CONSTRUCT-FIXTURE"])
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.game.current_phase = GamePhase::Main;
    r
}

fn play_fixture(r: &mut DebugRunner) {
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
}

#[test]
fn shuffle_hand_reorders_the_named_players_hand_only() {
    let names: Vec<(String, String)> =
        (0..10).map(|i| (format!("OPP-{i}"), format!("Opp {i}"))).collect();
    let extra: Vec<(&str, &str)> = names.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let mut r = fixture(
        r#"      - shuffle_hand:
          of: opponent"#,
        &extra,
    );
    let opp_ids: Vec<&str> = extra.iter().map(|(id, _)| *id).collect();
    r.game.players[1].hand.clear();
    for id in &opp_ids {
        r.add_to_hand(1, id);
    }
    r.add_to_hand(0, "FILL");
    let before = ids(&r, &r.game.players[1].hand);
    play_fixture(&mut r);
    let after = ids(&r, &r.game.players[1].hand);
    let mut sorted_before = before.clone();
    let mut sorted_after = after.clone();
    sorted_before.sort();
    sorted_after.sort();
    assert_eq!(sorted_after, sorted_before, "same cards, nothing moved out");
    assert_ne!(after, before, "the hand order changed");
    assert_eq!(ids(&r, &r.game.players[0].hand), vec!["FILL".to_string()], "own hand untouched");
}

#[test]
fn return_hand_list_to_deck_puts_the_picked_hand_cards_on_the_deck_bottom() {
    let mut r = fixture(
        r#"      - select_count_capped_multi:
          of: opponent
          zone: hand
          max: 2
          clamp_to_available: true
          filter: {}
          bind_as: picked
          prompt: "Pick 2 cards in your opponent's hand"
      - return_hand_list_to_deck:
          of: opponent
          cards: picked"#,
        &[("KEEP", "Keep"), ("BACK-A", "Back A"), ("BACK-B", "Back B")],
    );
    r.game.players[1].hand.clear();
    for id in ["KEEP", "BACK-A", "BACK-B"] {
        r.add_to_hand(1, id);
    }
    let deck_before = r.game.players[1].deck.len();
    play_fixture(&mut r);
    // Two picks (which two is the player's choice; the assertions follow
    // whichever cards were picked).
    for _ in 0..2 {
        let v = r.pending_selection_view().expect("hand pick");
        assert!(matches!(v.kind, SelectionKind::CountCappedMultiSelect { .. }), "{:?}", v.kind);
        assert_eq!(v.zone_owner, Some(1), "from the opponent's hand");
        let a = v
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .expect("a card to pick");
        r.execute_action(0, a).unwrap();
    }
    if let Some(v) = r.pending_selection_view() {
        if matches!(v.kind, SelectionKind::CountCappedMultiSelect { .. }) {
            r.execute_action(0, PASS).unwrap();
        }
    }
    assert_eq!(r.game.players[1].hand.len(), 1);
    assert_eq!(r.game.players[1].deck.len(), deck_before + 2);
    let mut bottom = ids(&r, &r.game.players[1].deck[..2]);
    bottom.sort();
    let remaining = ids(&r, &r.game.players[1].hand);
    let mut moved: Vec<String> = ["KEEP", "BACK-A", "BACK-B"]
        .iter()
        .map(|s| s.to_string())
        .filter(|id| !remaining.contains(id))
        .collect();
    moved.sort();
    assert_eq!(bottom, moved, "the two picked cards are on the bottom of the deck");
    assert!(r.game.players[1].trash.is_empty(), "returned, not discarded");
}

#[test]
fn name_matches_returned_card_selects_permanents_named_like_the_returned_card() {
    let mut r = fixture(
        r#"      - select_trash:
          of: opponent
          bind_as: returned
          optional: true
          cost: true
          filter: { kind: digimon }
          prompt: "Return 1 Digimon card from your opponent's trash to the bottom of their deck"
      - return_trash_list_to_deck_bottom:
          of: opponent
          cards: returned
      - delete_all_permanents:
          over:
            all_of:
              - of: opponent
              - kind: digimon
              - name_matches_returned_card: true"#,
        &[
            ("TWIN-A", "Twinmon"),
            ("TWIN-B", "Twinmon"),
            ("OTHER", "Othermon"),
            ("TWIN-TRASH", "Twinmon"),
        ],
    );
    r.place_on_field(1, "TWIN-A", None);
    r.place_on_field(1, "OTHER", None);
    r.place_on_field(1, "TWIN-B", None);
    r.inject_trash(1, "TWIN-TRASH");
    play_fixture(&mut r);
    let v = r.pending_selection_view().expect("trash pick");
    assert_eq!(v.kind, SelectionKind::Trash);
    assert_eq!(v.zone_owner, Some(1));
    r.execute_action(0, TRASH_EFFECT_START).unwrap();
    let field: Vec<String> = r.game.players[1]
        .battle_area
        .iter()
        .map(|p| r.game.card_data[p.top_card().data_index].card_id.clone())
        .collect();
    assert_eq!(field, vec!["OTHER".to_string()], "every Twinmon deleted, Othermon kept");
    assert_eq!(
        r.game.card_data[r.game.players[1].deck[0].data_index].card_id,
        "TWIN-TRASH",
        "the returned card is on the bottom of the deck"
    );
}
