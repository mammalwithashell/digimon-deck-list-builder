//! EX13-009 Huckmon — Digimon, Lv.3, Red/White, 2000 DP, cost 3.
//! Traits: Mini Dragon. Rookie / Data. Digivolve: Red Lv.2 cost 0.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-009.md`)
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with
//! [Huckmon] or [Sistermon] in its text and 1 such Tamer card or Option card
//! among them to the hand. Return the rest to the bottom of the deck.
//!
//! Inherited Effect: [Your Turn] [Once Per Turn] When any of your white Digimon
//! are played, gain 1 memory.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Red/EX13_009.cs
//! - On Play: `SimplifiedRevealDeckTopCardsAndSelect(3)` with two AddHand
//!   buckets (Digimon w/ text; Tamer-or-Option w/ text), rest to deck bottom.
//! - Inherited: `OnEnterFieldAnyone`, OPT, owner's turn, own Digimon whose top
//!   card is white → `AddMemory(1)`.
//!
//! # Patterns
//! - A1: reveal-N search with two typed buckets.
//! - G4/B3: inherited OPT trigger on an ally Digimon being played.

#![allow(dead_code, unused_imports)]

use super::orphan_support::*;
use digimon_dsl::compiled::{CompiledClause, CompiledColor, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "EX13-009";

fn txt(mut c: CardData, name: &str) -> CardData {
    c.card_name = name.to_string();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-009 YAML loads")
        .add_card(named("HUCK-DIGI", "BaoHuckmon", CardColor::Red, 4, 5000))
        .add_card(with_text(
            named("SIS-DIGI", "Angewomon", CardColor::White, 5, 7000),
            "Play 1 [Sistermon Blanc] from your hand.",
        ))
        .add_card(txt(tamer("HUCK-TAMER", CardColor::White, 4), "Huckmon Tamer"))
        .add_card({
            let mut t = tamer("SIS-TAMER-TXT", CardColor::White, 3);
            t.effect_text = "Search for 1 [Sistermon Ciel].".into();
            t
        })
        .add_card(txt(option("HUCK-OPT", CardColor::Red, 2), "Huckmon Strike"))
        .add_card(txt(tamer("PLAIN-TAMER", CardColor::White, 3), "Tai"))
        .add_card(named("PLAIN", "Greymon", CardColor::Red, 4, 5000))
        .add_card(named("WHITE-1", "WhiteOne", CardColor::White, 3, 3000))
        .add_card(named("WHITE-2", "WhiteTwo", CardColor::White, 3, 3000))
        .add_card(named("RED-1", "RedOne", CardColor::Red, 3, 3000))
        .add_card(named("CARRIER", "Carrier", CardColor::Red, 4, 5000))
        .add_card(named("FILL", "Filler", CardColor::Red, 3, 3000))
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
fn ex13_009_printed_metadata_and_digivolve_circle() {
    let r = builder().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(3), Some(2000), Some(3)));
    assert_eq!(c.color, vec![CompiledColor::Red, CompiledColor::White]);
    assert_eq!(c.traits, vec!["Mini Dragon".to_string()]);
    assert_eq!(c.alt_paths.len(), 1, "Red Lv.2 / cost 0");
}

#[test]
fn ex13_009_clause_shape() {
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
    assert_eq!(t[0].when, vec![CompiledTiming::OnPlay]);
    assert_eq!(t[0].scope, CompiledScope::FaceUp);
    assert!(!t[0].optional && !t[0].once_per_turn);
    assert_eq!(t[1].scope, CompiledScope::Inherited);
    assert!(t[1].once_per_turn && !t[1].optional);
}

// ─── Section 3 — [On Play] reveal ────────────────────────────────────────────

#[test]
fn ex13_009_first_pick_offers_only_huckmon_or_sistermon_text_digimon() {
    let mut r = on_play_setup(&["HUCK-DIGI", "SIS-DIGI", "PLAIN"]);
    play_card(&mut r, 0, CARD_ID);
    assert_eq!(
        offered_reveal_ids(&r),
        vec!["HUCK-DIGI".to_string(), "SIS-DIGI".to_string()],
        "only Digimon with [Huckmon]/[Sistermon] in text"
    );
}

