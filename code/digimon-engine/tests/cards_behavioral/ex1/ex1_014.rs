//! EX1-014 ExVeemon — Digimon, Lv.4, Blue, DP 4000, Cost 5.
//! Traits: Mythical Dragon.
//! Evo: Lv.3 Blue / Cost 2.
//!
//! # Card text (cards.json)
//!
//! ```text
//! ＜Jamming＞ (This Digimon can't be deleted in battles against Security Digimon.)
//!
//! Inherited Effect:
//! [Your Turn] While this Digimon has [Imperialdramon] in its name or [Free] trait,
//! it gains ＜Jamming＞.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX1/Blue/EX1_014.cs
//!
//! # Verdict: PARTIAL
//!
//! ## Clause 0 — face-up ＜Jamming＞
//! Implemented via `kind: grant_keyword / keyword: Jamming` (unconditional,
//! face-up). Resolves as G-DECLARATIVE-KEYWORD was RESOLVED 2026-05-02.
//!
//! ## Clause 1 — [Inherited][Your Turn] conditional ＜Jamming＞
//! Name arm (carrier stack contains "Imperialdramon"): IMPLEMENTED, with known
//! over-fire (G-DSL-AURA-TARGET-SOURCE-PERMANENT) — the aura targets all
//! controller Digimon, not only the carrier.
//! [Free] trait arm: BLOCKED (G-DSL-SELF-DIGIVOLUTION-CONTAINS-TRAIT).
//!
//! # Patterns this test covers
//! - H9 — face-up `＜Jamming＞` via declarative `grant_keyword`.
//! - D4 — inherited conditional keyword aura (`kind: aura`, `scope: inherited`)
//!   gated on `self_digivolution_contains_name: "Imperialdramon"` and
//!   `your_turn: true`.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;

// ═══════════════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════════════

/// A minimal Lv.5 Digimon carrier named "Imperialdramon".
fn imperialdramon_carrier() -> digimon_engine::card_data::CardData {
    let mut c = make_test_card("IMPERIALDRAMON", "Imperialdramon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(9000);
    c
}

/// A minimal Lv.5 Digimon carrier with an unrelated name (no "Imperialdramon").
fn unrelated_carrier() -> digimon_engine::card_data::CardData {
    let mut c = make_test_card("UNRELATED", "Paildramon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(7000);
    c
}

/// Base runner with EX1-014 registered.
fn ex1_014_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX1-014")
        .expect("EX1-014 found in embedded DSL pack")
        .memory(0)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// EX1-014 must parse and compile from the embedded DSL pack without errors.
#[test]
fn ex1_014_compiles_from_embedded_pack() {
    let runner = ex1_014_runner();
    assert!(
        runner.compiled_card("EX1-014").is_some(),
        "EX1-014 must be present and compiled in the embedded DSL pack"
    );
}

/// EX1-014 card metadata: Lv.4, DP 4000, cost 5.
#[test]
fn ex1_014_metadata_level_dp_cost() {
    let runner = ex1_014_runner();
    let card = runner.compiled_card("EX1-014").expect("EX1-014 compiles");
    assert_eq!(card.level, Some(4), "ExVeemon is Lv.4");
    assert_eq!(card.dp, Some(4000), "ExVeemon is DP 4000");
    assert_eq!(card.cost, Some(5), "ExVeemon costs 5 to play");
}

/// EX1-014 must declare at least one face-up `GrantKeyword` declarative clause
/// for Jamming (Clause 0 — printed face-up keyword).
#[test]
fn ex1_014_has_face_up_grant_keyword_jamming() {
    let runner = ex1_014_runner();
    let card = runner.compiled_card("EX1-014").expect("EX1-014 compiles");

    let has_face_up_jamming = card.effects.iter().any(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            scope,
            ..
        }) => *scope == CompiledScope::FaceUp && keyword.eq_ignore_ascii_case("Jamming"),
        _ => false,
    });

    assert!(
        has_face_up_jamming,
        "EX1-014 must declare a face-up GrantKeyword(Jamming) declarative clause"
    );
}

