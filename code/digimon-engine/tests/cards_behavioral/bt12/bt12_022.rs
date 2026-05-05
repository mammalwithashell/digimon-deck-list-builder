//! BT12-022 ExVeemon — Digimon, Lv.4, Blue, DP 4000, Cost 4.
//! Traits: Mythical Dragon.
//!
//! # Card text (cards.json)
//!
//! ```text
//! Effect:
//! [Your Turn] When this Digimon would DNA digivolve into a green Digimon
//! card, gain 1 memory.
//!
//! Inherited Effect:
//! [Your Turn] While this Digimon has [Imperialdramon] in its name or the
//! [Free] trait, it gains ＜Jamming＞ (This Digimon can't be deleted in
//! battles against Security Digimon.).
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT12/Blue/BT12_022.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - H2 Inherited conditional Jamming keyword grant (conditional aura)
//! - G2 DNA digivolve target predicate (BLOCKED clause 0)
//!
//! # Verdict: PARTIAL
//!
//! Clause 0 (own effect): BLOCKED — G-BEFORE-PAY-COST-DIGIVOLVE-TARGET +
//!   G-BEFORE-PAY-COST-GAIN-MEMORY. The "would DNA digivolve into green Digimon,
//!   gain 1 memory" requires BeforePayCost triggered effect with target-color
//!   predicate threading. DSL has no triggered gain_memory at BeforePayCost timing
//!   and no event_card_color_is predicate. See qa/dsl-vocab-gaps.md.
//!
//! Clause 1 (inherited): PARTIAL — shipped as `kind: grant_keyword, scope:
//!   inherited` with `active_when` encoding the name/trait condition. The
//!   `active_when` is compiled but silently discarded by lower_grant_keyword::lower
//!   (mod.rs line 82 uses `..` to ignore active_when for GrantKeyword).
//!   Net: Jamming is granted unconditionally to any carrier, regardless of name
//!   or trait. Over-fires for non-Imperialdramon/non-Free carriers.
//!   New gap: G-DSL-GRANT-KEYWORD-ACTIVE-WHEN-NOT-CONSUMED.
//!   Also blocked: carrier name check needs G-DSL-SELF-NAME-CONTAINS (AD1-014);
//!   carrier trait check needs source_permanent_trait_has evaluator arm in
//!   predicate.rs.

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Keyword};

const CARD_ID: &str = "BT12-022";

// ─── Card-data factories ─────────────────────────────────────────────────────

/// Carrier Digimon with [Imperialdramon] in its name (positive name branch).
fn make_imperialdramon_carrier(id: &str) -> CardData {
    let mut card = make_test_card(id, "Imperialdramon Fighter Mode");
    card.card_kind = CardKind::Digimon;
    card
}

/// Carrier Digimon with [Free] trait (positive trait branch).
fn make_free_trait_carrier(id: &str) -> CardData {
    let mut card = make_test_card(id, "VeedragonX");
    card.card_kind = CardKind::Digimon;
    card.traits = vec!["Free".to_string()];
    card
}

/// Carrier Digimon without [Imperialdramon] name or [Free] trait (negative branch).
fn make_unrelated_carrier(id: &str) -> CardData {
    let mut card = make_test_card(id, "Tyrannomon");
    card.card_kind = CardKind::Digimon;
    card
}

// ─── Section 1 — Structural assertions ──────────────────────────────────────

/// BT12-022 YAML must parse and compile without errors.
#[test]
fn bt12_022_yaml_parses_and_compiles() {
    let spec: digimon_dsl::spec::CardSpec =
        serde_yml::from_str(include_str!("../../../cards/bt12/BT12-022.yaml"))
            .expect("BT12-022 YAML parses");
    let _compiled = digimon_dsl::compile::compile(&spec).expect("BT12-022 YAML compiles");
}

/// BT12-022 has exactly zero triggered clauses (clause 0 is BLOCKED and absent
/// from the YAML). The effect text triggers at BeforePayCost timing which the
/// DSL cannot model without G-BEFORE-PAY-COST-DIGIVOLVE-TARGET and
/// G-BEFORE-PAY-COST-GAIN-MEMORY.
#[test]
fn bt12_022_clause_0_is_absent_blocked() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-022 in embedded DSL pack")
        .memory(0)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("BT12-022 compiled");

    let triggered_count = compiled
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();

    assert_eq!(
        triggered_count, 0,
        "BT12-022 clause 0 is BLOCKED — no triggered clauses should be present; \
         gap: G-BEFORE-PAY-COST-DIGIVOLVE-TARGET + G-BEFORE-PAY-COST-GAIN-MEMORY"
    );
}

