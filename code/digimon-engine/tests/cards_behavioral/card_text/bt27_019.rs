//! Source: `data/card_bundles/BT27-019.md` (printed text and official Q&A).
//!
//! Effect: [When Moving] [On Play] By placing 1 card from your hand face down
//! under any of your [Thomas H. Norstein]s, both players <Draw 1>.
//! Inherited Effect: [When Attacking] [Once Per Turn] Both players <Draw 1>.
//! Official Q&A: the card is placed on the bottom of the cards under the Tamer.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::GamePhase;
use digimon_engine::permanent::PermanentHandle;

use super::bt27_fx::*;
use super::common::candidates;

const CARD: &str = "BT27-019";
const THOMAS: &str = "THOMAS";

fn runner(hand: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(tamer(THOMAS, "Thomas H. Norstein", &["DATA SQUAD"]))
        .add_card(tamer("OTHER-T", "Masaru Daimon", &["DATA SQUAD"]))
        .add_card(mon("X", "X", 3, 3, &[]))
        .add_card(mon("OLD", "Old", 3, 3, &[]))
        .add_card(mon("TOP", "Top", 5, 7, &[]))
        .add_card(mon("FILL", "Fill", 3, 3, &[]))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .security(1, &["FILL"; 5])
        .hand(0, hand)
        .memory(10)
        .start()
}

fn thomas_with_card(r: &mut DebugRunner) -> PermanentHandle {
    let t = r.place_stack(0, &["OLD", THOMAS]);
    r.game.players[0].battle_area[t.index as usize].card_sources[0].face_down = true;
    t
}

#[test]
fn on_play_places_hand_card_face_down_at_bottom_of_thomas_and_both_draw() {
    let mut r = runner(&[CARD, "X"]);
    let t = thomas_with_card(&mut r);
    play_hand(&mut r, CARD);
    let v = r.pending_selection_view().expect("optional hand pick");
    assert!(v.is_optional);
    assert_eq!(candidates(&r), vec!["X"]);
    r.choose_prompt_card("X", None).unwrap();
    assert_eq!(candidates(&r), vec![THOMAS], "the player names the Thomas");
    r.choose_prompt_card(THOMAS, None).unwrap();
    no_prompt(&r);
    assert_eq!(source_ids(&r, t), vec!["X", "OLD"]);
    assert_eq!(face_down(&r, t), vec![true, true]);
    assert_eq!(hand_ids(&r, 0), vec!["FILL"]);
    assert_eq!(r.hand_size(1), 1);
}

#[test]
fn on_play_lets_the_player_choose_which_thomas() {
    let mut r = runner(&[CARD, "X"]);
    let t1 = thomas_with_card(&mut r);
    let t2 = r.place_on_field(0, THOMAS, Some(0));
    play_hand(&mut r, CARD);
    r.choose_prompt_card("X", None).unwrap();
    let v = r.pending_selection_view().expect("Thomas choice");
    assert!(!v.is_optional);
    assert_eq!(candidates(&r), vec![THOMAS, THOMAS]);
    r.choose_prompt_card(THOMAS, Some(1)).unwrap();
    no_prompt(&r);
    let (with_x, without) = if source_ids(&r, t2) == vec!["X"] { (t2, t1) } else { (t1, t2) };
    assert!(source_ids(&r, with_x).contains(&"X".to_string()));
    assert!(!source_ids(&r, without).contains(&"X".to_string()));
    assert_eq!(r.hand_size(1), 1);
}

#[test]
fn on_play_decline_draws_nothing() {
    let mut r = runner(&[CARD, "X"]);
    let t = thomas_with_card(&mut r);
    play_hand(&mut r, CARD);
    r.decline_prompt().unwrap();
    assert!(r.pending_selection().is_none());
    assert_eq!(source_ids(&r, t), vec!["OLD"]);
    assert_eq!(hand_ids(&r, 0), vec!["X"]);
    assert_eq!(r.hand_size(1), 0);
}

#[test]
fn on_play_needs_a_thomas_h_norstein() {
    let mut r = runner(&[CARD, "X"]);
    r.place_on_field(0, "OTHER-T", Some(0));
    play_hand(&mut r, CARD);
    assert!(r.pending_selection().is_none());
    assert_eq!(hand_ids(&r, 0), vec!["X"]);
    assert_eq!(r.hand_size(1), 0);
}

#[test]
fn on_play_needs_a_hand_card() {
    let mut r = runner(&[CARD]);
    let t = thomas_with_card(&mut r);
    play_hand(&mut r, CARD);
    assert!(r.pending_selection().is_none());
    assert_eq!(source_ids(&r, t), vec!["OLD"]);
    assert_eq!(r.hand_size(1), 0);
}

#[test]
fn when_moving_from_breeding_offers_the_same_placement() {
    let mut r = runner(&["X"]);
    let t = thomas_with_card(&mut r);
    r.place_in_breeding(0, CARD);
    r.set_phase(GamePhase::Breeding);
    assert!(r.move_from_breeding(0));
    let v = r.pending_selection_view().expect("When Moving hand pick");
    assert!(v.is_optional);
    r.choose_prompt_card("X", None).unwrap();
    assert_eq!(candidates(&r), vec![THOMAS], "the player names the Thomas");
    r.choose_prompt_card(THOMAS, None).unwrap();
    no_prompt(&r);
    assert_eq!(source_ids(&r, t), vec!["X", "OLD"]);
    assert_eq!(r.hand_size(1), 1);
}

#[test]
fn inherited_when_attacking_both_players_draw_once_per_turn() {
    let mut r = runner(&[]);
    let a = r.place_stack(0, &[CARD, "TOP"]).index;
    attack_security(&mut r, a);
    no_prompt(&r);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.hand_size(1), 1);
    attack_security(&mut r, a);
    no_prompt(&r);
    assert_eq!(r.hand_size(0), 1, "once per turn");
    assert_eq!(r.hand_size(1), 1);
}
