//! EX13-051 Guardromon — Digimon, Lv.4, Black, 5000 DP, cost 4.
//! Traits: Machine. Champion / Virus. Digivolve: Black Lv.3 cost 2.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-051.md`)
//!
//! <Blocker> [All Turns] When any of your other Digimon with <Blocker> would
//! leave the battle area other than by your effects, by suspending this
//! Digimon, they don't leave.
//!
//! Inherited Effect: [Opponent's Turn] [Once Per Turn] When any of your Digimon
//! suspend, this Digimon may unsuspend.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Black/EX13_051.cs
//! - Blocker; `WhenRemoveField`, optional, not OPT: own other Digimon with
//!   <Blocker>, not by an owner effect, `CanActivateSuspendCostEffect` →
//!   suspend this; success → the protected Digimon don't leave.
//! - Inherited `OnTappedAnyone`, OPT, optional, opponent's turn, own Digimon
//!   suspended, carrier suspended → unsuspend carrier.
//!
//! # Patterns
//! - H: printed <Blocker>.  - F4: optional replacement with a suspend cost.
//! - G4/E2: inherited OPT on-suspend unsuspend.

#![allow(dead_code, unused_imports)]

use super::orphan_support::*;
use digimon_dsl::compiled::{
    CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::replacement::ReplacementCause;

const CARD_ID: &str = "EX13-051";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-051 YAML loads")
        .add_card(with_text(
            named("ALLY-BLK", "Blockmon", CardColor::Black, 4, 5000),
            "＜Blocker＞",
        ))
        .add_card(named("ALLY-PLAIN", "Plainmon", CardColor::Black, 4, 5000))
        .add_card(named("TOP", "Topmon", CardColor::Black, 5, 7000))
        .add_card(named("ALLY2", "AllyTwo", CardColor::Black, 3, 3000))
        .add_card(named("FILL", "Filler", CardColor::Black, 3, 3000))
}

fn start() -> DebugRunner {
    builder()
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(3)
        .start()
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_051_printed_metadata_blocker_and_clause_shape() {
    let mut r = start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(4), Some(5000), Some(4)));
    assert_eq!(c.color, vec![CompiledColor::Black]);
    assert_eq!(c.alt_paths.len(), 1);
    let reps: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                scope,
                optional,
                once_per_turn,
                ..
            }) => Some((*scope, *optional, *once_per_turn)),
            _ => None,
        })
        .collect();
    assert_eq!(reps, vec![(CompiledScope::FaceUp, true, false)]);
    let t: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].scope, CompiledScope::Inherited);
    assert!(t[0].once_per_turn && t[0].optional);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(h, Keyword::Blocker));
}

// ─── Section 2/3 — replacement ───────────────────────────────────────────────

#[test]
fn ex13_051_suspending_saves_another_blocker_from_opponent_effect() {
    let mut r = start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "ALLY-BLK", Some(0));
    delete_with(&mut r, ally, ReplacementCause::OpponentEffect);
    assert_replacement_prompt(&r);
    accept(&mut r);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"ALLY-BLK".to_string()), "ally doesn't leave");
    assert!(is_suspended(&r, me), "cost: Guardromon suspended");
}

#[test]
fn ex13_051_battle_deletion_is_other_than_by_your_effects() {
    let mut r = start();
    r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "ALLY-BLK", Some(0));
    delete_with(&mut r, ally, ReplacementCause::Battle);
    assert_replacement_prompt(&r);
}

#[test]
fn ex13_051_declining_lets_the_ally_leave() {
    let mut r = start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "ALLY-BLK", Some(0));
    delete_with(&mut r, ally, ReplacementCause::OpponentEffect);
    assert_replacement_prompt(&r);
    decline(&mut r);
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"ALLY-BLK".to_string()));
    assert!(!is_suspended(&r, me));
}

#[test]
fn ex13_051_ally_without_blocker_is_not_protected() {
    let mut r = start();
    r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "ALLY-PLAIN", Some(0));
    delete_with(&mut r, ally, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection().is_none());
    assert!(!field_ids(&r, 0).contains(&"ALLY-PLAIN".to_string()));
}

#[test]
fn ex13_051_does_not_protect_itself() {
    let mut r = start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "ALLY-BLK", Some(0));
    delete_with(&mut r, me, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection().is_none(), "'other' Digimon only");
    assert!(!field_ids(&r, 0).contains(&CARD_ID.to_string()));
}

