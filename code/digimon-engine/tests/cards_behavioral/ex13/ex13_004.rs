//! EX13-004 DemiMeramon — Digi-Egg, Lv.2, Yellow. Traits: Flame.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-004.md`)
//!
//! Inherited Effect: [When Attacking] [Once Per Turn] This Digimon may
//! digivolve into a Digimon card with [Witchelny] in its text in the hand with
//! the cost reduced by 1. If this effect digivolved, trash your top security
//! card.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Yellow/EX13_004.cs
//! - `OnAllyAttack`, inherited, OPT, optional.
//! - CanActivate: a hand card `HasText("Witchelny")`.
//! - Body: `DigivolveIntoHandOrTrashCard(payCost, reduceCost 1, isHand)` with
//!   `successProcess` = `IDestroySecurity(owner, 1, fromTop)`.
//!
//! # Patterns
//! - G4: inherited [When Attacking] on a DigiEgg source.
//! - D2: effect-initiated digivolve with a cost reduction.
//! - D1: "if this effect digivolved" result-gated follow-up.
//! - E2: optional + OPT.

#![allow(dead_code, unused_imports)]

use super::orphan_support::*;
use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "EX13-004";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-004 YAML loads")
        .add_card(named("CARRIER", "Candlemon", CardColor::Yellow, 3, 3000))
        .add_card(with_evo(
            with_text(
                named("WITCH-TEXT", "Wizardmon", CardColor::Yellow, 4, 5000),
                "[On Play] Search for a card with [Witchelny] in its text.",
            ),
            CardColor::Yellow,
            3,
            3,
        ))
        .add_card(with_evo(
            digimon("WITCH-TRAIT", CardColor::Yellow, 4, 5000, &["Witchelny"]),
            CardColor::Yellow,
            3,
            3,
        ))
        .add_card(with_evo(
            named("PLAIN-EVO", "Greymon", CardColor::Yellow, 4, 5000),
            CardColor::Yellow,
            3,
            3,
        ))
        .add_card(named("FILL", "Filler", CardColor::Yellow, 3, 3000))
        .add_card(named("SEC", "SecCard", CardColor::Yellow, 3, 1000))
}

fn setup(hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = builder()
        .hand(0, hand)
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(0, &["SEC", "SEC", "SEC"])
        .security(1, &["SEC", "SEC", "SEC"])
        .memory(5)
        .start();
    let h = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    (r, h)
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_004_has_one_inherited_when_attacking_opt_clause() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "DemiMeramon");
    assert_eq!(card.level, Some(2));
    assert_eq!(card.effects.len(), 1);
    let CompiledClause::Triggered(t) = &card.effects[0] else {
        panic!("triggered clause expected");
    };
    assert_eq!(t.scope, CompiledScope::Inherited);
    assert_eq!(t.when, vec![CompiledTiming::WhenAttacking]);
    assert!(t.once_per_turn);
}

// ─── Section 2/3 — behavior ──────────────────────────────────────────────────

#[test]
fn ex13_004_attack_offers_only_witchelny_text_cards() {
    let (mut r, h) = setup(&["WITCH-TEXT", "WITCH-TRAIT", "PLAIN-EVO"]);
    r.attack_player(h, 1, false);
    assert!(r.pending_is_optional(), "'may digivolve'");
    assert_eq!(
        offered_hand_ids(&r),
        vec!["WITCH-TEXT".to_string(), "WITCH-TRAIT".to_string()],
        "[Witchelny] in text (effect text or trait); Greymon is not eligible"
    );
}

#[test]
fn ex13_004_digivolve_costs_one_less_and_trashes_own_top_security() {
    let (mut r, h) = setup(&["WITCH-TEXT"]);
    let mem0 = r.memory();
    r.attack_player(h, 1, false);
    pick_hand(&mut r, "WITCH-TEXT");
    // Digivolve draw happens; the attack continues afterwards.
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["WITCH-TEXT".to_string()]);
    assert_eq!(mem0 - r.memory(), 2, "digivolve cost 3 reduced by 1");
    assert_eq!(r.security_count(0), 2, "own top security trashed");
    assert!(trash_ids(&r, 0).contains(&"SEC".to_string()));
}

#[test]
fn ex13_004_declining_does_not_digivolve_or_trash_security() {
    let (mut r, h) = setup(&["WITCH-TEXT"]);
    r.attack_player(h, 1, false);
    assert!(r.pending_is_optional());
    decline(&mut r);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["CARRIER".to_string()]);
    assert_eq!(r.security_count(0), 3, "no digivolve → no security trash");
}

#[test]
fn ex13_004_without_witchelny_card_nothing_happens() {
    let (mut r, h) = setup(&["PLAIN-EVO"]);
    r.attack_player(h, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["CARRIER".to_string()]);
    assert_eq!(r.security_count(0), 3);
}

// ─── Section 5 — OPT ─────────────────────────────────────────────────────────

#[test]
fn ex13_004_once_per_turn_second_attack_gets_no_prompt() {
    let (mut r, h) = setup(&["WITCH-TEXT", "WITCH-TRAIT"]);
    r.attack_player(h, 1, false);
    pick_hand(&mut r, "WITCH-TEXT");
    let _ = r.auto_resolve();
    let h = handle_of(&r, 0, "WITCH-TEXT");
    set_suspended(&mut r, h, false);
    r.attack_player(h, 1, false);
    assert!(
        r.pending_selection().is_none(),
        "OPT spent: no second digivolve offer"
    );
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["WITCH-TEXT".to_string()]);
}

#[test]
fn ex13_004_declining_keeps_the_once_per_turn_available() {
    let (mut r, h) = setup(&["WITCH-TEXT"]);
    r.attack_player(h, 1, false);
    decline(&mut r);
    let _ = r.auto_resolve();
    set_suspended(&mut r, h, false);
    r.attack_player(h, 1, false);
    assert!(
        r.pending_is_optional(),
        "declined optional effect did not activate, so OPT is unspent"
    );
}
