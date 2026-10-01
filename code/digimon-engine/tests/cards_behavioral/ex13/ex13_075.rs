//! EX13-075 Mon — Tamer, White, cost 4.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-075.md`)
//!
//! [Start of Your Turn] If you have 2 or less memory, set it to 3. [On Play]
//! Reveal the top 3 cards of your deck. Add 1 card with [Huckmon] in its text
//! among them to the hand. Return the rest to the bottom of the deck.
//!
//! Security Effect: [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! None at b9a0638cd (no `EX13_075.cs`); printed text governs. Idioms:
//! BT21-084 (start-of-turn memory set + security self-play), EX13-027
//! (`reveal_search`).
//!
//! # Patterns
//! - B1: [Start of Your Turn] memory floor.  - A1: reveal-3 search.
//! - H: [Security] play this Tamer.

#![allow(dead_code, unused_imports)]

use super::orphan_support::*;
use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledColor, CompiledTiming};
use digimon_engine::enums::CardColor;

const CARD_ID: &str = "EX13-075";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-075 YAML loads")
        .add_card(named("HUCK", "BaoHuckmon", CardColor::Red, 4, 5000))
        .add_card({
            let mut o = option("HUCK-OPT", CardColor::White, 2);
            o.effect_text = "Search for 1 [Huckmon].".into();
            o
        })
        .add_card(named("PLAIN", "Greymon", CardColor::Red, 4, 5000))
        .add_card(named("ATK", "Attacker", CardColor::Red, 5, 7000))
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
fn ex13_075_is_a_white_cost_4_tamer_with_three_clauses() {
    let r = builder().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.kind, CompiledCardKind::Tamer);
    assert_eq!(c.color, vec![CompiledColor::White]);
    assert_eq!(c.cost, Some(4));
    let t: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    let whens: Vec<_> = t.iter().map(|t| t.when.clone()).collect();
    assert_eq!(
        whens,
        vec![
            vec![CompiledTiming::StartOfYourTurn],
            vec![CompiledTiming::OnPlay],
            vec![CompiledTiming::OnSecurity]
        ]
    );
}

// ─── Section 2/3 — [Start of Your Turn] ──────────────────────────────────────

#[test]
fn ex13_075_start_of_turn_with_2_or_less_memory_sets_it_to_3() {
    let mut r = builder().deck(0, &["FILL"; 6]).deck(1, &["FILL"; 6]).memory(3).start();
    r.place_on_field(0, CARD_ID, Some(0));
    cycle_to_my_turn_with_memory(&mut r, 1);
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 0);
    assert_eq!(r.memory(), 3);
}

#[test]
fn ex13_075_start_of_turn_with_more_than_2_memory_is_unchanged() {
    let mut r = builder().deck(0, &["FILL"; 6]).deck(1, &["FILL"; 6]).memory(3).start();
    r.place_on_field(0, CARD_ID, Some(0));
    cycle_to_my_turn_with_memory(&mut r, 5);
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 0);
    assert_eq!(r.memory(), 5);
}

// ─── Section 3 — [On Play] ───────────────────────────────────────────────────

#[test]
fn ex13_075_on_play_offers_huckmon_text_cards_and_bottoms_the_rest() {
    let mut r = on_play_setup(&["HUCK", "HUCK-OPT", "PLAIN"]);
    play_card(&mut r, 0, CARD_ID);
    assert_eq!(offered_reveal_ids(&r), vec!["HUCK".to_string(), "HUCK-OPT".to_string()]);
    pick_revealed(&mut r, "HUCK-OPT");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["HUCK-OPT".to_string()]);
    assert_eq!(r.deck_size(0), 6, "two returned to the bottom");
}

#[test]
fn ex13_075_on_play_no_match_bottoms_all() {
    let mut r = on_play_setup(&["PLAIN", "FILL", "PLAIN"]);
    play_card(&mut r, 0, CARD_ID);
    let _ = r.auto_resolve();
    assert!(hand_ids(&r, 0).is_empty());
    assert_eq!(r.deck_size(0), 7);
}

// ─── Section 3 — [Security] ──────────────────────────────────────────────────

#[test]
fn ex13_075_security_plays_mon_without_paying() {
    let mut r = builder()
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &[CARD_ID])
        .memory(0)
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(atk, 1, false);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).contains(&CARD_ID.to_string()), "played from security");
    assert_eq!(r.memory(), 0, "without paying the cost");
}
