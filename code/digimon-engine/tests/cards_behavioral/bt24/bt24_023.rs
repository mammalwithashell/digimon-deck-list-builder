//! BT24-023 Calmaramon — Lv.4 Blue, Aquatic/Titan/TS, DP 7000, cost 6.
//!
//! Digivolve: Blue Lv.3 / cost 3; [Digivolve] [Lanamon]: Cost 1 /
//!   Lv.3 w/[TS] trait: Cost 3.
//! <Blocker> <Decode ([Lanamon])> [On Play] [When Digivolving] Return 1 of
//!   your opponent's level 4 or lower Digimon to the bottom of the deck. Then,
//!   if played by effects, 1 of their Digimon or Tamers can't suspend until
//!   their turn ends.
//! Inherited: <Jamming>.
//!
//! DCGO: BT24/Blue/BT24_023.cs. Deliberate divergence: DCGO registers the
//! [On Play]/[When Digivolving] activations as optional; the printed text has
//! no "you may", so the clause is mandatory here.

#![allow(dead_code, unused_imports)]

#[path = "../bt26/support.rs"]
mod support;

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, EffectTiming, Keyword, ModifierType, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;
use support::*;

const CARD_ID: &str = "BT24-023";

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-023 in embedded DSL pack")
        .add_card(filler("FILLER"))
        .add_card(digimon("LANAMON", "Lanamon", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("TS3", "Ts Three", CardColor::Green, 3, 3, &["TS"]))
        .add_card(digimon("BLUE3", "Blue Three", CardColor::Blue, 3, 3, &[]))
        .add_card(digimon("RED3", "Red Three", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("TS2", "Ts Two", CardColor::Green, 2, 0, &["TS"]))
        .add_card(digimon("OPP3", "Opp Three", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("OPP4", "Opp Four", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("OPP5", "Opp Five", CardColor::Red, 5, 7, &[]))
        .add_card(tamer("OPP-TAMER", "Opp Tamer", CardColor::Red, &[]))
        .deck(0, &["FILLER"; 8])
        .deck(1, &["FILLER"; 8])
        .memory(10)
}

fn setup() -> DebugRunner {
    let mut r = base().start();
    r.set_first_player(0);
    r
}

fn clause(idx: usize) -> CompiledClause {
    setup().compiled_card(CARD_ID).unwrap().effects[idx].clone()
}

fn digivolve_onto(base_id: &str) -> Option<i16> {
    let mut r = setup();
    let b = r.place_on_field(0, base_id, Some(0));
    push_hand(&mut r, 0, CARD_ID);
    let before = r.memory();
    if !r
        .game
        .digivolve_from_hand(0, 0, b.index as usize, PlaySource::ByHand)
    {
        return None;
    }
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, b), CARD_ID);
    Some(before - r.memory())
}

// ─── Metadata / structure ────────────────────────────────────────────────────

#[test]
fn bt24_023_metadata() {
    let r = setup();
    let c = r.compiled_card(CARD_ID).unwrap();
    assert_eq!(c.level, Some(4));
    assert_eq!(c.color.len(), 1);
    assert_eq!(c.traits, vec!["Aquatic", "Titan", "TS"]);
    assert_eq!(c.effects.len(), 4);
}

#[test]
fn bt24_023_on_play_when_digivolving_structure() {
    let CompiledClause::Triggered(t) = clause(2) else {
        panic!("clause 2 must be triggered")
    };
    assert_eq!(
        t.when,
        vec![CompiledTiming::OnPlay, CompiledTiming::WhenDigivolving]
    );
    assert!(!t.optional, "no 'you may' in the printed text");
    assert!(!t.once_per_turn);
}

// ─── Digivolve paths ─────────────────────────────────────────────────────────

#[test]
fn bt24_023_digivolves_from_lanamon_for_one() {
    assert_eq!(digivolve_onto("LANAMON"), Some(1));
}

#[test]
fn bt24_023_digivolves_from_lv3_ts_for_three() {
    assert_eq!(digivolve_onto("TS3"), Some(3));
}

#[test]
fn bt24_023_digivolves_from_blue_lv3_for_three() {
    assert_eq!(digivolve_onto("BLUE3"), Some(3));
}

#[test]
fn bt24_023_cannot_digivolve_from_plain_red_lv3_or_lv2_ts() {
    assert_eq!(digivolve_onto("RED3"), None);
    assert_eq!(digivolve_onto("TS2"), None);
}

// ─── Keywords ────────────────────────────────────────────────────────────────

#[test]
fn bt24_023_has_blocker_but_not_own_jamming() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(h, Keyword::Blocker));
    assert!(!r.game.has_keyword(h, Keyword::Jamming), "Jamming is inherited only");
}

#[test]
fn bt24_023_inherited_jamming_on_host() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "OPP5"]);
    assert!(r.game.has_keyword(host, Keyword::Jamming));
    assert!(!r.game.has_keyword(host, Keyword::Blocker), "Blocker is not inherited");
}

// ─── <Decode ([Lanamon])> ────────────────────────────────────────────────────

#[test]
fn bt24_023_decode_plays_lanamon_source_on_non_battle_leave() {
    let mut r = setup();
    let h = r.place_stack(0, &["LANAMON", CARD_ID]);
    let mem = r.memory();
    r.game.return_to_hand(h);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement));
    assert!(r.pending_is_optional(), "Decode is a 'may'");
    let accept = non_pass(&r)[0];
    r.execute_action(0, accept).unwrap();
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"LANAMON".to_string()));
    assert_eq!(r.memory(), mem, "played without paying the cost");
}

