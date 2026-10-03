//! EX13-046 Kokuwamon — Digimon, Lv.3, Black, 2000 DP, cost 3.
//! Traits: Machine. Rookie / Data. Digivolve: Black Lv.2 cost 0.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-046.md`)
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 card with [Mamemon] in
//! its text and 1 card with the [Mutant] trait among them to the hand. Return
//! the rest to the bottom of the deck.
//!
//! Inherited Effect: [On Deletion] <De-Digivolve 1> 1 of your opponent's
//! Digimon.
//!
//! # DCGO C# reference
//! None at b9a0638cd (no `EX13_046.cs`); printed text + general_rule.pdf govern.
//! Reveal shape mirrors EX13-009's `SimplifiedRevealDeckTopCardsAndSelect`
//! two-bucket form; the inherited mirrors EX13-053 (shipped, Mutant slice).
//!
//! # Patterns
//! - A1: reveal-N search with two buckets (text / trait).
//! - G4 + F2: inherited [On Deletion] <De-Digivolve 1>.

#![allow(dead_code, unused_imports)]

use super::orphan_support::*;
use digimon_dsl::compiled::{CompiledClause, CompiledColor, CompiledScope, CompiledTiming};
use digimon_engine::enums::CardColor;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-046";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-046 YAML loads")
        .add_card(named("MAME-NAME", "PrinceMamemon", CardColor::Black, 5, 7000))
        .add_card(with_text(
            named("MAME-TEXT", "Thundermon", CardColor::Black, 4, 5000),
            "Treated as also having [Mamemon] in its name.",
        ))
        .add_card(digimon("MUTANT", CardColor::Black, 4, 5000, &["Mutant"]))
        .add_card(named("PLAIN", "Greymon", CardColor::Black, 4, 5000))
        .add_card(named("FILL", "Filler", CardColor::Black, 3, 3000))
        .add_card(named("TOP", "Topmon", CardColor::Black, 4, 5000))
        .add_card(named("OPP-LV3", "OppThree", CardColor::Red, 3, 3000))
        .add_card(named("OPP-LV4", "OppFour", CardColor::Red, 4, 5000))
}

fn on_play_setup(top: &[&str]) -> DebugRunner {
    let deck = deck_with_top(top, "FILL", 4);
    builder()
        .hand(0, &[CARD_ID])
        .deck(0, &deck)
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start()
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_046_printed_metadata() {
    let r = builder().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(3), Some(2000), Some(3)));
    assert_eq!(c.color, vec![CompiledColor::Black]);
    assert_eq!(c.traits, vec!["Machine".to_string()]);
    assert_eq!(c.alt_paths.len(), 1, "Black Lv.2 / cost 0");
}

#[test]
fn ex13_046_clause_shape() {
    let r = builder().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    let t: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(t.len(), 2);
    assert_eq!((t[0].scope, t[0].when.clone()), (CompiledScope::FaceUp, vec![CompiledTiming::OnPlay]));
    assert_eq!(
        (t[1].scope, t[1].when.clone()),
        (CompiledScope::Inherited, vec![CompiledTiming::OnDeletion])
    );
}

// ─── Section 3 — [On Play] ───────────────────────────────────────────────────

#[test]
fn ex13_046_first_bucket_offers_mamemon_text_cards_only() {
    let mut r = on_play_setup(&["MAME-NAME", "MAME-TEXT", "MUTANT"]);
    play_card(&mut r, 0, CARD_ID);
    assert_eq!(
        offered_reveal_ids(&r),
        vec!["MAME-NAME".to_string(), "MAME-TEXT".to_string()]
    );
}

#[test]
fn ex13_046_adds_one_mamemon_and_one_mutant_rest_to_bottom() {
    let mut r = on_play_setup(&["MAME-NAME", "MUTANT", "PLAIN"]);
    play_card(&mut r, 0, CARD_ID);
    pick_revealed(&mut r, "MAME-NAME");
    assert_eq!(offered_reveal_ids(&r), vec!["MUTANT".to_string()]);
    pick_revealed(&mut r, "MUTANT");
    let _ = r.auto_resolve();
    let mut hand = hand_ids(&r, 0);
    hand.sort();
    assert_eq!(hand, vec!["MAME-NAME".to_string(), "MUTANT".to_string()]);
    assert_eq!(deck_bottom_ids(&r, 0, 1), vec!["PLAIN".to_string()]);
}

#[test]
fn ex13_046_mutant_only_reveal_still_adds_the_mutant() {
    let mut r = on_play_setup(&["PLAIN", "MUTANT", "FILL"]);
    play_card(&mut r, 0, CARD_ID);
    assert_eq!(offered_reveal_ids(&r), vec!["MUTANT".to_string()]);
    pick_revealed(&mut r, "MUTANT");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["MUTANT".to_string()]);
    assert_eq!(r.deck_size(0), 6);
}

#[test]
fn ex13_046_no_match_returns_all_to_bottom() {
    let mut r = on_play_setup(&["PLAIN", "FILL", "PLAIN"]);
    play_card(&mut r, 0, CARD_ID);
    let _ = r.auto_resolve();
    assert!(hand_ids(&r, 0).is_empty());
    assert_eq!(r.deck_size(0), 7);
}

// ─── Section 3 — inherited [On Deletion] ─────────────────────────────────────

#[test]
fn ex13_046_inherited_on_deletion_de_digivolves_1() {
    let mut r = builder().deck(0, &["FILL"; 4]).deck(1, &["FILL"; 4]).start();
    let carrier = r.place_stack(0, &[CARD_ID, "TOP"]);
    let opp = r.place_stack(1, &["OPP-LV3", "OPP-LV4"]);
    delete_by_opponent_effect(&mut r, carrier);
    assert_eq!(r.pending_kind(), Some(SelectionKind::OppField));
    pick_field(&mut r, opp);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, opp), "OPP-LV3");
    assert!(trash_ids(&r, 1).contains(&"OPP-LV4".to_string()));
}

#[test]
fn ex13_046_face_up_kokuwamon_deletion_does_not_de_digivolve() {
    let mut r = builder().deck(0, &["FILL"; 4]).deck(1, &["FILL"; 4]).start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_stack(1, &["OPP-LV3", "OPP-LV4"]);
    delete_by_opponent_effect(&mut r, me);
    assert!(r.pending_selection().is_none(), "inherited only works as a source");
    assert_eq!(top_id(&r, opp), "OPP-LV4");
}
