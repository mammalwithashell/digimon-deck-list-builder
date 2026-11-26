//! Source: `data/card_bundles/BT27-057.md` (printed text and official Q&A).
//!
//! [Start of Your Main Phase] By trashing 1 card with [Chessmon] in its text
//! from your hand, <Draw 1> and gain 1 memory.
//! Inherited: [On Deletion] <De-Digivolve 1> 1 of your opponent's Digimon.

use digimon_engine::debug_runner::DebugRunner;

use super::bt27_fx::*;
use super::common::{advance_turn, candidates};

const CARD: &str = "BT27-057";

fn runner(hand: &[&str]) -> DebugRunner {
    let mut text = mon("TEXT-CHESS", "Mystery", 3, 3, &[]);
    text.effect_text = "[On Play] Play 1 [Chessmon].".into();
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(mon("PAWN", "PawnChessmon", 3, 3, &[]))
        .add_card(text)
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
fn start_of_main_trashes_a_chessmon_text_card_to_draw_and_gain_memory() {
    let mut r = runner(&["TEXT-CHESS", "PAWN", "AGU"]);
    r.place_on_field(0, CARD, Some(0));
    advance_turn(&mut r);
    advance_turn(&mut r);
    let v = r.pending_selection_view().expect("optional trash");
    assert!(v.is_optional);
    assert_eq!(candidates(&r), vec!["PAWN", "TEXT-CHESS"]);
    let hand = r.hand_size(0);
    let mem = r.memory();
    r.choose_prompt_card("TEXT-CHESS", None).unwrap();
    no_prompt(&r);
    assert!(trash_ids(&r, 0).contains(&"TEXT-CHESS".to_string()));
    assert_eq!(r.hand_size(0), hand);
    assert_eq!(r.memory(), mem + 1);
}

#[test]
fn start_of_main_may_be_declined() {
    let mut r = runner(&["PAWN"]);
    r.place_on_field(0, CARD, Some(0));
    advance_turn(&mut r);
    advance_turn(&mut r);
    let mem = r.memory();
    r.decline_prompt().unwrap();
    assert!(hand_ids(&r, 0).contains(&"PAWN".to_string()));
    assert_eq!(r.memory(), mem);
}

#[test]
fn start_of_main_needs_a_chessmon_text_card() {
    let mut r = runner(&["AGU"]);
    r.place_on_field(0, CARD, Some(0));
    advance_turn(&mut r);
    advance_turn(&mut r);
    assert!(r.pending_selection().is_none());
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
    no_prompt(&r);
    assert_eq!(top_id(&r, victim), "B");
}