/// EX1-014 must declare exactly one inherited Aura declarative clause
/// (Clause 1 — inherited conditional Jamming).
#[test]
fn ex1_014_has_inherited_aura_clause() {
    let runner = ex1_014_runner();
    let card = runner.compiled_card("EX1-014").expect("EX1-014 compiles");

    let inherited_aura_count = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                    scope: CompiledScope::Inherited,
                    ..
                })
            )
        })
        .count();

    assert_eq!(
        inherited_aura_count, 1,
        "EX1-014 must declare exactly one inherited Aura clause (Clause 1)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: face-up ＜Jamming＞ (Clause 0)
// ═══════════════════════════════════════════════════════════════════════════════

/// ExVeemon placed face-up on the field must have ＜Jamming＞ active.
/// G-DECLARATIVE-KEYWORD RESOLVED 2026-05-02 — declarative keyword grants
/// are installed at runtime by the declarative-effect tick.
#[test]
fn ex1_014_face_up_jamming_installs_on_field() {
    let mut runner = ex1_014_runner();
    let ex_veemon = runner.place_on_field(0, "EX1-014", Some(0));

    // Run a declarative-effect tick so the face-up grant materializes.
    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(ex_veemon, Keyword::Jamming),
        "Face-up ExVeemon must have <Jamming> installed after tick_declarative_effects \
         (G-DECLARATIVE-KEYWORD resolved 2026-05-02)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: inherited aura — name arm (Clause 1)
// ═══════════════════════════════════════════════════════════════════════════════
//
// These tests verify the name arm of Clause 1: while EX1-014 is in the
// digivolution stack of a carrier whose stack contains "Imperialdramon" in
// its name, the carrier gains ＜Jamming＞ on the controller's turn.
//
// KNOWN OVER-FIRE (G-DSL-AURA-TARGET-SOURCE-PERMANENT): the current `kind:
// aura` lowering applies the keyword to ALL controller battle_area Digimon
// matching `target: { owner: you }`, not only the carrier. The tests below
// reflect this known divergence; see the YAML header for the gap detail.
//
// BLOCKED (G-DSL-SELF-DIGIVOLUTION-CONTAINS-TRAIT): the [Free] trait arm is
// omitted from the YAML and therefore not tested. See the YAML header.

/// Positive path: EX1-014 stacked under "Imperialdramon" carrier on your turn.
/// The carrier must gain ＜Jamming＞ after a declarative-effect tick.
///
/// Stack layout: [EX1-014 (source 0)] → [IMPERIALDRAMON (top/carrier)]
/// `self_digivolution_contains_name: "Imperialdramon"` scans the carrier's
/// digivolution stack — finds "Imperialdramon" at the top — condition passes.
/// `your_turn: true` — game starts at player 0's turn (turn_player_idx = 0).
#[test]
fn ex1_014_inherited_jamming_active_when_carrier_is_imperialdramon_on_your_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX1-014")
        .expect("EX1-014 in embedded pack")
        .add_card(imperialdramon_carrier())
        .memory(0)
        .start();

    // EX1-014 is the base (source 0), IMPERIALDRAMON is the top card (carrier).
    let carrier = runner.place_stack(0, &["EX1-014", "IMPERIALDRAMON"]);

    // Ensure we are on player 0's turn.
    runner.game.turn_player_idx = 0;
    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(carrier, Keyword::Jamming),
        "Carrier 'Imperialdramon' must gain <Jamming> from the inherited EX1-014 aura \
         when it is the controller's turn and the stack contains 'Imperialdramon' in name"
    );
}

/// Negative path — wrong name: carrier is "Paildramon" (no "Imperialdramon"
/// substring). The `self_digivolution_contains_name` check must fail, so the
/// carrier does NOT gain ＜Jamming＞ from the inherited aura.
#[test]
fn ex1_014_inherited_jamming_inactive_when_carrier_name_unrelated() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX1-014")
        .expect("EX1-014 in embedded pack")
        .add_card(unrelated_carrier())
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &["EX1-014", "UNRELATED"]);

    runner.game.turn_player_idx = 0;
    runner.game.tick_declarative_effects();

    assert!(
        !runner.game.has_keyword(carrier, Keyword::Jamming),
        "Carrier 'Paildramon' must NOT gain <Jamming> from EX1-014's inherited aura \
         when the carrier's stack contains no 'Imperialdramon' substring"
    );
}

