//! BT25-007 Gatchmon — Digimon, Lv.3, Red, DP 2000, Cost 3.
//! Traits: Search (App Name) — Appmon trait line.
//!
//! # Card text (DCGO BT25_007.cs — authoritative; cards.json mis-slots the
//! WhenLinked clause as "inherited")
//!
//! Self link-condition: this card may be linked onto an [Appmon]-trait Digimon
//!   for link cost 1 (DCGO `AddSelfLinkConditionStaticEffect(HasAppmonTraits, 1)`).
//! Alt-digivolve: may digivolve from a level-2 [Appmon] for cost 0
//!   (DCGO `AddSelfDigivolutionRequirementStaticEffect(HasAppmonTraits, 0, lvl 2)`).
//! [On Play] Reveal the top 3 cards of your deck. Add 1 [Appmon] trait card and
//!   1 [Social], [Tool], [Reboot] or [Creation] trait card among them to the
//!   hand. Return the rest to the bottom of the deck.
//! [When Linking] Delete 1 of your opponent's Digimon with 3000 DP or less.
//!   (DCGO WhenLinked, SetIsLinkedEffect(true).)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Red/BT25_007.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - DigiLink Shape-B self link-condition (G-DSL-DIGILINK)
//! - when: when_linked triggered effect (linked scope)
//! - reveal-N two-bucket add-to-hand (select_reveal_buckets)
//! - alt-digivolve registration (alt_paths digivolve)
//! - delete by DP<=N

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
use digimon_engine::enums::{CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-007";

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

/// Card added to deck/hand to drive the reveal buckets.
fn appmon_card(id: &str) -> CardData {
    make_digimon(id, 3, 2000, 3, &["Search", "Appmon"])
}

fn link_bit(perm: PermanentHandle) -> usize {
    (FIELD_EFFECT_START + perm.index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_LINK)
        as usize
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-007 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        // Reveal pool: one Appmon, one Social, plus filler.
        .add_card(make_digimon("APPMON-X", 3, 2000, 3, &["Appmon"]))
        .add_card(make_digimon("SOCIAL-X", 3, 2000, 3, &["Social"]))
        // Host Appmon for the on-field link absorb path.
        .add_card(make_digimon("HOST-APP", 4, 4000, 4, &["Appmon"]))
        // Opponent Digimon: one <=3000, one >3000.
        .add_card(make_digimon("OPP-SMALL", 3, 3000, 3, &["Beast"]))
        .add_card(make_digimon("OPP-BIG", 4, 5000, 4, &["Beast"]))
        .deck(1, &["DECK-PAD"; 12])
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_007_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Gatchmon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.dp, Some(2000));
}

#[test]
fn bt25_007_has_link_condition_appmon_cost_1() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, .. }) if *cost == 1
    ));
    assert!(has, "BT25-007 must declare a self link-condition with cost 1");
}

#[test]
fn bt25_007_registers_appmon_alt_digivolve() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && matches!(p.cost, Some(CompiledCost::Literal(0)))
    });
    assert!(has, "BT25-007 must register a cost-0 alt-digivolve over Appmon");
}

#[test]
fn bt25_007_has_on_play_and_when_linked_clauses() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let on_play = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay)
    ));
    let when_linked = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenLinked)
    ));
    assert!(on_play, "BT25-007 must have an [On Play] clause");
    assert!(when_linked, "BT25-007 must have a [When Linking] clause");
}

// ─── Section 2 — On Play reveal-3 two-bucket add ─────────────────────────────

#[test]
fn bt25_007_on_play_reveals_and_adds_one_of_each_bucket() {
    // Top 3 of deck = [APPMON-X, SOCIAL-X, DECK-PAD] (last element = top).
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD", "SOCIAL-X", "APPMON-X"])
        .memory(10)
        .start();
    let hand_before = r.hand_size(0);

    // Play Gatchmon from hand index 0 → On Play fires.
    let _g = r.play(0, 0).expect("Gatchmon played");
    // Resolve the two mandatory bucket picks (Appmon then Social) — each is a
    // real selection surfaced through pending_selection (no auto-pick in YAML).
    r.auto_resolve().ok();

    // Net hand change: -1 (played Gatchmon) +2 (added two revealed cards) = +1.
    assert_eq!(
        r.hand_size(0),
        hand_before + 1,
        "On Play added exactly 2 revealed cards (one per bucket) net of the play"
    );
    // The non-selected revealed card went to deck bottom, not trash.
    assert_eq!(r.trash_size(0), 0, "remainder bottomed, not trashed");
}

// ─── Section 3 — When Linking: delete opp Digimon DP<=3000 ───────────────────

#[test]
fn bt25_007_when_linked_deletes_small_opp_digimon() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let gatch = r.place_on_field(0, CARD_ID, Some(0));
    let opp_small = r.place_on_field(1, "OPP-SMALL", Some(0));
    advance_to_main(&mut r);

    let opp_before = r.battle_area_size(1);

    // Activate the on-field Link ability and pick the host → absorb → WhenLinked.
    r.game.decode_action(link_bit(gatch) as u16, 0);
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    // WhenLinked delete prompt: resolve it (single eligible target).
    r.auto_resolve().ok();

    assert_eq!(
        r.battle_area_size(1),
        opp_before - 1,
        "WhenLinked deleted the <=3000 DP opponent Digimon"
    );
    assert_eq!(r.trash_size(1), 1, "deleted Digimon went to opponent trash");
}

#[test]
fn bt25_007_when_linked_cannot_delete_big_opp_digimon() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let gatch = r.place_on_field(0, CARD_ID, Some(0));
    let opp_big = r.place_on_field(1, "OPP-BIG", Some(0)); // 5000 DP — ineligible
    advance_to_main(&mut r);

    let opp_before = r.battle_area_size(1);

    r.game.decode_action(link_bit(gatch) as u16, 0);
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    r.auto_resolve().ok();

    assert_eq!(
        r.battle_area_size(1),
        opp_before,
        "5000-DP opponent Digimon is not an eligible delete target"
    );
}