#[test]
fn ex13_051_own_effect_removal_is_not_protected() {
    let mut r = start();
    r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "ALLY-BLK", Some(0));
    delete_with(&mut r, ally, ReplacementCause::OwnEffect);
    assert!(r.pending_selection().is_none(), "'other than by your effects'");
}

#[test]
fn ex13_051_suspended_guardromon_cannot_pay() {
    let mut r = start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    set_suspended(&mut r, me, true);
    let ally = r.place_on_field(0, "ALLY-BLK", Some(0));
    delete_with(&mut r, ally, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection().is_none(), "suspend cost unpayable");
}

#[test]
fn ex13_051_not_once_per_turn() {
    let mut r = start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "ALLY-BLK", Some(0));
    delete_with(&mut r, ally, ReplacementCause::OpponentEffect);
    accept(&mut r);
    let _ = r.auto_resolve();
    set_suspended(&mut r, me, false);
    let ally = handle_of(&r, 0, "ALLY-BLK");
    delete_with(&mut r, ally, ReplacementCause::OpponentEffect);
    assert_replacement_prompt(&r);
}

// ─── Section 2/3/5 — inherited on-suspend unsuspend ──────────────────────────

fn inherited_setup() -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut r = start();
    let carrier = r.place_stack(0, &[CARD_ID, "TOP"]);
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 1);
    r.game.memory = 3; // keep the opponent's turn alive through resolution
    (r, carrier)
}

#[test]
fn ex13_051_inherited_carrier_suspending_on_opponents_turn_may_unsuspend() {
    let (mut r, carrier) = inherited_setup();
    r.game.suspend(carrier);
    assert!(r.pending_is_optional(), "'may unsuspend'");
    accept(&mut r);
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 1, "still the opponent's turn");
    assert!(!is_suspended(&r, carrier));
}

#[test]
fn ex13_051_inherited_other_digimon_suspending_unsuspends_suspended_carrier() {
    let (mut r, carrier) = inherited_setup();
    set_suspended(&mut r, carrier, true);
    let ally = r.place_on_field(0, "ALLY2", Some(0));
    r.game.suspend(ally);
    accept(&mut r);
    let _ = r.auto_resolve();
    assert!(!is_suspended(&r, carrier));
    assert!(is_suspended(&r, ally), "only this Digimon unsuspends");
}

#[test]
fn ex13_051_inherited_declining_keeps_carrier_suspended() {
    let (mut r, carrier) = inherited_setup();
    r.game.suspend(carrier);
    decline(&mut r);
    let _ = r.auto_resolve();
    assert!(is_suspended(&r, carrier));
}

#[test]
fn ex13_051_inherited_not_on_your_own_turn() {
    let mut r = start();
    let carrier = r.place_stack(0, &[CARD_ID, "TOP"]);
    r.game.suspend(carrier);
    assert!(r.pending_selection().is_none(), "[Opponent's Turn] only");
}

#[test]
fn ex13_051_inherited_opponent_digimon_suspending_does_not_trigger() {
    let (mut r, carrier) = inherited_setup();
    set_suspended(&mut r, carrier, true);
    let opp = r.place_on_field(1, "ALLY2", Some(0));
    r.game.suspend(opp);
    assert!(r.pending_selection().is_none(), "'your' Digimon only");
}

#[test]
fn ex13_051_inherited_once_per_turn() {
    let (mut r, carrier) = inherited_setup();
    r.game.suspend(carrier);
    accept(&mut r);
    let _ = r.auto_resolve();
    r.game.suspend(carrier);
    assert!(r.pending_selection().is_none(), "OPT spent");
    assert!(is_suspended(&r, carrier));
}

/// Official Q&A: "When activated, all Digimon matching its trigger conditions
/// are prevented from leaving." Two <Blocker> allies deleted at once → one
/// suspension saves both.
#[test]
fn ex13_051_one_activation_saves_every_simultaneously_leaving_blocker() {
    let mut r = start();
    r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_on_field(0, "ALLY-BLK", Some(0));
    let b = r.place_on_field(0, "ALLY-BLK", Some(0));
    r.game
        .delete_permanents_batch(vec![a, b], ReplacementCause::OpponentEffect);
    assert_replacement_prompt(&r);
    accept(&mut r);
    let _ = r.auto_resolve();
    let n = field_ids(&r, 0).iter().filter(|c| *c == "ALLY-BLK").count();
    assert_eq!(n, 2, "both Blocker allies stay");
}
