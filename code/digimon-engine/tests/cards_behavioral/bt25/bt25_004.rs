//! BT25-004 Tapmon — DIGI-EGG, Lv.2, Green. Trait line "Appmon | Tool | Tap"
//! (Form: Appmon, Attribute: Tool, Type: Tap — official Bandai DB
//! data/card_bundles/BT25-004.md; the card face carries the DIGI-EGG banner).
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
//! - Optional ("you may") accept AND decline behavioral paths.
//! - [Once Per Turn] lockout (behavioral) + [Your Turn] owner-turn gate
//!   (structural — links to this Digimon only occur on the owner's turn).
//! - Digi-Egg identity metadata (kind/color/traits incl. the Tool attribute).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{
    encode_attack, EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START, PASS,
    REPLACEMENT_ACCEPT,
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

/// Base builder for the structural tests: Tapmon + a filler + an Appmon-trait
/// host Digimon (the stack Tapmon sits under as an inherited source).
fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-004 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        // Host: an Appmon-trait Digimon (legal link host for the sources).
        .add_card(make_digimon("HOST-APP", 4, 4000, 4, &["Appmon"]))
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
    // The card face carries the DIGI-EGG banner — Tapmon is a Digi-Egg,
    // not a playable Digimon (was mis-authored `kind: digimon`).
    assert_eq!(card.kind, CompiledCardKind::DigiEgg);
    assert_eq!(card.color, vec![CompiledColor::Green]);
    assert_eq!(card.dp, None, "Digi-Eggs print no DP");
    // Official trait line "Appmon | Tool | Tap" (form / attribute / type):
    // production folds all three into `traits`, so Tapmon IS a [Tool] card.
    assert_eq!(card.traits, vec!["Appmon", "Tool", "Tap"]);
    assert_eq!(card.attribute.as_deref(), Some("Tool"));
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
    // [Your Turn] owner-turn gate + the printed [Social]/[Tool]/[Game] trait
    // gate on the card about to link (DCGO: IsOwnerTurn + CardCondition).
    let gate = t
        .active_when
        .as_ref()
        .expect("reducer carries an active_when gate");
    assert_eq!(gate.your_turn, Some(true), "[Your Turn] gate");
    assert_eq!(
        gate.would_link_card_trait_any_of.as_deref(),
        Some(&["Social".to_string(), "Tool".to_string(), "Game".to_string()][..]),
        "trait gate is exactly [Social]/[Tool]/[Game]"
    );
}

// ─── Section 3 — Behavioral: trait gate negative (non-matching linking card) ──

// A neutral-trait linking source: [Beast]/[Appmon] — none of the printed
// [Social]/[Tool]/[Game] — with a self link-condition over Appmon hosts at
// cost 1. Synthetic on purpose: BT25-007 Gatchmon (used previously) officially
// prints Attribute "Social" and would in fact MATCH the gate in real play.
const NEUTRAL_LINK_SOURCE: &str = r#"
card: TEST-NEUTRAL-LINK
name: Test Neutral Link Source
kind: digimon
level: 3
color: [green]
cost: 3
dp: 2000
traits: [Beast, Appmon]
effects:
  - kind: link_condition
    cost: 1
    filter: { trait_has: Appmon }
"#;

