//! BT25-061 Offmon — Digimon, Lv.3, Black, DP 2000, Cost 3.
//! Trait line (official Bandai DB / card image): Stnd./Appmon | Offline | Game.
//!
//! # Printed text (data/card_bundles/BT25-061.md — authoritative)
//!
//! Digivolve circles: Black Lv.2 / Cost 0 (standard) AND
//!   [Digivolve] Lv.2 w/[Appmon] trait: Cost 0 (black-banner alt condition).
//! [Start of Your Main Phase] By trashing 1 card with the [Appmon] trait from
//!   your hand, <Draw 1> and gain 1 memory. (Optional — DCGO canNoSelect: true;
//!   both payoffs gated on a card actually being trashed.)
//! Link box: <Link> [Appmon] trait: Cost 1 · Link DP +2000 ·
//!   [When Linking] 1 of your opponent's Digimon can't unsuspend until their
//!   turn ends. (DCGO WhenLinked, SetIsLinkedEffect(true), canNoSelect: false
//!   → mandatory when a target exists; CanNotUnsuspendClass with
//!   UntilOwnerTurnEnd expiry on the selected opponent permanent.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_061.cs
//! (Link DP +2000 and the standard Black Lv.2 circle are data-driven in DCGO,
//! not per-card C# — authored in YAML as a scope:linked dp aura and an
//! equal-cost alt_path per pool convention, same as BT21-043 / BT24-097.)
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - DigiLink Shape-B self link-condition (G-DSL-DIGILINK)
//! - scope: linked kind: aura dp_modifier: 2000 (G-LINK-INHERITED-ESS)
//! - when: when_linked triggered effect (linked scope)
//! - StartOfYourMainPhase optional cost (trash Appmon) → memory + draw
//! - Optional-cost decline via PASS (no trash / no memory / no draw)
//! - CannotUnsuspend modifier with end-of-opponents-turn expiry

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START, PASS,
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

/// Drive Offmon's link action onto a host: decode_action → resolve the
/// (mandatory) host pick.
fn do_link(r: &mut DebugRunner, linking: PermanentHandle) {
    r.game.decode_action(link_bit(linking) as u16, 0);
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_061_yaml_printed_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Offmon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.dp, Some(2000));
    assert_eq!(card.cost, Some(3));
    // Printed: Black (bundle + card image) — was mis-authored purple.
    assert_eq!(card.color, vec![CompiledColor::Black], "Offmon is Black");
    // Trait line Stnd./Appmon | Offline; Attribute: Game (official Bandai DB).
    for t in ["Stnd.", "Appmon", "Offline"] {
        assert!(
            card.traits.iter().any(|x| x == t),
            "trait line must include {t:?}"
        );
    }
    assert_eq!(
        card.attribute.as_deref(),
        Some("Game"),
        "printed Attribute: Game"
    );
}

#[test]
fn bt25_061_has_link_condition_cost_1_over_appmon() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, filter, .. })
            if *cost == 1 && filter.trait_has.as_deref() == Some("Appmon")
    ));
    assert!(
        has,
        "BT25-061 must declare a self link-condition with cost 1 over [Appmon] hosts"
    );
}

#[test]
fn bt25_061_has_linked_scope_dp_aura_2000() {
    // Link box: "DP +2000" — scope:linked self-aura on the host.
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let aura = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope, dp_modifier, ..
        }) if *scope == CompiledScope::Linked => Some(*dp_modifier),
        _ => None,
    });
    assert_eq!(
        aura,
        Some(Some(2000)),
        "BT25-061 must declare a linked-scope dp aura of +2000 DP (printed Link DP +2000)"
    );
}

#[test]
fn bt25_061_registers_both_printed_digivolve_circles() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    // Printed standard circle: Black Lv.2 / Cost 0.
    let std_circle = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && matches!(p.cost, Some(CompiledCost::Literal(0)))
            && p.from
                .as_ref()
                .map(|f| f.level_eq == Some(2) && f.color_is == Some(CompiledColor::Black))
                .unwrap_or(false)
    });
    // [Digivolve] Lv.2 w/[Appmon] trait: Cost 0 (DCGO alt condition).
    let appmon_circle = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && matches!(p.cost, Some(CompiledCost::Literal(0)))
            && p.from
                .as_ref()
                .map(|f| f.level_eq == Some(2) && f.trait_has.as_deref() == Some("Appmon"))
                .unwrap_or(false)
    });
    assert!(
        std_circle,
        "BT25-061 must register the printed standard circle: Black Lv.2 / cost 0"
    );
    assert!(
        appmon_circle,
        "BT25-061 must register the [Digivolve] Lv.2 w/[Appmon] trait: cost 0 alt condition"
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
    let when_linked = card.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => {
            t.when.contains(&CompiledTiming::WhenLinked)
                && matches!(t.scope, CompiledScope::Linked)
        }
        _ => false,
    });
    assert!(
        start_main,
        "BT25-061 must have a [Start of Your Main Phase] clause"
    );
    assert!(
        when_linked,
        "BT25-061 must have a scope:linked [When Linking] clause"
    );
}

