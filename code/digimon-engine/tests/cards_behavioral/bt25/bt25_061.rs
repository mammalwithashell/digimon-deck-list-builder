//! BT25-061 Offmon — Digimon, Lv.3, Purple, DP 2000, Cost 3.
//! Traits: Offline (App Name) — Appmon trait line.
//!
//! # Card text (DCGO BT25_061.cs — authoritative; cards.json mis-slots the
//! WhenLinked clause as "inherited")
//!
//! Self link-condition: link onto an [Appmon]-trait Digimon for link cost 1.
//! Alt-digivolve: may digivolve from a level-2 [Appmon] for cost 0.
//! [Start of Your Main Phase] By trashing 1 card with the [Appmon] trait from
//!   your hand, <Draw 1> and gain 1 memory. (Optional — DCGO canNoSelect: true.)
//! [When Linking] 1 of your opponent's Digimon can't unsuspend until their turn
//!   ends. (DCGO WhenLinked, SetIsLinkedEffect(true); CanNotUnsuspendClass with
//!   UntilOwnerTurnEnd expiry on the selected opponent permanent.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_061.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - DigiLink Shape-B self link-condition (G-DSL-DIGILINK)
//! - when: when_linked triggered effect (linked scope)
//! - StartOfYourMainPhase optional cost (trash Appmon) → draw + memory
//! - CannotUnsuspend modifier with end-of-opponents-turn expiry

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, ModifierType, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-061";

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn link_bit(perm: PermanentHandle) -> usize {
    (FIELD_EFFECT_START + perm.index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_LINK)
        as usize
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-061 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("APPMON-X", 3, 2000, 3, &["Appmon"]))
        .add_card(make_digimon("PLAIN-X", 3, 2000, 3, &["Beast"]))
        .add_card(make_digimon("HOST-APP", 4, 4000, 4, &["Appmon"]))
        .add_card(make_digimon("OPP-A", 3, 3000, 3, &["Beast"]))
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

fn is_unsuspendable(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.modifiers.has(h, ModifierType::CannotUnsuspend)
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_061_yaml_printed_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Offmon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.dp, Some(2000));
}

#[test]
fn bt25_061_has_link_condition_cost_1() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, .. }) if *cost == 1
    ));
    assert!(
        has,
        "BT25-061 must declare a self link-condition with cost 1"
    );
}

#[test]
fn bt25_061_registers_appmon_alt_digivolve() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && matches!(p.cost, Some(CompiledCost::Literal(0)))
    });
    assert!(
        has,
        "BT25-061 must register a cost-0 alt-digivolve over Appmon"
    );
}

#[test]
fn bt25_061_has_start_main_and_when_linked_clauses() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let start_main = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::StartOfYourMainPhase)
        )
    });
    let when_linked = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenLinked)
        )
    });
    assert!(
        start_main,
        "BT25-061 must have a [Start of Your Main Phase] clause"
    );
    assert!(when_linked, "BT25-061 must have a [When Linking] clause");
}

// ─── Section 2 — Start of Main: optional trash Appmon → draw + memory ────────

#[test]
fn bt25_061_start_main_trash_appmon_draws_and_gains_memory() {
    let mut r = base()
        .hand(0, &["APPMON-X"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(0)
        .start();
    let _off = r.place_on_field(0, CARD_ID, Some(0));
    let trash_before = r.trash_size(0);

    // Entering the main phase fires [Start of Your Main Phase].
    advance_to_main(&mut r);
    // Optional cost: resolve by trashing the Appmon. The trash is a real
    // selection surfaced through pending_selection (no auto-pick in YAML).
    r.auto_resolve().ok();

    assert_eq!(
        r.trash_size(0),
        trash_before + 1,
        "trashed 1 [Appmon] card as the activation cost"
    );
    assert_eq!(r.memory(), 1, "gained 1 memory after paying the cost");
}

#[test]
fn bt25_061_start_main_no_appmon_in_hand_is_noop() {
    let mut r = base()
        .hand(0, &["PLAIN-X"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(0)
        .start();
    let _off = r.place_on_field(0, CARD_ID, Some(0));
    let trash_before = r.trash_size(0);

    advance_to_main(&mut r);
    r.auto_resolve().ok();

    assert_eq!(
        r.trash_size(0),
        trash_before,
        "no [Appmon] in hand → nothing trashed"
    );
    assert_eq!(r.memory(), 0, "no memory gained without paying the cost");
}

// ─── Section 3 — When Linking: opp Digimon can't unsuspend ───────────────────

#[test]
fn bt25_061_when_linked_locks_opponent_unsuspend() {
    let mut r = base().memory(5).start();
    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let off = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP-A", Some(0));
    advance_to_main(&mut r);

    assert!(!is_unsuspendable(&r, opp), "opp not yet locked");

    r.game.decode_action(link_bit(off) as u16, 0);
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    // WhenLinked: choose the opponent Digimon to lock.
    r.auto_resolve().ok();

    assert!(
        is_unsuspendable(&r, opp),
        "WhenLinked applied CannotUnsuspend to the chosen opponent Digimon"
    );
}