#[test]
fn bt25_004_does_not_reduce_non_social_tool_game_link() {
    // Host = HOST-APP with Tapmon as an under-source (inherited reducer live).
    // Linking source = a [Beast]/[Appmon] card — none of Social/Tool/Game —
    // so no reduce prompt installs and its link cost (1) is NOT reduced.
    let mut r = base()
        .from_dsl_yaml(NEUTRAL_LINK_SOURCE)
        .expect("neutral link source compiles")
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .memory(5)
        .start();

    let host = r.place_stack(0, &[CARD_ID, "HOST-APP"]);
    let src = r.place_on_field(0, "TEST-NEUTRAL-LINK", Some(0));
    advance_to_main(&mut r);

    let mem_before = r.memory();
    r.game.decode_action(link_bit(src) as u16, 0);
    let _ = r.game.resolve_selection(0, encode_attack(0, host.index as u16));
    assert!(
        !r.pending_is_optional(),
        "a non-matching-trait link must not offer the reduce prompt"
    );
    r.auto_resolve().ok();

    assert_eq!(
        r.memory(),
        mem_before - 1,
        "a [Beast]/[Appmon] (non Social/Tool/Game) linking card pays full link cost 1 \
         — the reducer's trait gate excludes it"
    );
    // The link still attached (the source absorbed onto HOST-APP).
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
    // Resolve the host-selection prompt (pick the HOST-APP stack explicitly).
    let _ = r.game.resolve_selection(0, encode_attack(0, host.index as u16));
    // The reducer is an optional WhenWouldLink replacement — ACCEPT it
    // explicitly, so the in-flight cost drops by 1.
    assert!(
        r.pending_is_optional(),
        "the [Social] link must offer the optional 'you may reduce' prompt"
    );
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept the reduce");
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

#[test]
fn bt25_004_declining_reducer_pays_full_link_cost() {
    // Same setup as the accept test, but DECLINE the optional prompt —
    // "you may reduce" is a choice; declining pays the printed link cost 1.
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
    let _ = r.game.resolve_selection(0, encode_attack(0, host.index as u16));
    assert!(
        r.pending_is_optional(),
        "the [Social] link must offer the optional 'you may reduce' prompt"
    );
    r.game
        .resolve_selection(0, PASS)
        .expect("decline the reduce");
    r.auto_resolve().ok();

    assert_eq!(
        r.memory(),
        mem_before - 1,
        "declining the reducer pays the full link cost (1)"
    );
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1,
        "the link still resolves at full cost after declining"
    );
}

#[test]
fn bt25_004_reducer_is_once_per_turn() {
    // Two [Social] linking sources, one Tapmon-sourced host. The first
    // matching link offers + spends the [Once Per Turn] reduce; the second
    // link the SAME turn gets no prompt and pays the full cost 1.
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
    let src_a = r.place_on_field(0, "TEST-SOCIAL-LINK", Some(0));
    let _src_b = r.place_on_field(0, "TEST-SOCIAL-LINK", Some(0));
    advance_to_main(&mut r);

    // First link — accept the reduce (1 -> 0).
    let before_a = r.memory();
    r.game.decode_action(link_bit(src_a) as u16, 0);
    let _ = r.game.resolve_selection(0, encode_attack(0, host.index as u16));
    assert!(r.pending_is_optional(), "first link offers the reduce");
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept first reduce");
    r.auto_resolve().ok();
    assert_eq!(r.memory(), before_a, "first link pays 0 (1 - 1)");

    // Absorbing src_a shifted the battle-area Vec — re-resolve the second
    // source's current index (the host is index 0 and never shifts).
    let src_b_idx = r
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|p| {
            p.card_sources
                .last()
                .map(|c| c.card_id(&r.game.card_data) == "TEST-SOCIAL-LINK")
                .unwrap_or(false)
        })
        .expect("second [Social] source still standing");

    // Second link the SAME turn — the OPT is spent: no reduce prompt, full
    // cost 1 paid.
    let before_b = r.memory();
    let src_b_bit =
        FIELD_EFFECT_START + src_b_idx as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_LINK;
    r.game.decode_action(src_b_bit, 0);
    let _ = r.game.resolve_selection(0, encode_attack(0, host.index as u16));
    assert!(
        r.game.pending_selection.is_none(),
        "the second matching link the same turn must not offer the reduce ([Once Per Turn])"
    );
    r.auto_resolve().ok();
    assert_eq!(
        r.memory(),
        before_b - 1,
        "the second link pays the full cost (1)"
    );
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        2,
        "both sources linked onto the host"
    );
}
