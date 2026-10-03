//! EX13-073 Tai Kamiya & Matt Ishida — Tamer, Black/Purple, cost 4.
//! Traits: ADVENTURE.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-073.md`)
//!
//! [Start of Your Main Phase] If you have an [ADVENTURE] trait Digimon, gain 1
//! memory. [Your Turn] When your [ADVENTURE] trait Digimon or Tamers are
//! played, by suspending this Tamer, <Draw 1> and trash 1 card in your hand.
//! [All Turns] All of your level 5 or higher [ADVENTURE] trait Digimon gain
//! <Rush> and <Blocker>.
//!
//! Security Effect: [Security] Play this card without paying the cost.
//!
//! Official Q&A: "Yes, it triggers." (this Tamer is itself an [ADVENTURE]
//! Tamer, so its own play triggers the [Your Turn] clause).
//!
//! # DCGO C# reference
//! None at b9a0638cd (no `EX13_073.cs`); printed text governs. Idioms:
//! BT21-084 (start-of-main memory, suspend-self cost → draw → trash 1 from
//! hand, security self-play), BT24-041 (trait-filtered keyword auras).
//!
//! # Patterns
//! - B1: [Start of Your Main Phase] conditional memory gain.
//! - B3 + E2: optional suspend-self cost on an ally-played trigger.
//! - D4: level/trait-filtered keyword auras.  - H: [Security] self-play.

#![allow(dead_code, unused_imports)]

use super::orphan_support::*;
use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledColor, CompiledTiming};
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX13-073";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-073 YAML loads")
        .add_card(digimon("ADV4", CardColor::Black, 4, 5000, &["ADVENTURE"]))
        .add_card(digimon("ADV5", CardColor::Black, 5, 7000, &["ADVENTURE"]))
        .add_card(digimon("PLAIN5", CardColor::Black, 5, 7000, &["Dragon"]))
        .add_card({
            let mut t = tamer("ADV-T", CardColor::Black, 2);
            t.traits = vec!["ADVENTURE".into()];
            t
        })
        .add_card(tamer("PLAIN-T", CardColor::Black, 2))
        .add_card(named("DRAWN", "Drawnmon", CardColor::Black, 3, 3000))
        .add_card(named("ATK", "Attacker", CardColor::Red, 5, 7000))
        .add_card(named("FILL", "Filler", CardColor::Black, 3, 3000))
}

fn start(hand: &[&str]) -> DebugRunner {
    builder()
        .hand(0, hand)
        .deck(0, &["DRAWN"; 6])
        .deck(1, &["FILL"; 6])
        .memory(6)
        .start()
}

fn fire_start_of_main(r: &mut DebugRunner, h: PermanentHandle) {
    r.game
        .enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_073_is_a_black_purple_adventure_tamer() {
    let r = start(&[]);
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.kind, CompiledCardKind::Tamer);
    assert_eq!(c.color, vec![CompiledColor::Black, CompiledColor::Purple]);
    assert_eq!(c.traits, vec!["ADVENTURE".to_string()]);
    assert_eq!(c.cost, Some(4));
    let t: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(t.len(), 3, "start-of-main, ally-played, security");
    assert!(t[1].optional, "'by suspending this Tamer' is a may-pay cost");
    assert!(!t[1].once_per_turn);
}

// ─── Section 2/3 — [Start of Your Main Phase] ────────────────────────────────

#[test]
fn ex13_073_start_of_main_with_adventure_digimon_gains_1() {
    let mut r = start(&[]);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "ADV4", Some(0));
    let m0 = r.memory();
    fire_start_of_main(&mut r, me);
    assert_eq!(r.memory(), m0 + 1);
}

#[test]
fn ex13_073_start_of_main_without_adventure_digimon_gains_nothing() {
    let mut r = start(&[]);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "PLAIN5", Some(0));
    r.place_on_field(0, "ADV-T", Some(0)); // an ADVENTURE Tamer is not a Digimon
    let m0 = r.memory();
    fire_start_of_main(&mut r, me);
    assert_eq!(r.memory(), m0);
}

// ─── Section 2/3 — [Your Turn] ally played → suspend: draw 1, trash 1 ───────