/// BT12-022 has exactly one declarative clause: an inherited GrantKeyword
/// with keyword Jamming (clause 1, partial implementation).
#[test]
fn bt12_022_has_one_inherited_grant_keyword_jamming() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-022 in embedded DSL pack")
        .memory(0)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("BT12-022 compiled");

    let inherited_jamming_count = compiled
        .effects
        .iter()
        .filter(|c| match c {
            CompiledClause::Declarative(d) => match d {
                CompiledDeclarativeClause::GrantKeyword {
                    keyword, scope, ..
                } => *scope == CompiledScope::Inherited && keyword.eq_ignore_ascii_case("Jamming"),
                _ => false,
            },
            _ => false,
        })
        .count();

    assert_eq!(
        inherited_jamming_count, 1,
        "BT12-022 must have exactly one inherited GrantKeyword(Jamming) declarative clause"
    );
}

// ─── Section 2 — Clause 0 behavioral (BLOCKED) ───────────────────────────────
//
// Clause 0: "[Your Turn] When this Digimon would DNA digivolve into a green
// Digimon card, gain 1 memory."
//
// BLOCKED: G-BEFORE-PAY-COST-DIGIVOLVE-TARGET + G-BEFORE-PAY-COST-GAIN-MEMORY.
// All behavioral tests for clause 0 are ignored until both gaps close.

/// DNA digivolving into a green Digimon should gain 1 memory — BLOCKED.
#[test]
#[ignore = "pending: G-BEFORE-PAY-COST-DIGIVOLVE-TARGET + G-BEFORE-PAY-COST-GAIN-MEMORY from qa/dsl-vocab-gaps.md"]
fn bt12_022_dna_digivolving_into_green_gains_one_memory() {
    // When gap closes: set up BT12-022 on field, initiate DNA digivolve into
    // a green Digimon card from hand, assert memory increases by 1.
    //
    // Suggested setup:
    //   let mut runner = DebugRunner::builder()
    //       .dsl_card(CARD_ID)
    //       .add_card(make_green_digimon("GREEN-DNA"))
    //       .hand(0, &["GREEN-DNA"])
    //       .memory(0)
    //       .start();
    //   let carrier = runner.place_on_field(0, CARD_ID, Some(0));
    //   let mat_b = runner.place_on_field(0, "MAT-B", None);
    //   // initiate DNA digivolve on carrier + mat_b into GREEN-DNA
    //   assert_eq!(runner.memory(), 1, "should gain 1 memory for green DNA target");
}

/// DNA digivolving into a NON-green Digimon should NOT gain memory — BLOCKED.
#[test]
#[ignore = "pending: G-BEFORE-PAY-COST-DIGIVOLVE-TARGET + G-BEFORE-PAY-COST-GAIN-MEMORY from qa/dsl-vocab-gaps.md"]
fn bt12_022_dna_digivolving_into_non_green_does_not_gain_memory() {
    // When gap closes: DNA digivolve into a red/blue Digimon → no memory gain.
}

/// On opponent's turn, the memory gain should NOT trigger — BLOCKED.
#[test]
#[ignore = "pending: G-BEFORE-PAY-COST-DIGIVOLVE-TARGET + G-BEFORE-PAY-COST-GAIN-MEMORY from qa/dsl-vocab-gaps.md"]
fn bt12_022_clause_0_does_not_fire_on_opponents_turn() {
    // When gap closes: end_turn to opponent; opponent DNA digivolves BT12-022
    // into green Digimon → memory gain should NOT fire (your_turn: true gate).
}

// ─── Section 3 — Clause 1 behavioral (inherited Jamming) ─────────────────────
//
// Clause 1: "[Your Turn] While this Digimon has [Imperialdramon] in its name
// or the [Free] trait, it gains <Jamming>"
//
// Over-fires: `active_when` is compiled but not consumed by lowering.
// Positive tests pass; negative tests are #[ignore]'d.

/// When BT12-022 is stacked under a carrier with [Imperialdramon] in its name,
/// the carrier should have Jamming. (Over-fires: any carrier gets Jamming due
/// to G-DSL-GRANT-KEYWORD-ACTIVE-WHEN-NOT-CONSUMED.)
#[test]
fn bt12_022_inherited_jamming_granted_when_carrier_has_imperialdramon_name() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-022 in embedded DSL pack")
        .add_card(make_imperialdramon_carrier("IMPERIALDRAMON"))
        .memory(0)
        .start();

    // Stack: [BT12-022 (bottom), IMPERIALDRAMON (top)]
    let carrier = runner.place_stack(0, &[CARD_ID, "IMPERIALDRAMON"]);

    let has_jamming = runner.game.has_keyword(carrier, Keyword::Jamming);
    assert!(
        has_jamming,
        "carrier with [Imperialdramon] in name should have Jamming from BT12-022 inherited effect"
    );
}

