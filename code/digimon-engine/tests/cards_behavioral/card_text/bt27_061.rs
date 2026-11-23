//! Source: `data/card_bundles/BT27-061.md` (printed text).
//!
//! <Blocker> [On Deletion] You may play 1 play cost 4 or lower Digimon card
//! with [Chessmon] in its name from your hand without paying the cost.
//! Inherited: [On Deletion] <De-Digivolve 1> 1 of your opponent's Digimon.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;

use super::bt27_fx::*;
use super::common::candidates;

const CARD: &str = "BT27-061";

fn runner(hand: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(mon("PAWN", "PawnChessmon", 3, 3, &[]))
        .add_card(mon("BISHOP", "BishopChessmon", 4, 4, &[]))
        .add_card(mon("ROOK", "RookChessmon", 5, 5, &[]))
        .add_card(mon("AGU", "Agumon", 3, 3, &[]))
        .add_card(mon("A", "A", 3, 3, &[]))
        .add_card(mon("B", "B", 4, 5, &[]))
        .add_card(mon("TOP", "Top", 5, 7, &[]))
        .add_card(wall())
        .add_card(mon("FILL", "Fill", 3, 3, &[]))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .hand(0, hand)
        .memory(3)
        .start()
}

#[test]
fn has_blocker() {
    let mut r = runner(&[]);
    let h = r.place_on_field(0, CARD, Some(0));
    assert!(r.game.has_keyword(h, Keyword::Blocker));
}

#[test]
fn on_deletion_plays_a_cost_four_or_lower_chessmon_free() {
    let mut r = runner(&["PAWN", "BISHOP", "ROOK", "AGU"]);
    let a = r.place_on_field(0, CARD, Some(0)).index;
    lose_battle(&mut r, a);
    let v = r.pending_selection_view().expect("optional play");
    assert!(v.is_optional);
    assert_eq!(candidates(&r), vec!["BISHOP", "PAWN"]);
    let mem = r.memory();
    r.choose_prompt_card("BISHOP", None).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["BISHOP"]);
    assert_eq!(r.memory(), mem);
}

#[test]
fn on_deletion_play_may_be_declined() {
    let mut r = runner(&["PAWN"]);
    let a = r.place_on_field(0, CARD, Some(0)).index;
    lose_battle(&mut r, a);
    r.decline_prompt().unwrap();
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).is_empty());
    assert_eq!(hand_ids(&r, 0), vec!["PAWN"]);
}

#[test]
fn inherited_on_deletion_de_digivolves_one_opposing_digimon() {
    let mut r = runner(&[]);
    let a = r.place_stack(0, &[CARD, "TOP"]).index;
    let victim = r.place_stack(1, &["A", "B", "TOP"]);
    lose_battle(&mut r, a);
    let v = r.pending_selection_view().expect("De-Digivolve pick");
    assert!(!v.is_optional);
    r.choose_prompt_card("TOP", None).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, victim), "B");
}
