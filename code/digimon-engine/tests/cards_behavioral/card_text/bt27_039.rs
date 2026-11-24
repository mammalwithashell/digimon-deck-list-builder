//! Source: `data/card_bundles/BT27-039.md` (printed text and official Q&A).
//!
//! <Barrier> [When Attacking] [Once Per Turn] You may play 1 Digimon card with
//! [Chessmon] in its name from your hand with the cost reduced by 2.
//! Inherited: [When Attacking] [Once Per Turn] 1 of your opponent's Digimon
//! gets -2000 DP for the turn.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;

use super::bt27_fx::*;
use super::common::candidates;

const CARD: &str = "BT27-039";

fn runner(hand: &[&str], memory: i16) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(mon("PAWN", "PawnChessmon", 3, 3, &[]))
        .add_card(mon("ROOK", "RookChessmon", 6, 9, &[]))
        .add_card(mon("AGU", "Agumon", 3, 3, &[]))
        .add_card(mon("TOP", "Top", 5, 7, &[]))
        .add_card(mon("FILL", "Fill", 3, 3, &[]))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .security(1, &["FILL"; 5])
        .hand(0, hand)
        .memory(memory)
        .start()
}

#[test]
fn has_barrier() {
    let mut r = runner(&[], 5);
    let h = r.place_on_field(0, CARD, Some(0));
    assert!(r.game.has_keyword(h, Keyword::Barrier));
}

#[test]
fn when_attacking_plays_a_chessmon_from_hand_with_cost_reduced_by_two() {
    let mut r = runner(&["PAWN", "AGU"], 5);
    let a = r.place_on_field(0, CARD, Some(0)).index;
    attack_security(&mut r, a);
    let v = r.pending_selection_view().expect("optional play");
    assert!(v.is_optional);
    assert_eq!(candidates(&r), vec!["PAWN"]);
    let before = r.memory();
    r.choose_prompt_card("PAWN", None).unwrap();
    assert!(field_ids(&r, 0).contains(&"PAWN".to_string()));
    assert_eq!(before - r.memory(), 1);
}

#[test]
fn play_may_be_declined_and_is_once_per_turn() {
    let mut r = runner(&["PAWN"], 5);
    let a = r.place_on_field(0, CARD, Some(0)).index;
    attack_security(&mut r, a);
    r.decline_prompt().unwrap();
    no_prompt(&r);
    assert_eq!(hand_ids(&r, 0), vec!["PAWN"]);
    attack_security(&mut r, a);
    no_prompt(&r);
    assert_eq!(hand_ids(&r, 0), vec!["PAWN"], "once per turn");
}

#[test]
fn inherited_gives_minus_2000_dp_once_per_turn() {
    let mut r = runner(&[], 5);
    let a = r.place_stack(0, &[CARD, "TOP"]).index;
    let victim = r.place_on_field(1, "TOP", Some(0));
    let base = r.dp_of(victim).unwrap();
    attack_security(&mut r, a);
    r.choose_prompt_card("TOP", None).unwrap();
    no_prompt(&r);
    assert_eq!(r.dp_of(victim), Some(base - 2000));
    attack_security(&mut r, a);
    no_prompt(&r);
    assert_eq!(r.dp_of(victim), Some(base - 2000), "once per turn");
}