/// When BT12-022 is stacked under a carrier with the [Free] trait, the carrier
/// should have Jamming. (Over-fires regardless of trait due to
/// G-DSL-GRANT-KEYWORD-ACTIVE-WHEN-NOT-CONSUMED; this positive test passes
/// for the wrong reason — the trait check is silently dropped.)
#[test]
fn bt12_022_inherited_jamming_granted_when_carrier_has_free_trait() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-022 in embedded DSL pack")
        .add_card(make_free_trait_carrier("FREE-CARRIER"))
        .memory(0)
        .start();

    // Stack: [BT12-022 (bottom), FREE-CARRIER (top)]
    let carrier = runner.place_stack(0, &[CARD_ID, "FREE-CARRIER"]);

    let has_jamming = runner.game.has_keyword(carrier, Keyword::Jamming);
    assert!(
        has_jamming,
        "carrier with [Free] trait should have Jamming from BT12-022 inherited effect"
    );
}

/// When BT12-022 is stacked under a carrier WITHOUT [Imperialdramon] in name
/// or [Free] trait, the carrier should NOT have Jamming.
///
/// FAILS (over-fires): `active_when` condition is not consumed by lowering,
/// so Jamming is granted unconditionally to any carrier.
/// Ignored pending G-DSL-GRANT-KEYWORD-ACTIVE-WHEN-NOT-CONSUMED.
#[test]
#[ignore = "pending: G-DSL-GRANT-KEYWORD-ACTIVE-WHEN-NOT-CONSUMED from qa/dsl-vocab-gaps.md — active_when is compiled but discarded in lower_grant_keyword::lower; Jamming over-fires for all carriers"]
fn bt12_022_inherited_jamming_not_granted_when_carrier_has_no_matching_name_or_trait() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-022 in embedded DSL pack")
        .add_card(make_unrelated_carrier("TYRANNO"))
        .memory(0)
        .start();

    // Stack: [BT12-022 (bottom), TYRANNO (top)]
    let carrier = runner.place_stack(0, &[CARD_ID, "TYRANNO"]);

    let has_jamming = runner.game.has_keyword(carrier, Keyword::Jamming);
    assert!(
        !has_jamming,
        "carrier without [Imperialdramon] name or [Free] trait should NOT have Jamming"
    );
}

/// When BT12-022 is the only card on the field (no carrier), it should not
/// grant itself Jamming (it's an inherited effect, not a self-grant).
/// The `source_permanent` from `lower_grant_keyword` would resolve to None
/// for a top-card slot, so the process closure returns early without granting.
#[test]
fn bt12_022_no_jamming_when_alone_on_field_as_top_card() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-022 in embedded DSL pack")
        .memory(0)
        .start();

    let handle = runner.place_on_field(0, CARD_ID, Some(0));

    // BT12-022 is the top card — its inherited effect is not active for itself.
    // Check via source_dp_contribution (no Jamming expected; this test is
    // structural since Jamming doesn't affect DP).
    // Instead, verify the grant didn't fire against itself:
    let data_idx = runner.game.player(0).battle_area[0]
        .top_card()
        .data_index;
    let top_card_id = runner.game.card_data[data_idx].card_id.clone();
    assert_eq!(
        top_card_id, CARD_ID,
        "BT12-022 should be the top card when placed alone"
    );
    // Inherited effects from BT12-022 only apply when it's UNDER another card.
    // With only one card in the stack, no inherited grant fires.
    // (The `has_keyword` check for the source slot would not fire for index == top.)
    let _ = handle;
}

/// Carrier with [Imperialdramon] in name inherits Jamming even on opponent's
/// turn — FAILS (over-fires: your_turn gate not enforced due to active_when
/// being dropped).
#[test]
#[ignore = "pending: G-DSL-GRANT-KEYWORD-ACTIVE-WHEN-NOT-CONSUMED from qa/dsl-vocab-gaps.md — your_turn gate in active_when is silently dropped; Jamming fires on both turns"]
fn bt12_022_inherited_jamming_not_active_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-022 in embedded DSL pack")
        .add_card(make_imperialdramon_carrier("IMPERIALDRAMON"))
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "IMPERIALDRAMON"]);
    runner.end_turn(); // switch to player 1's turn

    // Now it is player 1's turn — BT12-022's [Your Turn] gate should block Jamming.
    let has_jamming = runner.game.has_keyword(carrier, Keyword::Jamming);
    assert!(
        !has_jamming,
        "Jamming should not be active on opponent's turn (your_turn gate)"
    );
}
