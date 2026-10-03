//! EX13-002 DemiVeemon — Digi-Egg, Lv.2, Blue. Traits: Baby Dragon.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-002.md`)
//!
//! Inherited Effect: [Your Turn] [Once Per Turn] When any of your blue Tamers
//! are played, this Digimon with [Veedramon] in its name may unsuspend.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Blue/EX13_002.cs
//! - `OnEnterFieldAnyone`, inherited, OPT, optional (`SetUpActivateClass(.., 1, true, ..)`).
//! - CanUse: owner's turn && `CanTriggerOnPermanentPlay` for an own Tamer whose
//!   top card is blue.
//! - CanActivate: carrier top card `ContainsCardName("Veedramon")` &&
//!   `CanUnsuspend(carrier)` (it must be suspended).
//! - Body: `IUnsuspendPermanents([carrier])`.
//!
//! # Patterns
//! - G4: inherited trigger on a DigiEgg source.
//! - B3: trigger on another permanent's play (blue Tamer gate).
//! - E2: optional + [Once Per Turn].

#![allow(dead_code, unused_imports)]

use super::orphan_support::*;
use digimon_dsl::compiled::{CompiledClause, CompiledScope};
use digimon_engine::enums::CardColor;

const CARD_ID: &str = "EX13-002";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-002 YAML loads")
        .add_card(named("VEED", "Veedramon", CardColor::Blue, 4, 6000))
        .add_card(named("XVEED", "XVeedramon", CardColor::Blue, 5, 8000))
        .add_card(named("PLAIN", "Greymon", CardColor::Blue, 4, 6000))
        .add_card(tamer("BLUE-T", CardColor::Blue, 2))
        .add_card(tamer("BLUE-T2", CardColor::Blue, 2))
        .add_card(tamer("RED-T", CardColor::Red, 2))
        .add_card(named("FILL", "Filler", CardColor::Blue, 3, 3000))
}

/// Carrier `top` with DemiVeemon under it, suspended, `hand` in hand.
fn setup(top: &str, hand: &[&str]) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut r = builder()
        .hand(0, hand)
        .hand(1, &["BLUE-T"])
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();
    let h = r.place_stack(0, &[CARD_ID, top]);
    set_suspended(&mut r, h, true);
    (r, h)
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_002_has_one_inherited_optional_opt_clause() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "DemiVeemon");
    assert_eq!(card.level, Some(2));
    assert_eq!(card.effects.len(), 1);
    let CompiledClause::Triggered(t) = &card.effects[0] else {
        panic!("triggered clause expected");
    };
    assert_eq!(t.scope, CompiledScope::Inherited);
    assert!(t.once_per_turn, "[Once Per Turn]");
    assert!(t.optional, "'may unsuspend'");
}

// ─── Section 2/3 — trigger + behavior ────────────────────────────────────────

#[test]
fn ex13_002_blue_tamer_play_unsuspends_veedramon_carrier_on_accept() {
    let (mut r, h) = setup("VEED", &["BLUE-T"]);
    play_card(&mut r, 0, "BLUE-T");
    assert!(r.pending_is_optional(), "'may unsuspend' prompt");
    accept(&mut r);
    let _ = r.auto_resolve();
    assert!(!is_suspended(&r, h), "carrier unsuspended");
}

#[test]
fn ex13_002_name_contains_matches_compound_veedramon_names() {
    let (mut r, h) = setup("XVEED", &["BLUE-T"]);
    play_card(&mut r, 0, "BLUE-T");
    accept(&mut r);
    let _ = r.auto_resolve();
    assert!(!is_suspended(&r, h), "XVeedramon has [Veedramon] in its name");
}

#[test]
fn ex13_002_declining_leaves_the_carrier_suspended() {
    let (mut r, h) = setup("VEED", &["BLUE-T"]);
    play_card(&mut r, 0, "BLUE-T");
    assert!(r.pending_is_optional());
    decline(&mut r);
    let _ = r.auto_resolve();
    assert!(is_suspended(&r, h));
}

#[test]
fn ex13_002_non_veedramon_carrier_does_not_trigger() {
    let (mut r, h) = setup("PLAIN", &["BLUE-T"]);
    play_card(&mut r, 0, "BLUE-T");
    assert!(r.pending_selection().is_none(), "carrier lacks [Veedramon]");
    assert!(is_suspended(&r, h));
}

#[test]
fn ex13_002_non_blue_tamer_does_not_trigger() {
    let (mut r, h) = setup("VEED", &["RED-T"]);
    play_card(&mut r, 0, "RED-T");
    assert!(r.pending_selection().is_none(), "red Tamer is not blue");
    assert!(is_suspended(&r, h));
}

#[test]
fn ex13_002_unsuspended_carrier_gets_no_prompt() {
    let (mut r, h) = setup("VEED", &["BLUE-T"]);
    set_suspended(&mut r, h, false);
    play_card(&mut r, 0, "BLUE-T");
    assert!(
        r.pending_selection().is_none(),
        "DCGO CanUnsuspend: nothing to unsuspend"
    );
}

#[test]
fn ex13_002_opponent_blue_tamer_on_their_turn_does_not_trigger() {
    let (mut r, h) = setup("VEED", &[]);
    r.end_turn();
    let _ = r.auto_resolve();
    // Opponent's turn: our carrier stays suspended through our unsuspend-less turn swap.
    set_suspended(&mut r, h, true);
    assert_eq!(r.turn_player(), 1);
    r.game.memory = 10;
    play_card(&mut r, 1, "BLUE-T");
    assert!(
        r.pending_selection().is_none(),
        "[Your Turn] + 'your' Tamers: opponent's Tamer on their turn is ignored"
    );
    assert!(is_suspended(&r, h));
}

// ─── Section 5 — OPT ─────────────────────────────────────────────────────────

#[test]
fn ex13_002_once_per_turn_lockout_and_reset() {
    let (mut r, h) = setup("VEED", &["BLUE-T", "BLUE-T2"]);
    play_card(&mut r, 0, "BLUE-T");
    accept(&mut r);
    let _ = r.auto_resolve();
    assert!(!is_suspended(&r, h));
    set_suspended(&mut r, h, true);
    play_card(&mut r, 0, "BLUE-T2");
    assert!(r.pending_selection().is_none(), "OPT already used this turn");
    assert!(is_suspended(&r, h));
}

#[test]
fn ex13_002_once_per_turn_clears_on_next_own_turn() {
    let (mut r, h) = setup("VEED", &["BLUE-T", "BLUE-T2"]);
    play_card(&mut r, 0, "BLUE-T");
    accept(&mut r);
    let _ = r.auto_resolve();
    r.end_turn();
    let _ = r.auto_resolve();
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 0);
    set_suspended(&mut r, h, true);
    r.game.memory = 10;
    play_card(&mut r, 0, "BLUE-T2");
    assert!(r.pending_is_optional(), "fresh turn → OPT available again");
    accept(&mut r);
    let _ = r.auto_resolve();
    assert!(!is_suspended(&r, h));
}
