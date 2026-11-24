//! Source: `data/card_bundles/BT27-081.md` (printed text and official Q&A).
//!
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] If your
//! opponent's hand has 6 or more cards, they trash 1 card in their hand. Then,
//! by trashing 2 bottom face-down cards from under any of your Tamers, you may
//! play or use 1 [DATA SQUAD] trait card with a play or use cost of 5 or less
//! from your hand without paying the cost.
//! Inherited: [On Deletion] You may play 1 play cost 5 or lower card with
//! [Avian] or [Bird] in any of its traits or the [DATA SQUAD] trait from your
//! trash without paying the cost.
//! Q&A: with 5 or fewer opposing hand cards the Then part still resolves.

use digimon_engine::debug_runner::DebugRunner;

use super::bt27_fx::*;
use super::common::candidates;

const CARD: &str = "BT27-081";

fn runner(hand: &[&str], opp_hand: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(tamer("T", "Tamer", &["DATA SQUAD"]))
        .add_card(mon("SQ5", "Squad", 3, 5, &["DATA SQUAD"]))
        .add_card(mon("SQ6", "Squad6", 3, 6, &["DATA SQUAD"]))
        .add_card(mon("BIRD", "Bird", 3, 4, &["Mysterious Bird"]))
        .add_card(mon("AVIAN", "Avian", 3, 5, &["Avian"]))
        .add_card(mon("PLAIN", "Plain", 3, 3, &[]))
        .add_card(mon("TOP", "Top", 6, 9, &[]))
        .add_card(mon("O1", "O1", 3, 3, &[]))
        .add_card(mon("O2", "O2", 3, 3, &[]))
        .add_card(mon("FD", "Fd", 3, 3, &[]))
        .add_card(wall())
        .add_card(mon("FILL", "Fill", 3, 3, &[]))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .security(1, &["FILL"; 5])
        .hand(0, hand)
        .hand(1, opp_hand)
        .memory(8)
        .start()
}

#[test]
fn six_opposing_hand_cards_makes_them_trash_one_then_pays_to_play_data_squad_free() {
    let mut r = runner(&[CARD, "SQ5", "SQ6"], &["O1", "O2", "O2", "O2", "O2", "O2"]);
    let t = stocked(&mut r, "T", &["FD", "FD"]);
    play_hand(&mut r, CARD);
    let v = r.pending_selection_view().expect("opponent's hand trash");
    assert_eq!(v.selecting_player, 1);
    assert!(!v.is_optional);
    r.choose_prompt_card("O1", None).unwrap();
    assert_eq!(trash_ids(&r, 1), vec!["O1"]);
    choose_branch(&mut r, 0);
    pay_tamer_stock(&mut r);
    assert!(source_ids(&r, t).is_empty());
    assert_eq!(candidates(&r), vec!["SQ5"]);
    let mem = r.memory();
    r.choose_prompt_card("SQ5", None).unwrap();
    no_prompt(&r);
    assert!(field_ids(&r, 0).contains(&"SQ5".to_string()));
    assert_eq!(r.memory(), mem);
}

#[test]
fn five_opposing_hand_cards_skips_the_trash_but_the_then_part_still_resolves() {
    let mut r = runner(&[CARD, "SQ5"], &["O2"; 5]);
    stocked(&mut r, "T", &["FD", "FD"]);
    play_hand(&mut r, CARD);
    let v = r.pending_selection_view().expect("cost choice");
    assert_eq!(v.selecting_player, 0);
    assert_eq!(v.effect_choices.as_ref().map(|c| c.len()), Some(2));
    assert_eq!(r.hand_size(1), 5);
    choose_branch(&mut r, 1);
    no_prompt(&r);
    assert_eq!(hand_ids(&r, 0), vec!["SQ5"]);
}

#[test]
fn no_face_down_cards_means_no_then_cost() {
    let mut r = runner(&[CARD, "SQ5"], &[]);
    stocked(&mut r, "T", &["FD"]);
    play_hand(&mut r, CARD);
    no_prompt(&r);
    assert!(r.pending_selection().is_none());
}

#[test]
fn when_attacking_fires_once_per_turn() {
    let mut r = runner(&[], &["O2"; 6]);
    let me = r.place_on_field(0, CARD, Some(0)).index;
    attack_security(&mut r, me);
    let v = r.pending_selection_view().expect("When Attacking hand trash");
    assert_eq!(v.selecting_player, 1);
    r.execute_action(1, v.valid_action_ids[0]).unwrap();
    no_prompt(&r);
    assert_eq!(r.hand_size(1), 5);
    r.game.players[0].battle_area[me as usize].is_suspended = false;
    attack_security(&mut r, me);
    no_prompt(&r);
    assert_eq!(r.hand_size(1), 5, "Once Per Turn");
}

#[test]
fn inherited_on_deletion_plays_an_avian_bird_or_data_squad_card_from_trash_free() {
    let mut r = runner(&[], &[]);
    for id in ["BIRD", "AVIAN", "SQ5", "SQ6", "PLAIN"] {
        r.inject_trash(0, id);
    }
    let me = r.place_stack(0, &[CARD, "TOP"]);
    lose_battle(&mut r, me.index);
    let v = r.pending_selection_view().expect("optional revive");
    assert!(v.is_optional);
    let mut c = candidates(&r);
    c.sort();
    assert_eq!(c, vec!["AVIAN", "BIRD", "SQ5"], "cost 6, untraited and Crowmon itself excluded");
    let mem = r.memory();
    r.choose_prompt_card("BIRD", None).unwrap();
    no_prompt(&r);
    assert!(field_ids(&r, 0).contains(&"BIRD".to_string()));
    assert_eq!(r.memory(), mem);
}

#[test]
fn inherited_on_deletion_may_be_declined() {
    let mut r = runner(&[], &[]);
    r.inject_trash(0, "SQ5");
    let me = r.place_stack(0, &[CARD, "TOP"]);
    lose_battle(&mut r, me.index);
    r.decline_prompt().unwrap();
    no_prompt(&r);
    assert!(trash_ids(&r, 0).contains(&"SQ5".to_string()));
}