/// Negative path — opponent's turn: even when the carrier is "Imperialdramon",
/// the inherited aura is gated by `your_turn: true`. On the opponent's turn
/// (turn_player_idx = 1), the condition must fail.
#[test]
fn ex1_014_inherited_jamming_inactive_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX1-014")
        .expect("EX1-014 in embedded pack")
        .add_card(imperialdramon_carrier())
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &["EX1-014", "IMPERIALDRAMON"]);

    // Switch to opponent's turn — turn_player() will now return 1,
    // which != rctx.player (0), so `your_turn: true` evaluates false.
    runner.game.turn_player_idx = 1;
    runner.game.tick_declarative_effects();

    assert!(
        !runner.game.has_keyword(carrier, Keyword::Jamming),
        "Carrier 'Imperialdramon' must NOT gain <Jamming> from EX1-014's inherited aura \
         on the opponent's turn (your_turn: true gate must block it)"
    );
}

/// Aura restores on return to your turn after opponent's turn.
/// Runs two ticks: one on opponent's turn (should be inactive), one after
/// switching back to your turn (should be active again).
#[test]
fn ex1_014_inherited_jamming_reactivates_when_turn_switches_back() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX1-014")
        .expect("EX1-014 in embedded pack")
        .add_card(imperialdramon_carrier())
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &["EX1-014", "IMPERIALDRAMON"]);

    // 1st tick — opponent's turn.
    runner.game.turn_player_idx = 1;
    runner.game.tick_declarative_effects();
    assert!(
        !runner.game.has_keyword(carrier, Keyword::Jamming),
        "Should be inactive on opponent's turn"
    );

    // 2nd tick — your turn again.
    runner.game.turn_player_idx = 0;
    runner.game.tick_declarative_effects();
    assert!(
        runner.game.has_keyword(carrier, Keyword::Jamming),
        "Should be active again on your turn"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — BLOCKED: [Free] trait arm (Clause 1 trait branch)
// ═══════════════════════════════════════════════════════════════════════════════
//
// DCGO: `card.PermanentOfThisCard().TopCard.CardTraits.Contains("Free")`
// DSL gap: G-DSL-SELF-DIGIVOLUTION-CONTAINS-TRAIT — no BoolPredicate that
// evaluates whether the carrier permanent's stack contains a card with trait X.
// `source_permanent_trait_has` exists in CompiledPredicate spec but is not
// evaluated in `eval_predicate_with_bindings` at runtime.
//
// Tests below are `#[ignore]`'d with the gap tag. They document the intended
// behavioral coverage once the gap closes.

/// When EX1-014 is stacked under a carrier with the [Free] trait (but no
/// "Imperialdramon" in its name), the inherited aura should grant ＜Jamming＞.
/// BLOCKED: G-DSL-SELF-DIGIVOLUTION-CONTAINS-TRAIT.
#[test]
#[ignore = "BLOCKED: G-DSL-SELF-DIGIVOLUTION-CONTAINS-TRAIT — no DSL predicate to check carrier's stack for a trait; [Free] trait arm omitted from YAML pending gap closure"]
fn ex1_014_inherited_jamming_active_when_carrier_has_free_trait() {
    let mut free_carrier = make_test_card("FREE-CARRIER", "FreeCarrier");
    free_carrier.card_kind = CardKind::Digimon;
    free_carrier.level = Some(5);
    free_carrier.dp = Some(7000);
    free_carrier.traits.push("Free".to_string());

    let mut runner = DebugRunner::builder()
        .dsl_card("EX1-014")
        .expect("EX1-014 in embedded pack")
        .add_card(free_carrier)
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &["EX1-014", "FREE-CARRIER"]);

    runner.game.turn_player_idx = 0;
    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(carrier, Keyword::Jamming),
        "Carrier with [Free] trait must gain <Jamming> from EX1-014 inherited aura \
         (BLOCKED pending G-DSL-SELF-DIGIVOLUTION-CONTAINS-TRAIT)"
    );
}
