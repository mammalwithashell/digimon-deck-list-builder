//! EX13-005 Bebydomon — Digi-Egg, Lv.2, Green. Traits: Baby Dragon.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-005.md`)
//!
//! Inherited Effect: [When Attacking] [Once Per Turn] You may play or use 1
//! card with [Dracomon] or [Examon] in its text from your hand with the cost
//! reduced by 1.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Green/EX13_005.cs
//! - `OnAllyAttack`, inherited, OPT, skippable.
//! - Candidate: `HasText("Dracomon") || HasText("Examon")` and playable/usable
//!   at cost − 1. Optional hand pick (`canNoSelect: true`); Options are used
//!   (`PlayOptionCards`), Digimon/Tamers played (`PlayPermanentCards`), both
//!   paying cost − 1.
//!
//! # Patterns
//! - G4: inherited [When Attacking] on a DigiEgg source.
//! - A3: play-or-use from hand with a cost reduction.
//! - E2: optional + OPT.

#![allow(dead_code, unused_imports)]

use super::orphan_support::*;
use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "EX13-005";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-005 YAML loads")
        .add_card(named("CARRIER", "Monodramon", CardColor::Green, 3, 3000))
        .add_card(with_cost(
            named("DRACO", "Dracomon X", CardColor::Green, 4, 5000),
            4,
        ))
        .add_card(with_cost(
            with_text(
                named("EXA-TEXT", "Breakdramon", CardColor::Green, 5, 7000),
                "Treated as an [Examon] material.",
            ),
            5,
        ))
        .add_card({
            let mut o = option("EXA-OPT", CardColor::Green, 2);
            o.card_name = "Examon Strike".into();
            o
        })
        .add_card(with_cost(named("PLAIN", "Greymon", CardColor::Green, 4, 5000), 4))
        .add_card(named("FILL", "Filler", CardColor::Green, 3, 3000))
        .add_card(named("SEC", "SecCard", CardColor::Green, 3, 1000))
}

fn setup(hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = builder()
        .hand(0, hand)
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(0, &["SEC", "SEC", "SEC"])
        .security(1, &["SEC", "SEC", "SEC"])
        .memory(6)
        .start();
    let h = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    (r, h)
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_005_has_one_inherited_when_attacking_opt_clause() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Bebydomon");
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
fn ex13_005_offers_only_dracomon_or_examon_text_cards() {
    let (mut r, h) = setup(&["DRACO", "EXA-TEXT", "EXA-OPT", "PLAIN"]);
    r.attack_player(h, 1, false);
    assert!(r.pending_is_optional(), "'You may play or use'");
    assert_eq!(
        offered_hand_ids(&r),
        vec![
            "DRACO".to_string(),
            "EXA-OPT".to_string(),
            "EXA-TEXT".to_string()
        ],
        "[Dracomon] in name, [Examon] in effect text, [Examon] in an Option's name"
    );
}

#[test]
fn ex13_005_plays_a_digimon_with_cost_reduced_by_one() {
    let (mut r, h) = setup(&["DRACO"]);
    let mem0 = r.memory();
    r.attack_player(h, 1, false);
    pick_hand(&mut r, "DRACO");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"DRACO".to_string()), "Dracomon X played");
    assert_eq!(mem0 - r.memory(), 3, "play cost 4 reduced by 1");
}

#[test]
fn ex13_005_uses_an_option_with_cost_reduced_by_one() {
    let (mut r, h) = setup(&["EXA-OPT"]);
    let mem0 = r.memory();
    r.attack_player(h, 1, false);
    pick_hand(&mut r, "EXA-OPT");
    let _ = r.auto_resolve();
    assert!(hand_ids(&r, 0).is_empty(), "Option left the hand");
    assert!(trash_ids(&r, 0).contains(&"EXA-OPT".to_string()), "used Option is trashed");
    assert_eq!(mem0 - r.memory(), 1, "use cost 2 reduced by 1");
}

#[test]
fn ex13_005_declining_plays_nothing() {
    let (mut r, h) = setup(&["DRACO"]);
    let mem0 = r.memory();
    r.attack_player(h, 1, false);
    decline(&mut r);
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["DRACO".to_string()]);
    assert_eq!(r.memory(), mem0);
}

#[test]
fn ex13_005_no_eligible_card_no_prompt() {
    let (mut r, h) = setup(&["PLAIN"]);
    r.attack_player(h, 1, false);
    assert!(r.pending_selection().is_none(), "no [Dracomon]/[Examon] card in hand");
}

// ─── Section 5 — OPT ─────────────────────────────────────────────────────────

#[test]
fn ex13_005_once_per_turn_second_attack_gets_no_prompt() {
    let (mut r, h) = setup(&["DRACO", "EXA-TEXT"]);
    r.attack_player(h, 1, false);
    pick_hand(&mut r, "DRACO");
    let _ = r.auto_resolve();
    let h = handle_of(&r, 0, "CARRIER");
    set_suspended(&mut r, h, false);
    r.attack_player(h, 1, false);
    assert!(r.pending_selection().is_none(), "OPT spent this turn");
}
