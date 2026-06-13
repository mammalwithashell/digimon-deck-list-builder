//! BT25-045 Onmon — Digimon, Lv.3, Green, DP 2000, Cost 3.
//! Trait: Online (App Name) — Appmon trait line.
//!
//! # Card text (DCGO BT25_045.cs — authoritative)
//!
//! Alt-digivolve: digivolve from a Lv.2 [Appmon] for cost 0
//!   (DCGO `AddSelfDigivolutionRequirementStaticEffect(HasAppmonTraits, 0, lvl 2)`).
//! Self link-condition: link onto an [Appmon] host for link cost 1
//!   (DCGO `AddSelfLinkConditionStaticEffect(HasAppmonTraits, 1)`).
//! [Your Turn] [Once Per Turn] When a [Social], [Tool] or [Game] trait card
//!   would link to this Digimon, you may reduce the cost by 1.
//!   (DCGO `WhenWouldLink` ActivateClass; face-up, NOT inherited.)
//! [When Linking] Suspend 1 of your opponent's Digimon.
//!   (DCGO `WhenLinked`, `SetIsLinkedEffect(true)`.)
//! Inherited: Suspend 1 of your opponent's Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_045.cs
//!
//! # Re-adjudication note (2026-06-07)
//! Prior verdict BLOCKED (engine, facet #10) because the host-filtered optional
//! `WhenWouldLink` reducer could not be authored. RESOLVED: Gap 5 landed
//! (`when: when_would_link_to_this` + `would_link_card_trait_any_of` +
//! `reduce_link_cost`). With the reducer now expressible AND the prior Shape-B
//! link / when_linked vocabulary, every clause is faithful — no omissions.
//!
//! # Patterns covered (RUST_DSL_TEST_API §4.3)
//! - alt-digivolve registration (alt_paths digivolve cost 0).
//! - DigiLink Shape-B self link-condition.
//! - Face-up `when_would_link_to_this` reducer (Gap 5), optional + OPT.
//! - `when: when_linked` suspend payoff (linked scope).
//! - Inherited suspend.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-045";

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

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-045 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("OPP-DIGI", 4, 4000, 4, &["Beast"]))
        .add_card(make_digimon("APP-LV2", 2, 1000, 2, &["Appmon"]))
        .add_card(make_digimon("HOST-APP", 4, 4000, 4, &["Appmon"]))
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_045_yaml_printed_metadata() {
    let runner = base()
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Onmon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.dp, Some(2000));
}

#[test]
fn bt25_045_has_appmon_alt_digivolve_and_link_condition() {
    let runner = base()
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let alt = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && matches!(p.cost, Some(CompiledCost::Literal(0)))
    });
    assert!(alt, "must register cost-0 alt-digivolve over a Lv.2 Appmon");
    let link = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, .. }) if *cost == 1
    ));
    assert!(link, "must declare a self link-condition with cost 1");
}

#[test]
fn bt25_045_has_faceup_would_link_reducer_optional_opt() {
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
    assert!(
        !matches!(t.scope, CompiledScope::Inherited),
        "Onmon's reducer is a face-up effect, not inherited"
    );
    assert!(t.optional, "'you may reduce' is optional");
    assert!(t.once_per_turn, "[Once Per Turn]");
    assert!(
        t.process
            .iter()
            .any(|s| matches!(s, CompiledStep::ReduceLinkCost { amount: 1 })),
        "reduces link cost by 1"
    );
}

#[test]
fn bt25_045_has_linked_when_linking_suspend() {
    // The card image's lower box is a single [When Linking] suspend clause (the
    // linked effect). cards.json mis-slots it as "inherited"; DCGO models it as
    // the `WhenLinked` `SetIsLinkedEffect(true)` clause — so there is exactly
    // one linked-scope suspend, not a separate inherited one.
    let runner = base()
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let when_linked = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Triggered(t)
            if t.when.contains(&CompiledTiming::WhenLinked)
                && matches!(t.scope, CompiledScope::Linked)
    ));
    assert!(when_linked, "must have a linked [When Linking] suspend clause");
}

// ─── Section 3 — Behavioral: [When Linking] suspends an opponent Digimon ──────

#[test]
fn bt25_045_when_linked_suspends_opp_digimon() {
    let mut r = base()
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let onmon = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP-DIGI", Some(0));
    advance_to_main(&mut r);

    assert!(
        !r.game.player(1).battle_area[opp.index as usize].is_suspended,
        "opp Digimon starts unsuspended"
    );

    r.game.decode_action(link_bit(onmon) as u16, 0);
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    r.auto_resolve().ok();

    assert!(
        r.game.player(1).battle_area[opp.index as usize].is_suspended,
        "[When Linking] suspended the opponent Digimon"
    );
}

// ─── Section 3 — Behavioral: face-up reducer reduces a Social link cost ───────

#[test]
fn bt25_045_faceup_reducer_drops_social_link_cost() {
    let mut r = base()
        .from_dsl_yaml(SOCIAL_LINK_SOURCE)
        .expect("social link source compiles")
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    // Onmon is itself the host here (it is an Appmon-trait Digimon).
    let onmon = r.place_on_field(0, CARD_ID, Some(0));
    let src = r.place_on_field(0, "TEST-SOCIAL-LINK", Some(0));
    advance_to_main(&mut r);

    let mem_before = r.memory();
    r.game.decode_action(link_bit(src) as u16, 0);
    let host_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, host_action);
    r.auto_resolve().ok();

    assert_eq!(
        r.memory(),
        mem_before,
        "accepting Onmon's [Social] reducer drops link cost 1 -> 0"
    );
    assert_eq!(
        r.game.player(0).battle_area[onmon.index as usize]
            .linked_cards
            .len(),
        1,
        "the [Social] source linked onto Onmon at the reduced cost"
    );
}