#[test]
fn ex13_073_adventure_digimon_played_suspend_to_draw_and_trash() {
    let mut r = start(&["ADV4", "FILL"]);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    play_card(&mut r, 0, "ADV4");
    assert!(r.pending_is_optional(), "may pay the suspend cost");
    accept(&mut r);
    assert!(is_suspended(&r, me), "cost: this Tamer suspended");
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand), "trash 1 card in hand");
    assert!(hand_ids(&r, 0).contains(&"DRAWN".to_string()), "<Draw 1> first");
    pick_hand(&mut r, "FILL");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["DRAWN".to_string()]);
    assert!(trash_ids(&r, 0).contains(&"FILL".to_string()));
}

#[test]
fn ex13_073_adventure_tamer_played_also_triggers() {
    let mut r = start(&["ADV-T"]);
    r.place_on_field(0, CARD_ID, Some(0));
    play_card(&mut r, 0, "ADV-T");
    assert!(r.pending_is_optional());
}

#[test]
fn ex13_073_its_own_play_triggers() {
    let mut r = start(&[CARD_ID, "FILL"]);
    play_card(&mut r, 0, CARD_ID);
    assert!(r.pending_is_optional(), "Official Q&A: it triggers on its own play");
    accept(&mut r);
    let me = handle_of(&r, 0, CARD_ID);
    assert!(is_suspended(&r, me));
}

#[test]
fn ex13_073_declining_keeps_tamer_unsuspended_and_hand_unchanged() {
    let mut r = start(&["ADV4", "FILL"]);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    play_card(&mut r, 0, "ADV4");
    decline(&mut r);
    let _ = r.auto_resolve();
    assert!(!is_suspended(&r, me));
    assert_eq!(hand_ids(&r, 0), vec!["FILL".to_string()]);
}

#[test]
fn ex13_073_non_adventure_play_does_not_trigger() {
    let mut r = start(&["PLAIN5", "PLAIN-T"]);
    r.place_on_field(0, CARD_ID, Some(0));
    play_card(&mut r, 0, "PLAIN-T");
    assert!(r.pending_selection().is_none());
}

#[test]
fn ex13_073_suspended_tamer_cannot_pay() {
    let mut r = start(&["ADV4"]);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    set_suspended(&mut r, me, true);
    play_card(&mut r, 0, "ADV4");
    assert!(r.pending_selection().is_none());
}

#[test]
fn ex13_073_not_once_per_turn() {
    let mut r = start(&["ADV4", "ADV-T", "FILL", "FILL"]);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    play_card(&mut r, 0, "ADV4");
    accept(&mut r);
    pick_hand(&mut r, "FILL");
    let _ = r.auto_resolve();
    set_suspended(&mut r, me, false);
    play_card(&mut r, 0, "ADV-T");
    assert!(r.pending_is_optional(), "fires again when payable");
}

// ─── Section 2 — [All Turns] auras ───────────────────────────────────────────

#[test]
fn ex13_073_level_5_adventure_digimon_gain_rush_and_blocker() {
    let mut r = start(&[]);
    r.place_on_field(0, CARD_ID, Some(0));
    let a5 = r.place_on_field(0, "ADV5", Some(0));
    let a4 = r.place_on_field(0, "ADV4", Some(0));
    let p5 = r.place_on_field(0, "PLAIN5", Some(0));
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(a5, Keyword::Rush));
    assert!(r.game.has_keyword(a5, Keyword::Blocker));
    assert!(!r.game.has_keyword(a4, Keyword::Rush), "level 4");
    assert!(!r.game.has_keyword(p5, Keyword::Blocker), "not [ADVENTURE]");
}

#[test]
fn ex13_073_aura_also_applies_on_the_opponents_turn() {
    let mut r = start(&[]);
    r.place_on_field(0, CARD_ID, Some(0));
    let a5 = r.place_on_field(0, "ADV5", Some(0));
    end_turn_declining(&mut r);
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(a5, Keyword::Blocker));
}

// ─── Section 3 — [Security] ──────────────────────────────────────────────────

#[test]
fn ex13_073_security_plays_without_paying() {
    let mut r = builder()
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &[CARD_ID])
        .memory(0)
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(atk, 1, false);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).contains(&CARD_ID.to_string()));
    assert_eq!(r.memory(), 0);
}
