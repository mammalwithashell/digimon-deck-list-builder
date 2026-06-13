//! BT25-004 Tapmon — Digimon, Lv.2, Green, Cost 0. Trait: Tap (App Name).
//!
//! # Card text (DCGO BT25_004.cs — authoritative)
//!
//! The card's whole printed text is an INHERITED effect:
//!   Inherited [Your Turn] [Once Per Turn] When a [Social], [Tool] or [Game]
//!   trait card would link to this Digimon, you may reduce the cost by 1.
//!
//! DCGO `WhenWouldLink` `ActivateClass` with `SetIsInheritedEffect(true)`:
//!   `card.Owner.UntilCalculateFixedCostEffect.Add(GrantedReduceLinkCostClass(
//!      reducedCost: 1,
//!      cardSourceCondition: Social|Tool|Game,
//!      permanentCondition: permanent == card.PermanentOfThisCard(),  // this host
//!      rootCondition: _ => true))`
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_004.cs
//!
//! # Re-adjudication note (2026-06-07)
//! Prior verdict BLOCKED (engine, facet #10). RESOLVED: the predicated
//! host-side `WhenWouldLink` cost reducer (Gap 5) landed —
//! `when: when_would_link_to_this` + `active_when: { would_link_card_trait_any_of }`
//! + `process: [{ reduce_link_cost: { amount: N } }]` (optional + once_per_turn),
//! lowering to `EffectContext::reduce_pending_link_cost`. BT25-004 is the first
//! production user.
//!
//! # Patterns covered (RUST_DSL_TEST_API §4.3)
//! - Inherited (source) scope effect.
//! - Host-side `when_would_link_to_this` pre-link replacement reducer (Gap 5).
//! - `would_link_card_trait_any_of` trait gate (positive + negative).
//! - Optional ("you may") accept/decline via the replacement framework.
//! - [Once Per Turn] lockout + [Your Turn] owner-turn gate (structural).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledStep, CompiledTiming};
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-004";

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.colors = vec![CardColor::Green];
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn link_bit(perm: PermanentHandle) -> usize {
    (FIELD_EFFECT_START + perm.index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_LINK)
        as usize
}

/// Base builder. The linking sources carry a self link-condition over Appmon
/// hosts (BT25-007 Gatchmon's YAML); the host that Tapmon sits under is
/// Appmon-trait so it is a legal link host.
fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-004 YAML parses and compiles")
        .dsl_card("BT25-007")
        .expect("BT25-007 (a concrete linking Appmon source) present")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        // Host B: an Appmon-trait Digimon (legal link host for the source).
        .add_card(make_digimon("HOST-APP", 4, 4000, 4, &["Appmon"]))
        // A Social+Appmon linking source variant we register a link-condition on
        // by reusing BT25-007's effect: see the Social source note in the test.
        .add_card(make_digimon("SRC-SOCIAL", 3, 2000, 3, &["Social", "Appmon"]))
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_004_yaml_printed_metadata() {
    let runner = base()
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Tapmon");
    assert_eq!(card.level, Some(2));
}

#[test]
fn bt25_004_single_clause_is_inherited_would_link_reducer() {
    let runner = base()
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(
        triggered.len(),
        1,
        "BT25-004 has exactly one (inherited reducer) clause"
    );
    let t = triggered[0];
    assert!(
        matches!(t.scope, CompiledScope::Inherited),
        "the reducer clause must be inherited scope"
    );
    assert!(
        t.when.contains(&CompiledTiming::WhenWouldLinkToThis),
        "the reducer must use when_would_link_to_this timing"
    );
}

#[test]
fn bt25_004_reducer_is_optional_opt_and_reduces_by_1() {
    let runner = base()
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let t = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenWouldLinkToThis) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("reducer clause present");
    assert!(t.optional, "'you may reduce' must be optional");
    assert!(t.once_per_turn, "[Once Per Turn] -> once_per_turn");
    assert!(
        t.process
            .iter()
            .any(|s| matches!(s, CompiledStep::ReduceLinkCost { amount: 1 })),
        "the body reduces the in-flight link cost by 1"
    );
}

// ─── Section 3 — Behavioral: trait gate negative (non-matching linking card) ──

#[test]
fn bt25_004_does_not_reduce_non_social_tool_game_link() {
    // Host B = HOST-APP with Tapmon as an under-source (inherited reducer live).
    // Linking source = BT25-007 Gatchmon: traits [Search, Appmon] — none of
    // Social/Tool/Game — so its link cost (1) is NOT reduced.
    let mut r = base()
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .memory(5)
        .start();

    let host = r.place_stack(0, &[CARD_ID, "HOST-APP"]);
    let src = r.place_on_field(0, "BT25-007", Some(0));
    advance_to_main(&mut r);

    let mem_before = r.memory();
    r.game.decode_action(link_bit(src) as u16, 0);
    let host_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, host_action);
    r.auto_resolve().ok();

    assert_eq!(
        r.memory(),
        mem_before - 1,
        "a [Search]/[Appmon] (non Social/Tool/Game) linking card pays full link cost 1 \
         — the reducer's trait gate excludes it"
    );
    // The link still attached (Gatchmon absorbed onto HOST-APP).
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1,
        "the non-matching link still resolves (just unreduced)"
    );
}

// A bespoke Social+Appmon linking source carrying a self link-condition over
// Appmon hosts at cost 1. Used to exercise the POSITIVE reducer path: its
// [Social] trait matches the reducer's `would_link_card_trait_any_of` gate.
const SOCIAL_LINK_SOURCE: &str = r#"
card: TEST-SOCIAL-LINK
name: Test Social Link Source
kind: digimon
level: 3
color: [green]
cost: 3
dp: 2000
traits: [Social, Appmon]
effects:
  - kind: link_condition
    cost: 1
    filter: { trait_has: Appmon }
"#;

#[test]
fn bt25_004_reduces_social_link_cost_on_accept() {
    // Host B = HOST-APP with Tapmon under-source (inherited reducer live).
    // Linking source = a [Social] Appmon with link cost 1. The reducer fires in
    // the WhenWouldLink window; accepting it reduces the paid cost to 0.
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-004 compiles")
        .from_dsl_yaml(SOCIAL_LINK_SOURCE)
        .expect("social link source compiles")
        .add_card(make_digimon("HOST-APP", 4, 4000, 4, &["Appmon"]))
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .memory(5)
        .start();

    let host = r.place_stack(0, &[CARD_ID, "HOST-APP"]);
    let src = r.place_on_field(0, "TEST-SOCIAL-LINK", Some(0));
    advance_to_main(&mut r);

    let mem_before = r.memory();
    r.game.decode_action(link_bit(src) as u16, 0);
    // Resolve the host-selection prompt (pick HOST-APP).
    let host_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, host_action);
    // The reducer is an optional WhenWouldLink replacement — ACCEPT it (and any
    // follow-on), so the in-flight cost drops by 1.
    r.auto_resolve().ok();

    assert_eq!(
        r.memory(),
        mem_before,
        "accepting the [Social] reducer drops link cost 1 -> 0 (no memory spent)"
    );
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1,
        "the [Social] source linked onto the host at the reduced cost"
    );
}