#[test]
fn ex13_009_adds_one_digimon_and_one_tamer_rest_to_bottom() {
    let mut r = on_play_setup(&["HUCK-DIGI", "HUCK-TAMER", "PLAIN"]);
    play_card(&mut r, 0, CARD_ID);
    pick_revealed(&mut r, "HUCK-DIGI");
    assert_eq!(offered_reveal_ids(&r), vec!["HUCK-TAMER".to_string()]);
    pick_revealed(&mut r, "HUCK-TAMER");
    let _ = r.auto_resolve();
    let mut hand = hand_ids(&r, 0);
    hand.sort();
    assert_eq!(hand, vec!["HUCK-DIGI".to_string(), "HUCK-TAMER".to_string()]);
    assert_eq!(deck_bottom_ids(&r, 0, 1), vec!["PLAIN".to_string()]);
}

#[test]
fn ex13_009_second_bucket_accepts_option_cards_and_text_matches() {
    let mut r = on_play_setup(&["SIS-TAMER-TXT", "HUCK-OPT", "PLAIN-TAMER"]);
    play_card(&mut r, 0, CARD_ID);
    // No eligible Digimon → straight to the Tamer/Option bucket.
    assert_eq!(
        offered_reveal_ids(&r),
        vec!["HUCK-OPT".to_string(), "SIS-TAMER-TXT".to_string()],
        "Option with [Huckmon] in name, Tamer with [Sistermon] in effect text; plain Tamer excluded"
    );
    pick_revealed(&mut r, "HUCK-OPT");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["HUCK-OPT".to_string()]);
    assert_eq!(r.deck_size(0), 6, "two unpicked cards returned to the bottom");
}

#[test]
fn ex13_009_no_match_returns_all_three_to_bottom() {
    let mut r = on_play_setup(&["PLAIN", "PLAIN-TAMER", "FILL"]);
    play_card(&mut r, 0, CARD_ID);
    let _ = r.auto_resolve();
    assert!(hand_ids(&r, 0).is_empty());
    assert_eq!(r.deck_size(0), 7);
}

// ─── Section 2/3/5 — inherited memory gain ───────────────────────────────────

fn inherited_setup(hand: &[&str]) -> DebugRunner {
    let mut r = builder()
        .hand(0, hand)
        .hand(1, &["WHITE-1"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.place_stack(0, &[CARD_ID, "CARRIER"]);
    r
}

#[test]
fn ex13_009_inherited_white_digimon_played_gains_one_memory() {
    let mut r = inherited_setup(&["WHITE-1"]);
    let mem0 = r.memory();
    play_card(&mut r, 0, "WHITE-1");
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem0 - 3 + 1, "paid 3, gained 1");
}

#[test]
fn ex13_009_inherited_non_white_digimon_does_not_gain() {
    let mut r = inherited_setup(&["RED-1"]);
    let mem0 = r.memory();
    play_card(&mut r, 0, "RED-1");
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem0 - 3);
}

#[test]
fn ex13_009_inherited_white_tamer_does_not_gain() {
    let mut r = inherited_setup(&["HUCK-TAMER"]);
    let mem0 = r.memory();
    play_card(&mut r, 0, "HUCK-TAMER");
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem0 - 4, "Tamers are not Digimon");
}

#[test]
fn ex13_009_inherited_opponents_white_digimon_on_their_turn_does_not_gain() {
    let mut r = inherited_setup(&[]);
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 1);
    r.game.memory = 5;
    play_card(&mut r, 1, "WHITE-1");
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 2, "opponent paid 3 and gained nothing");
}

#[test]
fn ex13_009_inherited_once_per_turn() {
    let mut r = inherited_setup(&["WHITE-1", "WHITE-2"]);
    let mem0 = r.memory();
    play_card(&mut r, 0, "WHITE-1");
    let _ = r.auto_resolve();
    play_card(&mut r, 0, "WHITE-2");
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem0 - 6 + 1, "only the first white play gains");
}
