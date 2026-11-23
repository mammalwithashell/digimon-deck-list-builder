//! Source: `data/card_bundles/BT27-036.md` (printed text and official Q&A).
//!
//! Effect: [On Play] Reveal the top 3 cards of your deck. Add 1 card with
//! [Chessmon] in its text and 1 [DATA SQUAD] trait card among them to the
//! hand. Return the rest to the bottom of the deck.
//! Inherited Effect: [When Attacking] [Once Per Turn] 1 of your opponent's
//! Digimon gets -2000 DP for the turn.
//! Official Q&A: "[Chessmon] in its text" includes names, traits, effects and
//! other text. Ruling Q4704 (BT3-093): a card matching both categories may be
//! taken for the first pick even when that leaves none for the second.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::DebugRunner;

use super::bt27_fx::*;

const CARD: &str = "BT27-036";

fn runner(deck: &[&str]) -> DebugRunner {
    let mut text = mon("TEXT-CHESS", "Mystery", 3, 3, &[]);
    text.effect_text = "[On Play] Play 1 [Chessmon] from your hand.".into();
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(mon("CHESS", "KnightChessmon", 4, 5, &[]))
        .add_card(mon("DS", "Squad", 4, 5, &["DATA SQUAD"]))
        .add_card(mon("BOTH", "RookChessmon", 4, 5, &["DATA SQUAD"]))
        .add_card(mon("PLAIN", "Plain", 4, 5, &[]))
        .add_card(text)
        .add_card(mon("TOP", "Top", 5, 7, &[]))
        .add_card(mon("FILL", "Fill", 3, 3, &[]))
        .deck(0, deck)
        .deck(1, &["FILL"; 5])
        .security(1, &["FILL"; 5])
        .hand(0, &[CARD])
        .memory(10)
        .start()
}

fn hand_sorted(r: &DebugRunner) -> Vec<String> {
    let mut h = hand_ids(r, 0);
    h.sort();
    h
}

#[test]
fn adds_one_chessmon_text_card_and_one_data_squad_card_and_bottoms_the_rest() {
    let mut r = runner(&["CHESS", "DS", "PLAIN"]);
    play_hand(&mut r, CARD);
    assert!(!legal(&r, PASS), "available category pick is mandatory");
    assert!(!legal(&r, reveal_action(&r, "DS")), "DS is not a Chessmon-text card");
    pick_reveal(&mut r, "CHESS");
    assert!(!legal(&r, reveal_action(&r, "PLAIN")));
    pick_reveal(&mut r, "DS");
    let _ = r.auto_resolve();
    assert_eq!(hand_sorted(&r), vec!["CHESS", "DS"]);
    assert_eq!(r.deck_size(0), 1);
    assert_eq!(r.game.players[0].deck[0].card_id(&r.game.card_data), "PLAIN");
}

#[test]
fn chessmon_in_effect_text_counts() {
    let mut r = runner(&["TEXT-CHESS", "PLAIN", "FILL"]);
    play_hand(&mut r, CARD);
    pick_reveal(&mut r, "TEXT-CHESS");
    let _ = r.auto_resolve();
    assert_eq!(hand_sorted(&r), vec!["TEXT-CHESS"]);
    assert_eq!(r.deck_size(0), 2);
}

#[test]
fn overlap_card_may_fill_the_chessmon_pick_alone_per_q4704() {
    let mut r = runner(&["CHESS", "BOTH", "PLAIN"]);
    play_hand(&mut r, CARD);
    assert!(legal(&r, reveal_action(&r, "CHESS")));
    assert!(legal(&r, reveal_action(&r, "BOTH")));
    pick_reveal(&mut r, "BOTH");
    let _ = r.auto_resolve();
    assert!(r.pending_selection().is_none());
    assert_eq!(hand_sorted(&r), vec!["BOTH"]);
    assert_eq!(r.deck_size(0), 2);
}

#[test]
fn overlap_card_may_fill_the_data_squad_pick_after_another_chessmon() {
    let mut r = runner(&["CHESS", "BOTH", "PLAIN"]);
    play_hand(&mut r, CARD);
    pick_reveal(&mut r, "CHESS");
    assert!(!legal(&r, reveal_action(&r, "CHESS")), "no card is used twice");
    pick_reveal(&mut r, "BOTH");
    let _ = r.auto_resolve();
    assert_eq!(hand_sorted(&r), vec!["BOTH", "CHESS"]);
}

#[test]
fn no_matches_returns_all_three_to_the_bottom() {
    let mut r = runner(&["PLAIN", "FILL", "FILL"]);
    play_hand(&mut r, CARD);
    let _ = r.auto_resolve();
    assert!(r.pending_selection().is_none());
    assert_eq!(r.hand_size(0), 0);
    assert_eq!(r.deck_size(0), 3);
}

#[test]
fn inherited_when_attacking_gives_minus_2000_dp_once_per_turn() {
    let mut r = runner(&["FILL"; 5]);
    let a = r.place_stack(0, &[CARD, "TOP"]).index;
    let victim = r.place_on_field(1, "TOP", Some(0));
    let base = r.dp_of(victim).unwrap();
    attack_security(&mut r, a);
    let v = r.pending_selection_view().expect("opponent Digimon pick");
    assert!(!v.is_optional);
    r.choose_prompt_card("TOP", None).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(r.dp_of(victim), Some(base - 2000));
    let other = r.place_on_field(1, "TOP", Some(0));
    attack_security(&mut r, a);
    let _ = r.auto_resolve();
    assert_eq!(r.dp_of(other), Some(base), "once per turn");
}
