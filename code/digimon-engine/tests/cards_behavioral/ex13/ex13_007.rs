//! EX13-007 Guilmon — Digimon, Lv.3, Red, 1000 DP, cost 3.
//! Traits: Reptile. Rookie / Virus. Digivolve: Red Lv.2 cost 0.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-007.md`)
//!
//! [When Moving] [On Play] By trashing 1 card in your hand, you may return 1
//! Digimon card with [Gallantmon] in its name or 1 red Tamer card from your
//! trash to the hand.
//!
//! Inherited Effect: [All Turns] Add 2000 to this Digimon's DP deletion
//! effects' maximums.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Red/EX13_007.cs
//! - Shared WM/OP gated on `HandCards.Count >= 1`: SelectHandEffect Discard
//!   (canNoSelect) → if a card was trashed, SelectCardEffect AddHand from trash
//!   (canNoSelect) over `IsDigimon && ContainsCardName("Gallantmon") ||
//!   IsTamer && Red`.
//! - Inherited `ChangeDPDeleteEffectMaxDPClass`: +2000 to every DP deletion
//!   effect whose source is this Digimon's stack (`Player.MaxDP_DeleteEffect`).
//!
//! # Patterns
//! - cost-gated optional hand trash → optional trash-to-hand pick.
//! - G-ENGINE-DP-DELETION-MAX-MODIFIER (inherited deletion-max +2000).

#![allow(dead_code, unused_imports)]

use super::orphan_support::*;
use digimon_dsl::compiled::{
    CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{encode_digivolve, PASS, TRASH_EFFECT_START};
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-007";
const GROWLMON: &str = "EX13-010";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-007 YAML loads")
        .dsl_card(GROWLMON)
        .expect("EX13-010 YAML loads")
        .add_card(named("GALLANT", "Gallantmon", CardColor::Red, 6, 12000))
        .add_card(named("GALLANT-X", "Gallantmon X", CardColor::Red, 6, 12000))
        .add_card(named("PLAIN-D", "Plainmon", CardColor::Red, 4, 5000))
        .add_card(tamer("RED-T", CardColor::Red, 3))
        .add_card(tamer("BLUE-T", CardColor::Blue, 3))
        .add_card(named("RED-L3", "Redmon", CardColor::Red, 3, 3000))
        .add_card(named("OPP-4K", "OppFour", CardColor::Blue, 4, 4000))
        .add_card(named("OPP-6K", "OppSix", CardColor::Blue, 4, 6000))
        .add_card(named("OPP-7K", "OppSeven", CardColor::Blue, 4, 7000))
        .add_card(named("FILL", "Filler", CardColor::Red, 3, 3000))
}

fn start(hand: &[&str], mem: i16) -> DebugRunner {
    builder()
        .hand(0, hand)
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(mem)
        .start()
}

fn offered_trash_ids(r: &DebugRunner) -> Vec<String> {
    let view = r.pending_selection_view().expect("trash prompt pending");
    let trash = trash_ids(r, view.selecting_player);
    let mut out: Vec<String> = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .filter_map(|&a| a.checked_sub(TRASH_EFFECT_START))
        .filter_map(|i| trash.get(i as usize).cloned())
        .collect();
    out.sort();
    out
}

fn digivolve_onto(r: &mut DebugRunner, id: &str, base: PermanentHandle) {
    let slot = hand_index(r, 0, id) as u16;
    r.game
        .decode_action(encode_digivolve(slot, base.index as u16), 0);
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_007_printed_metadata_and_clause_shape() {
    let r = start(&[], 5);
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(3), Some(1000), Some(3)));
    assert_eq!(c.color, vec![CompiledColor::Red]);
    assert_eq!(c.traits, vec!["Reptile".to_string()]);
    let CompiledClause::Triggered(t) = &c.effects[0] else {
        panic!("clause 0 triggered");
    };
    assert!(t.when.contains(&CompiledTiming::OnMove));
    assert!(t.when.contains(&CompiledTiming::OnPlay));
    assert!(
        !t.optional,
        "DCGO optional: false — the hand pick itself is declinable"
    );
    let CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
        scope,
        modifier,
        modifier_value,
        ..
    }) = &c.effects[1]
    else {
        panic!("clause 1 is the inherited aura: {:?}", c.effects[1]);
    };
    assert_eq!(*scope, CompiledScope::Inherited);
    assert_eq!(modifier.as_deref(), Some("ChangeDPDeleteEffectMaxDP"));
    assert_eq!(*modifier_value, Some(2000));
}

// ─── Section 2 — [On Play] trash 1 → return [Gallantmon] / red Tamer ─────────

#[test]
fn ex13_007_on_play_trash_one_then_return_gallantmon_or_red_tamer() {
    let mut r = start(&[CARD_ID, "FILL"], 5);
    r.game.players[0].trash.clear();
    for id in ["GALLANT", "RED-T", "BLUE-T", "PLAIN-D"] {
        r.inject_trash(0, id);
    }
    play_card(&mut r, 0, CARD_ID);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    assert!(
        r.pending_is_optional(),
        "'you may' — the hand trash is declinable"
    );
    pick_hand(&mut r, "FILL");
    assert!(trash_ids(&r, 0).contains(&"FILL".to_string()), "cost paid");
    assert_eq!(
        offered_trash_ids(&r),
        vec!["GALLANT".to_string(), "RED-T".to_string()],
        "[Gallantmon]-named Digimon or red Tamer only"
    );
    assert!(r.pending_is_optional());
    pick_trash(&mut r, "GALLANT");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["GALLANT".to_string()]);
}