// ─── Section 2 — Start of Main: optional trash Appmon → memory + draw ────────

#[test]
fn bt25_061_start_main_trash_appmon_draws_and_gains_memory() {
    let mut r = base()
        .hand(0, &["APPMON-X"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(0)
        .start();
    let _off = r.place_on_field(0, CARD_ID, Some(0));
    let trash_before = r.trash_size(0);
    let hand_before = r.hand_size(0);
    let deck_before = r.deck_size(0);

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
    // <Draw 1>: one card left the deck and entered the hand (the hand also
    // lost the trashed Appmon, so its size is net unchanged).
    assert_eq!(
        r.deck_size(0),
        deck_before - 1,
        "drew exactly 1 card from the deck"
    );
    assert_eq!(
        r.hand_size(0),
        hand_before,
        "hand: -1 trashed Appmon, +1 drawn card"
    );
}

#[test]
fn bt25_061_start_main_decline_is_free_noop() {
    // "By trashing ..." is an optional cost (DCGO canNoSelect: true): PASS on
    // the trash pick must skip the cost AND both payoffs.
    let mut r = base()
        .hand(0, &["APPMON-X"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(0)
        .start();
    let _off = r.place_on_field(0, CARD_ID, Some(0));
    let trash_before = r.trash_size(0);
    let hand_before = r.hand_size(0);
    let deck_before = r.deck_size(0);

    advance_to_main(&mut r);
    assert!(
        r.game.pending_selection.is_some(),
        "optional trash cost must surface a pending selection"
    );
    r.execute_action(0, PASS).expect("decline the optional trash");
    r.auto_resolve().ok();

    assert_eq!(r.trash_size(0), trash_before, "declined ⇒ nothing trashed");
    assert_eq!(r.memory(), 0, "declined ⇒ no memory gained");
    assert_eq!(r.deck_size(0), deck_before, "declined ⇒ no draw");
    assert_eq!(r.hand_size(0), hand_before, "declined ⇒ hand untouched");
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

// ─── Section 3 — Link box behavioral ─────────────────────────────────────────

#[test]
fn bt25_061_linked_aura_gives_host_plus_2000_dp() {
    // No opponent Digimon on the board: the [When Linking] pick has no legal
    // target and no-ops, isolating the Link DP +2000 aura.
    let mut r = base().memory(5).start();
    let host = r.place_on_field(0, "HOST-APP", Some(0)); // 4000 base DP
    let off = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);

    let dp_before = r.effective_dp(host).expect("host on field");
    do_link(&mut r, off);
    r.auto_resolve().ok();

    assert_eq!(
        r.effective_dp(host),
        Some(dp_before + 2000),
        "host effective DP must increase by +2000 while Offmon is linked (printed Link DP +2000)"
    );
}

#[test]
fn bt25_061_when_linked_locks_opponent_unsuspend() {
    let mut r = base().memory(5).start();
    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let off = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP-A", Some(0));
    advance_to_main(&mut r);

    assert!(!is_unsuspendable(&r, opp), "opp not yet locked");

    do_link(&mut r, off);
    // WhenLinked: choose the opponent Digimon to lock.
    r.auto_resolve().ok();

    assert!(
        is_unsuspendable(&r, opp),
        "WhenLinked applied CannotUnsuspend to the chosen opponent Digimon"
    );
}

#[test]
fn bt25_061_when_linked_no_opponent_digimon_is_noop() {
    // canNoSelect: false but 0 legal targets → the selection simply doesn't
    // fire; the link itself still completes.
    let mut r = base().memory(5).start();
    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let off = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);

    do_link(&mut r, off);
    r.auto_resolve().ok();

    assert_eq!(
        r.effective_dp(host),
        Some(4000 + 2000),
        "link completed (DP aura live) even with no opponent Digimon to lock"
    );
}
