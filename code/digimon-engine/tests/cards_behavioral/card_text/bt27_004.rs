//! Source: `data/card_bundles/BT27-004.md` (printed text and official Q&A).
//!
//! Inherited Effect:
//! [Your Turn] [Once Per Turn] When any of your other Digimon with [Chessmon]
//! in their names are played, this Digimon may digivolve into a Digimon card
//! with [Chessmon] in its name in the hand with the cost reduced by 2.

use digimon_engine::debug_runner::DebugRunner;

use super::bt27_fx::*;
use super::common::candidates;

const CARD: &str = "BT27-004";

fn runner(hand: &[&str]) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(mon("CARRIER", "Carrier", 3, 3, &[]))
        .add_card(mon("PAWN", "PawnChessmon", 3, 3, &[]))
        .add_card(mon("PAWN2", "PawnChessmon", 3, 3, &[]))
        .add_card(mon("PLAIN", "Agumon", 3, 3, &[]))
        .add_card(evo("KNIGHT", "KnightChessmon", 4, 6, 3))
        .add_card(evo("GREY", "Greymon", 4, 6, 3))
        .add_card(evo("KING", "KingChessmon", 4, 6, 20))
        .add_card(mon("FILL", "Fill", 3, 3, &[]))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .hand(0, hand)
        .memory(10)
        .start();
    let carrier = r.place_stack(0, &[CARD, "CARRIER"]);
    (r, carrier)
}

#[test]
fn chessmon_play_offers_only_affordable_chessmon_named_hand_digimon() {
    let (mut r, _) = runner(&["PAWN", "KNIGHT", "GREY", "KING"]);
    play_hand(&mut r, "PAWN");
    let v = r.pending_selection_view().expect("optional digivolve pick");
    assert!(v.is_optional);
    assert_eq!(candidates(&r), vec!["KNIGHT"]);
}

#[test]
fn digivolves_with_cost_reduced_by_two() {
    let (mut r, carrier) = runner(&["PAWN", "KNIGHT"]);
    play_hand(&mut r, "PAWN");
    let before = r.memory();
    r.choose_prompt_card("KNIGHT", None).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, carrier), "KNIGHT");
    assert_eq!(before - r.memory(), 1, "digivolve cost 3 reduced by 2");
}

#[test]
fn digivolve_may_be_declined() {
    let (mut r, carrier) = runner(&["PAWN", "KNIGHT"]);
    play_hand(&mut r, "PAWN");
    r.decline_prompt().unwrap();
    assert!(r.pending_selection().is_none());
    assert_eq!(top_id(&r, carrier), "CARRIER");
    assert!(hand_ids(&r, 0).contains(&"KNIGHT".to_string()));
}

#[test]
fn non_chessmon_play_does_not_trigger() {
    let (mut r, carrier) = runner(&["PLAIN", "KNIGHT"]);
    play_hand(&mut r, "PLAIN");
    assert!(r.pending_selection().is_none());
    assert_eq!(top_id(&r, carrier), "CARRIER");
}

#[test]
fn is_once_per_turn() {
    let (mut r, carrier) = runner(&["PAWN", "PAWN2", "KNIGHT"]);
    play_hand(&mut r, "PAWN");
    r.decline_prompt().unwrap();
    play_hand(&mut r, "PAWN2");
    assert!(r.pending_selection().is_none(), "already used this turn");
    assert_eq!(top_id(&r, carrier), "CARRIER");
}

#[test]
fn opponent_chessmon_play_does_not_trigger() {
    let (mut r, carrier) = runner(&["KNIGHT"]);
    r.pass_turn();
    r.game.enter_main_phase();
    assert_eq!(r.turn_player(), 1);
    let i = r.add_to_hand(1, "PAWN");
    let _ = r.play(1, i);
    assert!(r
        .pending_selection_view()
        .map_or(true, |v| v.selecting_player != 0));
    assert_eq!(top_id(&r, carrier), "CARRIER");
}