#[test]
fn ex13_007_name_contains_gallantmon_and_red_tamer_both_selectable() {
    let mut r = start(&[CARD_ID, "FILL"], 5);
    r.game.players[0].trash.clear();
    r.inject_trash(0, "GALLANT-X");
    r.inject_trash(0, "RED-T");
    play_card(&mut r, 0, CARD_ID);
    pick_hand(&mut r, "FILL");
    assert_eq!(
        offered_trash_ids(&r),
        vec!["GALLANT-X".to_string(), "RED-T".to_string()]
    );
    pick_trash(&mut r, "RED-T");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["RED-T".to_string()]);
}

#[test]
fn ex13_007_declining_the_trash_cost_does_nothing() {
    let mut r = start(&[CARD_ID, "FILL"], 5);
    r.game.players[0].trash.clear();
    r.inject_trash(0, "GALLANT");
    play_card(&mut r, 0, CARD_ID);
    decline(&mut r);
    let _ = r.auto_resolve();
    assert!(r.pending_selection().is_none());
    assert_eq!(hand_ids(&r, 0), vec!["FILL".to_string()], "no card trashed");
    assert_eq!(
        trash_ids(&r, 0),
        vec!["GALLANT".to_string()],
        "nothing returned"
    );
}

#[test]
fn ex13_007_empty_hand_offers_nothing() {
    let mut r = start(&[CARD_ID], 5);
    r.game.players[0].trash.clear();
    r.inject_trash(0, "GALLANT");
    play_card(&mut r, 0, CARD_ID);
    assert!(
        r.pending_selection().is_none(),
        "no card to trash → no activation"
    );
    assert_eq!(trash_ids(&r, 0), vec!["GALLANT".to_string()]);
}

#[test]
fn ex13_007_the_just_trashed_card_can_be_returned() {
    let mut r = start(&[CARD_ID, "GALLANT"], 5);
    r.game.players[0].trash.clear();
    play_card(&mut r, 0, CARD_ID);
    pick_hand(&mut r, "GALLANT");
    assert_eq!(offered_trash_ids(&r), vec!["GALLANT".to_string()]);
    pick_trash(&mut r, "GALLANT");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["GALLANT".to_string()]);
}

#[test]
fn ex13_007_return_is_optional_after_paying() {
    let mut r = start(&[CARD_ID, "FILL"], 5);
    r.game.players[0].trash.clear();
    r.inject_trash(0, "GALLANT");
    play_card(&mut r, 0, CARD_ID);
    pick_hand(&mut r, "FILL");
    decline(&mut r);
    let _ = r.auto_resolve();
    assert!(hand_ids(&r, 0).is_empty());
    let mut t = trash_ids(&r, 0);
    t.sort();
    assert_eq!(t, vec!["FILL".to_string(), "GALLANT".to_string()]);
}

// ─── Section 2 — [When Moving] ───────────────────────────────────────────────

#[test]
fn ex13_007_when_moving_fires_the_same_effect() {
    let mut r = start(&["FILL"], 5);
    r.game.players[0].trash.clear();
    r.inject_trash(0, "RED-T");
    r.place_in_breeding(0, CARD_ID);
    assert!(r.move_from_breeding(0), "Guilmon moves to the battle area");
    pick_hand(&mut r, "FILL");
    pick_trash(&mut r, "RED-T");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["RED-T".to_string()]);
}

#[test]
fn ex13_007_another_digimon_moving_does_not_fire() {
    let mut r = start(&["FILL"], 5);
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_in_breeding(0, "RED-L3");
    assert!(r.move_from_breeding(0));
    assert!(
        r.pending_selection().is_none(),
        "[When Moving] is self-only"
    );
}

// ─── Section 3 — Inherited: +2000 to DP deletion maximums ────────────────────

#[test]
fn ex13_007_inherited_installs_the_deletion_max_delta_on_the_carrier_only() {
    let mut r = start(&[], 5);
    let alone = r.place_on_field(0, CARD_ID, Some(0));
    let carrier = r.place_stack(0, &[CARD_ID, "PLAIN-D"]);
    r.game.tick_declarative_effects();
    assert_eq!(
        r.game.dp_delete_effect_max_bonus(alone),
        0,
        "inherited only"
    );
    assert_eq!(r.game.dp_delete_effect_max_bonus(carrier), 2000);
}

#[test]
fn ex13_007_inherited_raises_growlmons_deletion_cap_to_6000() {
    let mut r = start(&[GROWLMON], 5);
    let base = r.place_on_field(0, CARD_ID, Some(0));
    let o4 = r.place_on_field(1, "OPP-4K", Some(0));
    let o6 = r.place_on_field(1, "OPP-6K", Some(0));
    let o7 = r.place_on_field(1, "OPP-7K", Some(0));
    digivolve_onto(&mut r, GROWLMON, base);
    assert_eq!(r.pending_kind(), Some(SelectionKind::OppField));
    assert!(field_offered(&r, o4));
    assert!(field_offered(&r, o6), "4000 + 2000 (Guilmon inherited)");
    assert!(!field_offered(&r, o7), "7000 exceeds 6000");
    pick_field(&mut r, o6);
    let _ = r.auto_resolve();
    assert_eq!(
        field_ids(&r, 1),
        vec!["OPP-4K".to_string(), "OPP-7K".to_string()],
        "the 6000-DP Digimon was deleted"
    );
}