#[test]
fn bt24_023_decode_declined_plays_nothing() {
    let mut r = setup();
    let h = r.place_stack(0, &["LANAMON", CARD_ID]);
    r.game.return_to_hand(h);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement));
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"LANAMON".to_string()));
}

#[test]
fn bt24_023_decode_requires_a_lanamon_source() {
    let mut r = setup();
    let h = r.place_stack(0, &["RED3", CARD_ID]);
    r.game.return_to_hand(h);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).is_empty());
}

// ─── [On Play] / [When Digivolving] ──────────────────────────────────────────

#[test]
fn bt24_023_on_play_from_hand_bottom_decks_lv4_or_lower_without_lock() {
    let mut r = setup();
    let opp4 = r.place_on_field(1, "OPP4", Some(0));
    r.place_on_field(1, "OPP5", Some(0));
    let tamer = r.place_on_field(1, "OPP-TAMER", Some(0));
    push_hand(&mut r, 0, CARD_ID);
    r.play(0, 0).expect("play Calmaramon");
    let v = r.pending_selection_view().expect("bottom-deck pick");
    assert!(!v.is_optional, "mandatory");
    assert_eq!(non_pass(&r).len(), 1, "only the Lv.4 — Lv.5 is excluded");
    pick_side_field(&mut r, 0, opp4);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP5".to_string(), "OPP-TAMER".to_string()]);
    assert_eq!(deck_ids(&r, 1)[0], "OPP4", "to the BOTTOM of the deck");
    assert!(r.pending_selection_view().is_none(), "hand play ⇒ no lock tail");
    let tamer = r.perm_handle(1, 1);
    assert!(!r.game.modifiers.has(tamer, ModifierType::CannotSuspend));
}

#[test]
fn bt24_023_on_play_by_effect_also_locks_digimon_or_tamer() {
    let mut r = setup();
    let opp3 = r.place_on_field(1, "OPP3", Some(0));
    r.place_on_field(1, "OPP5", Some(0));
    r.place_on_field(1, "OPP-TAMER", Some(0));
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.fire_play_event_triggers(0, me.index as usize, true, false);
    pick_side_field(&mut r, 0, opp3);
    // Field is now [OPP5, OPP-TAMER].
    let v = r.pending_selection_view().expect("can't-suspend pick");
    assert!(!v.is_optional);
    assert_eq!(non_pass(&r).len(), 2, "Digimon or Tamer");
    let tamer = r.perm_handle(1, 1);
    pick_side_field(&mut r, 0, tamer);
    let _ = r.auto_resolve();
    assert!(r.game.modifiers.has(tamer, ModifierType::CannotSuspend));
    assert!(!r
        .game
        .modifiers
        .has(r.perm_handle(1, 0), ModifierType::CannotSuspend));
}

#[test]
fn bt24_023_lock_tail_runs_even_without_bottom_deck_target() {
    // "Then" — the tail is not contingent on the bottom-deck succeeding.
    let mut r = setup();
    let opp5 = r.place_on_field(1, "OPP5", Some(0));
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.fire_play_event_triggers(0, me.index as usize, true, false);
    let v = r.pending_selection_view().expect("can't-suspend pick");
    assert!(!v.is_optional);
    pick_side_field(&mut r, 0, opp5);
    let _ = r.auto_resolve();
    assert!(r.game.modifiers.has(opp5, ModifierType::CannotSuspend));
    assert_eq!(field_ids(&r, 1), vec!["OPP5".to_string()]);
}

#[test]
fn bt24_023_lock_lasts_through_opponents_turn_only() {
    let mut r = setup();
    let opp5 = r.place_on_field(1, "OPP5", Some(0));
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.fire_play_event_triggers(0, me.index as usize, true, false);
    pick_side_field(&mut r, 0, opp5);
    let _ = r.auto_resolve();
    r.end_turn();
    let _ = r.auto_resolve();
    assert!(
        r.game.modifiers.has(opp5, ModifierType::CannotSuspend),
        "still locked during the opponent's turn"
    );
    r.end_turn();
    let _ = r.auto_resolve();
    assert!(
        !r.game.modifiers.has(opp5, ModifierType::CannotSuspend),
        "expires when their turn ends"
    );
}

#[test]
fn bt24_023_when_digivolving_bottom_decks_but_never_locks() {
    let mut r = setup();
    let opp4 = r.place_on_field(1, "OPP4", Some(0));
    r.place_on_field(1, "OPP-TAMER", Some(0));
    let b = r.place_on_field(0, "BLUE3", Some(0));
    push_hand(&mut r, 0, CARD_ID);
    assert!(r
        .game
        .digivolve_from_hand(0, 0, b.index as usize, PlaySource::ByHand));
    let v = r.pending_selection_view().expect("bottom-deck pick");
    assert!(!v.is_optional);
    pick_side_field(&mut r, 0, opp4);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP-TAMER".to_string()]);
    assert_eq!(deck_ids(&r, 1)[0], "OPP4");
    assert!(!r
        .game
        .modifiers
        .has(r.perm_handle(1, 0), ModifierType::CannotSuspend));
}

#[test]
fn bt24_023_on_play_with_no_opponent_digimon_does_nothing() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    r.play(0, 0).expect("play Calmaramon");
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.deck_size(1), 8);
}
