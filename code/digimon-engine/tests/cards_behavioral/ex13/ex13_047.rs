//! EX13-047 Gotsumon — Digimon, Lv.3, Black, 3000 DP, cost 3.
//! Traits: Rock. Rookie / Data. Digivolve: Black Lv.2 cost 0.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-047.md`)
//!
//! <Blocker> [On Play] Reveal the top 3 cards of your deck. Add 1 card with the
//! [Royal Knight] trait and 1 card with <Blocker> among them to the hand.
//! Return the rest to the bottom of the deck. [When Attacking] Lose 2 memory.
//!
//! Inherited Effect: [Opponent's Turn] This Digimon gets +2000 DP.
//!
//! Official Q&A: "a card with <Blocker>" refers to a card that always has
//! <Blocker>; cards that gain it from effects, or that have it as an inherited
//! effect, can't be referenced.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Black/EX13_047.cs
//! - `BlockerSelfStaticEffect`; On Play `SimplifiedRevealDeckTopCardsAndSelect(3)`
//!   buckets `CardTraits.Contains("Royal Knight")` / `HasBlocker`, rest bottom;
//!   When Attacking `AddMemory(-2)`; inherited `ChangeSelfDPStaticEffect(+2000)`
//!   gated `IsOpponentTurn`.
//!
//! # Patterns
//! - H: printed <Blocker>.
//! - A1: two-bucket reveal (trait / printed keyword).
//! - D3: [When Attacking] memory loss.
//! - G4: inherited conditional DP aura.

#![allow(dead_code, unused_imports)]

use super::orphan_support::*;
use digimon_dsl::compiled::{CompiledClause, CompiledColor, CompiledScope, CompiledTiming};
use digimon_engine::enums::{CardColor, Keyword};

const CARD_ID: &str = "EX13-047";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-047 YAML loads")
        .add_card(digimon("RK", CardColor::Black, 6, 12000, &["Royal Knight"]))
        .add_card(with_text(
            named("BLOCKER", "Guardromon", CardColor::Black, 4, 5000),
            "＜Blocker＞",
        ))
        .add_card({
            let mut c = named("INH-BLOCKER", "Inheritmon", CardColor::Black, 4, 5000);
            c.inherited_text = "＜Blocker＞".into();
            c
        })
        .add_card(named("PLAIN", "Greymon", CardColor::Black, 4, 5000))
        .add_card(named("FILL", "Filler", CardColor::Black, 3, 3000))
        .add_card(named("TOP", "Topmon", CardColor::Black, 4, 5000))
        .add_card(named("SEC", "SecCard", CardColor::Black, 3, 1000))
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
fn ex13_047_printed_metadata_and_blocker() {
    let mut r = builder().deck(0, &["FILL"; 3]).deck(1, &["FILL"; 3]).start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(3), Some(3000), Some(3)));
    assert_eq!(c.color, vec![CompiledColor::Black]);
    assert_eq!(c.traits, vec!["Rock".to_string()]);
    assert_eq!(c.alt_paths.len(), 1);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(h, Keyword::Blocker), "<Blocker>");
}

#[test]
fn ex13_047_clause_shape() {
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
    assert_eq!(t[1].when, vec![CompiledTiming::WhenAttacking]);
    assert!(t.iter().all(|t| t.scope == CompiledScope::FaceUp && !t.optional));
}

// ─── Section 3 — [On Play] ───────────────────────────────────────────────────

#[test]
fn ex13_047_first_bucket_offers_royal_knight_trait_only() {
    let mut r = on_play_setup(&["RK", "BLOCKER", "PLAIN"]);
    play_card(&mut r, 0, CARD_ID);
    assert_eq!(offered_reveal_ids(&r), vec!["RK".to_string()]);
}

#[test]
fn ex13_047_adds_royal_knight_and_printed_blocker_rest_to_bottom() {
    let mut r = on_play_setup(&["RK", "BLOCKER", "PLAIN"]);
    play_card(&mut r, 0, CARD_ID);
    pick_revealed(&mut r, "RK");
    assert_eq!(offered_reveal_ids(&r), vec!["BLOCKER".to_string()]);
    pick_revealed(&mut r, "BLOCKER");
    let _ = r.auto_resolve();
    let mut hand = hand_ids(&r, 0);
    hand.sort();
    assert_eq!(hand, vec!["BLOCKER".to_string(), "RK".to_string()]);
    assert_eq!(deck_bottom_ids(&r, 0, 1), vec!["PLAIN".to_string()]);
}

#[test]
fn ex13_047_inherited_blocker_does_not_count_as_a_card_with_blocker() {
    let mut r = on_play_setup(&["INH-BLOCKER", "PLAIN", "FILL"]);
    play_card(&mut r, 0, CARD_ID);
    let _ = r.auto_resolve();
    assert!(
        hand_ids(&r, 0).is_empty(),
        "Official Q&A: inherited <Blocker> can't be referenced"
    );
    assert_eq!(r.deck_size(0), 7);
}

// ─── Section 3 — [When Attacking] ────────────────────────────────────────────

#[test]
fn ex13_047_when_attacking_loses_2_memory() {
    let mut r = builder()
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .security(1, &["SEC", "SEC"])
        .memory(4)
        .start();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    r.attack_player(h, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 2, "lose 2 memory");
}

// ─── Section 2 — inherited [Opponent's Turn] +2000 ───────────────────────────

#[test]
fn ex13_047_inherited_no_bonus_on_own_turn() {
    let mut r = builder().deck(0, &["FILL"; 4]).deck(1, &["FILL"; 4]).start();
    let h = r.place_stack(0, &[CARD_ID, "TOP"]);
    assert_eq!(r.effective_dp(h), Some(5000));
}

#[test]
fn ex13_047_inherited_plus_2000_on_opponents_turn() {
    let mut r = builder().deck(0, &["FILL"; 4]).deck(1, &["FILL"; 4]).start();
    let h = r.place_stack(0, &[CARD_ID, "TOP"]);
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 1);
    assert_eq!(r.effective_dp(h), Some(7000));
}

#[test]
fn ex13_047_face_up_gotsumon_gets_no_inherited_bonus() {
    let mut r = builder().deck(0, &["FILL"; 4]).deck(1, &["FILL"; 4]).start();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.effective_dp(h), Some(3000), "inherited text only applies as a source");
}
