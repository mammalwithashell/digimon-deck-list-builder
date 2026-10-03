//! EX13-056 Giromon — Digimon, Lv.5, Black, 7000 DP, cost 7.
//! Traits: Mine, Machine (Rule). Ultimate / Vaccine. Digivolve: Black Lv.4 cost 3.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-056.md`)
//!
//! <Collision> <Blocker> [All Turns] [Once Per Turn] When this Digimon
//! suspends, reveal the top 3 cards of your deck. You may play 1 level 4 or
//! lower black Digimon card with <Blocker> among them without paying the cost.
//! Trash the rest. (Rule) Trait: Has [Machine] Type.
//!
//! Inherited Effect: [Opponent's Turn] [Once Per Turn] When any of your Digimon
//! suspend, you may play 1 level 5 or lower black Digimon card with <Blocker>
//! from your hand without paying the cost.
//!
//! Official Q&A: "a card with <Blocker>" = a card that always has it (printed);
//! granted / inherited <Blocker> can't be referenced.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Black/EX13_056.cs
//! - Collision + Blocker statics.
//! - `OnTappedAnyone` OPT, self suspends: reveal 3, optional pick of a
//!   `Level <= 4 && HasBlocker` Digimon (DCGO omits the printed "black" test —
//!   printed text governs, the colour gate is kept), play free, rest → trash.
//! - Inherited `OnTappedAnyone` OPT, opponent's turn, any own Digimon: optional
//!   `PlayByEffect` of a `Level <= 5 && HasBlocker` hand Digimon; `RemoveUse()`
//!   when nothing is played.
//!
//! # Patterns
//! - H: printed <Collision>, <Blocker>; Rule-granted trait.
//! - A1: reveal-3 optional free play, rest trashed.
//! - G4/E2: inherited OPT on-suspend free play from hand.

#![allow(dead_code, unused_imports)]

use super::orphan_support::*;
use digimon_dsl::compiled::{CompiledClause, CompiledColor, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "EX13-056";

fn blocker(id: &str, color: CardColor, level: u8) -> CardData {
    with_text(digimon(id, color, level, 1000 * level as i32, &[]), "＜Blocker＞")
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-056 YAML loads")
        .add_card(blocker("B4", CardColor::Black, 4))
        .add_card(blocker("B5", CardColor::Black, 5))
        .add_card(blocker("B6", CardColor::Black, 6))
        .add_card(blocker("R4", CardColor::Red, 4))
        .add_card(digimon("NB4", CardColor::Black, 4, 4000, &[]))
        .add_card(named("ALLY", "Allymon", CardColor::Black, 3, 3000))
        .add_card(named("TOP", "Topmon", CardColor::Black, 6, 10000))
        .add_card(named("FILL", "Filler", CardColor::Black, 3, 3000))
}

fn start(hand: &[&str], top: &[&str]) -> DebugRunner {
    let deck = deck_with_top(top, "FILL", 4);
    builder()
        .hand(0, hand)
        .deck(0, &deck)
        .deck(1, &["FILL"; 6])
        .memory(3)
        .start()
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_056_printed_metadata_keywords_and_rule_trait() {
    let mut r = start(&[], &[]);
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(5), Some(7000), Some(7)));
    assert_eq!(c.color, vec![CompiledColor::Black]);
    assert_eq!(c.traits, vec!["Mine".to_string(), "Machine".to_string()], "(Rule) Has [Machine]");
    let t: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(t.len(), 2);
    assert_eq!((t[0].scope, t[0].once_per_turn), (CompiledScope::FaceUp, true));
    assert_eq!((t[1].scope, t[1].once_per_turn), (CompiledScope::Inherited, true));
    let h = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(h, Keyword::Collision));
    assert!(r.game.has_keyword(h, Keyword::Blocker));
}

// ─── Section 2/3/5 — self-suspend reveal ─────────────────────────────────────

#[test]
fn ex13_056_suspending_reveals_and_offers_only_lv4_or_lower_black_printed_blockers() {
    let mut r = start(&[], &["B4", "B5", "R4"]);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.game.suspend(me);
    assert_eq!(offered_reveal_ids(&r), vec!["B4".to_string()]);
    assert!(r.pending_is_optional(), "'You may play'");
}

#[test]
fn ex13_056_plays_the_pick_free_and_trashes_the_rest() {
    let mut r = start(&[], &["NB4", "B4", "B6"]);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.game.suspend(me);
    pick_revealed(&mut r, "B4");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"B4".to_string()));
    assert_eq!(r.memory(), 3, "without paying the cost");
    let mut trash = trash_ids(&r, 0);
    trash.sort();
    assert_eq!(trash, vec!["B6".to_string(), "NB4".to_string()]);
}

#[test]
fn ex13_056_declining_trashes_all_three() {
    let mut r = start(&[], &["B4", "FILL", "FILL"]);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.game.suspend(me);
    decline(&mut r);
    let _ = r.auto_resolve();
    assert_eq!(r.trash_size(0), 3);
    assert_eq!(r.deck_size(0), 4);
}

#[test]
fn ex13_056_other_digimon_suspending_does_not_trigger_the_reveal() {
    let mut r = start(&[], &["B4", "FILL", "FILL"]);
    r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    r.game.suspend(ally);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.deck_size(0), 7);
}

#[test]
fn ex13_056_reveal_is_once_per_turn() {
    let mut r = start(&[], &["FILL", "FILL", "FILL", "B4"]);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.game.suspend(me);
    let _ = r.auto_resolve();
    set_suspended(&mut r, me, false);
    r.game.suspend(me);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.trash_size(0), 3, "only one reveal");
}

// ─── Section 2/3/5 — inherited free play from hand ───────────────────────────

fn inherited_setup(hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = start(hand, &[]);
    let carrier = r.place_stack(0, &[CARD_ID, "TOP"]);
    end_turn_declining(&mut r);
    r.game.memory = 3;
    (r, carrier)
}

#[test]
fn ex13_056_inherited_opponents_turn_suspend_plays_lv5_black_blocker_free() {
    let (mut r, carrier) = inherited_setup(&["B5", "B6", "R4", "NB4"]);
    r.game.suspend(carrier);
    assert!(r.pending_is_optional());
    assert_eq!(offered_hand_ids(&r), vec!["B5".to_string()], "Lv.5- black printed <Blocker>");
    pick_hand(&mut r, "B5");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"B5".to_string()));
    assert_eq!(r.memory(), 3, "free");
}

#[test]
fn ex13_056_inherited_not_on_your_own_turn() {
    let mut r = start(&["B5"], &[]);
    let carrier = r.place_stack(0, &[CARD_ID, "TOP"]);
    r.game.suspend(carrier);
    assert!(r.pending_selection().is_none(), "[Opponent's Turn]");
}

#[test]
fn ex13_056_inherited_decline_refunds_once_per_turn() {
    let (mut r, carrier) = inherited_setup(&["B5", "B4"]);
    r.game.suspend(carrier);
    decline(&mut r);
    let _ = r.auto_resolve();
    let ally = r.place_on_field(0, "ALLY", Some(0));
    r.game.suspend(ally);
    assert!(r.pending_is_optional(), "nothing played → DCGO RemoveUse(): still available");
}

#[test]
fn ex13_056_inherited_once_per_turn_after_a_play() {
    let (mut r, carrier) = inherited_setup(&["B5", "B4"]);
    r.game.suspend(carrier);
    pick_hand(&mut r, "B5");
    let _ = r.auto_resolve();
    let ally = r.place_on_field(0, "ALLY", Some(0));
    r.game.suspend(ally);
    assert!(r.pending_selection().is_none(), "OPT spent");
}
