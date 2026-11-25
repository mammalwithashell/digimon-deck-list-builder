//! Source: `data/card_bundles/BT27-024.md` (printed text).
//!
//! <Jamming> [On Play] [When Digivolving] For every 4 cards your opponent's
//! hand has, trash any 2 digivolution cards from their Digimon. Then, 1 of
//! their Digimon with no digivolution cards can't suspend until their turn ends.
//! Inherited: [When Attacking] [Once Per Turn] Both players <Draw 1>.
//! Special digivolve: Lv.3 w/[DATA SQUAD] trait: Cost 2.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;
use digimon_engine::enums::ModifierType;

use super::bt27_fx::*;
use super::common::candidates;

const CARD: &str = "BT27-024";

fn runner(opp_hand: usize) -> DebugRunner {
    let mut green = mon("GREEN-DS3", "Green", 3, 3, &["DATA SQUAD"]);
    green.colors = vec![digimon_engine::enums::CardColor::Green];
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(green)
        .add_card(mon("A", "A", 3, 3, &[]))
        .add_card(mon("B", "B", 4, 5, &[]))
        .add_card(mon("C", "C", 3, 3, &[]))
        .add_card(mon("TOP", "Top", 5, 7, &[]))
        .add_card(mon("BARE", "Bare", 5, 7, &[]))
        .add_card(mon("FILL", "Fill", 3, 3, &[]))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .security(1, &["FILL"; 5])
        .hand(0, &[CARD])
        .hand(1, &vec!["FILL"; opp_hand])
        .memory(10)
        .start()
}

#[test]
fn has_jamming() {
    let mut r = runner(0);
    let h = r.place_on_field(0, CARD, Some(0));
    assert!(r.game.has_keyword(h, Keyword::Jamming));
}

#[test]
fn digivolves_from_any_colour_level_three_data_squad_for_two() {
    let mut r = runner(0);
    let base = r.place_on_field(0, "GREEN-DS3", Some(0));
    let before = r.memory();
    assert!(digivolve_hand(&mut r, CARD, base));
    no_prompt(&r);
    assert_eq!(top_id(&r, base), CARD);
    assert_eq!(before - r.memory(), 2);
}

#[test]
fn trashes_two_chosen_sources_per_four_opponent_hand_cards_across_their_digimon() {
    let mut r = runner(8);
    let x = r.place_stack(1, &["A", "B", "TOP"]);
    let y = r.place_stack(1, &["C", "TOP"]);
    r.place_on_field(1, "BARE", Some(0));
    play_hand(&mut r, CARD);
    let v = r.pending_selection_view().expect("source pick");
    assert!(!v.is_optional);
    for id in ["A", "B", "C"] {
        assert!(r.prompt_card_candidates().iter().any(|(_, c)| c == id), "{id} pickable");
    }
    // 8 cards -> 4 picks, but only 3 sources exist: all are trashed.
    while r.pending_selection().is_some()
        && candidates(&r).iter().any(|c| ["A", "B", "C"].contains(&c.as_str()))
    {
        let id = candidates(&r).into_iter().find(|c| ["A", "B", "C"].contains(&c.as_str())).unwrap();
        r.choose_prompt_card(&id, None).unwrap();
    }
    assert!(source_ids(&r, x).is_empty());
    assert!(source_ids(&r, y).is_empty());
}

#[test]
fn four_cards_trash_exactly_two_of_the_players_choice() {
    let mut r = runner(4);
    let x = r.place_stack(1, &["A", "B", "TOP"]);
    let y = r.place_stack(1, &["C", "TOP"]);
    play_hand(&mut r, CARD);
    r.choose_prompt_card("A", None).unwrap();
    r.choose_prompt_card("C", None).unwrap();
    // Lock pick: Y is now sourceless.
    assert_eq!(candidates(&r), vec!["TOP"]);
    r.choose_prompt_card("TOP", None).unwrap();
    no_prompt(&r);
    assert_eq!(source_ids(&r, x), vec!["B"]);
    assert!(source_ids(&r, y).is_empty());
    assert!(r.game.modifiers.has(y, ModifierType::CannotSuspend));
    assert!(!r.game.modifiers.has(x, ModifierType::CannotSuspend));
}

#[test]
fn under_four_cards_trashes_nothing_but_still_locks_a_sourceless_digimon() {
    let mut r = runner(3);
    let x = r.place_stack(1, &["A", "TOP"]);
    let bare = r.place_on_field(1, "BARE", Some(0));
    play_hand(&mut r, CARD);
    let v = r.pending_selection_view().expect("lock pick");
    assert!(!v.is_optional);
    assert_eq!(candidates(&r), vec!["BARE"]);
    r.choose_prompt_card("BARE", None).unwrap();
    no_prompt(&r);
    assert_eq!(source_ids(&r, x), vec!["A"]);
    assert!(r.game.modifiers.has(bare, ModifierType::CannotSuspend));
}

#[test]
fn no_sourceless_digimon_means_no_lock() {
    let mut r = runner(0);
    let x = r.place_stack(1, &["A", "TOP"]);
    play_hand(&mut r, CARD);
    no_prompt(&r);
    assert!(r.pending_selection().is_none());
    assert!(!r.game.modifiers.has(x, ModifierType::CannotSuspend));
}

#[test]
fn inherited_both_players_draw_once_per_turn() {
    let mut r = runner(0);
    let a = r.place_stack(0, &[CARD, "TOP"]).index;
    attack_security(&mut r, a);
    no_prompt(&r);
    assert_eq!((r.hand_size(0), r.hand_size(1)), (2, 1));
    attack_security(&mut r, a);
    no_prompt(&r);
    assert_eq!((r.hand_size(0), r.hand_size(1)), (2, 1));
}
