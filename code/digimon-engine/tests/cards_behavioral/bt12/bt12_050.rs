//! BT12-050 Stingmon — Digimon, Lv.4, Green, DP 4000, Cost 4.
//! Traits: Insectoid.
//!
//! # Card text (cards.json)
//!
//! ```text
//! Effect:
//! [Your Turn] When this Digimon would DNA digivolve into a blue Digimon
//! card, gain 1 memory.
//!
//! Inherited Effect:
//! [Your Turn] While this Digimon has [Imperialdramon] in its name or the
//! [Free] trait, it gains ＜Piercing＞ (When this Digimon attacks and deletes
//! an opponent's Digimon and survives the battle, it performs any security
//! checks it normally would.).
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT12/Green/BT12_050.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - H2 Inherited conditional Piercing keyword grant (conditional aura)
//! - G2 DNA digivolve target predicate (BLOCKED clause 0)
//!
//! # Verdict: PARTIAL
//!
//! Clause 0 (own effect): BLOCKED — G-BEFORE-PAY-COST-DIGIVOLVE-TARGET +
//!   G-BEFORE-PAY-COST-GAIN-MEMORY. The "would DNA digivolve into blue Digimon,
//!   gain 1 memory" requires BeforePayCost triggered effect with target-color
//!   predicate threading. DSL has no triggered gain_memory at BeforePayCost timing
//!   and no event_card_color_is predicate. See qa/dsl-vocab-gaps.md.
//!   Structurally identical to BT12-022 clause 0 (green→blue swap only).
//!
//! Clause 1 (inherited): PARTIAL — shipped as `kind: grant_keyword, scope:
//!   inherited` with `active_when` encoding the name/trait condition. The
//!   `active_when` is compiled but silently discarded by lower_grant_keyword::lower
//!   (mod.rs uses `..` to ignore active_when for GrantKeyword).
//!   Net: Piercing is granted unconditionally to any carrier, regardless of name
//!   or trait. Over-fires for non-Imperialdramon/non-Free carriers.
//!   Gap: G-DSL-GRANT-KEYWORD-ACTIVE-WHEN-NOT-CONSUMED.
//!   Structurally identical to BT12-022 clause 1 (Jamming→Piercing swap only).

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Keyword};

const CARD_ID: &str = "BT12-050";

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

/// BT12-050 YAML must parse and compile without errors.
#[test]
fn bt12_050_yaml_parses_and_compiles() {
    let spec: digimon_dsl::spec::CardSpec =
        serde_yml::from_str(include_str!("../../../cards/bt12/BT12-050.yaml"))
            .expect("BT12-050 YAML parses");
    let _compiled = digimon_dsl::compile::compile(&spec).expect("BT12-050 YAML compiles");
}

/// BT12-050 has exactly zero triggered clauses (clause 0 is BLOCKED and absent
/// from the YAML). The effect text triggers at BeforePayCost timing which the
/// DSL cannot model without G-BEFORE-PAY-COST-DIGIVOLVE-TARGET and
/// G-BEFORE-PAY-COST-GAIN-MEMORY.
#[test]
fn bt12_050_clause_0_is_absent_blocked() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-050 in embedded DSL pack")
        .memory(0)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("BT12-050 compiled");

    let triggered_count = compiled
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();

    assert_eq!(
        triggered_count, 0,
        "BT12-050 clause 0 is BLOCKED — no triggered clauses should be present; \
         gap: G-BEFORE-PAY-COST-DIGIVOLVE-TARGET + G-BEFORE-PAY-COST-GAIN-MEMORY"
    );
}

/// BT12-050 has exactly one declarative clause: an inherited GrantKeyword
/// with keyword Piercing (clause 1, partial implementation).
#[test]
fn bt12_050_has_one_inherited_grant_keyword_piercing() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-050 in embedded DSL pack")
        .memory(0)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("BT12-050 compiled");

    let inherited_piercing_count = compiled
        .effects
        .iter()
        .filter(|c| match c {
            CompiledClause::Declarative(d) => match d {
                CompiledDeclarativeClause::GrantKeyword { keyword, scope, .. } => {
                    *scope == CompiledScope::Inherited && keyword.eq_ignore_ascii_case("Piercing")
                }
                _ => false,
            },
            _ => false,
        })
        .count();

    assert_eq!(
        inherited_piercing_count, 1,
        "BT12-050 must have exactly one inherited GrantKeyword(Piercing) declarative clause"
    );
}

// ─── Section 2 — Clause 0 behavioral (BLOCKED) ───────────────────────────────
//
// Clause 0: "[Your Turn] When this Digimon would DNA digivolve into a blue
// Digimon card, gain 1 memory."
//
// BLOCKED: G-BEFORE-PAY-COST-DIGIVOLVE-TARGET + G-BEFORE-PAY-COST-GAIN-MEMORY.
// All behavioral tests for clause 0 are ignored until both gaps close.

/// DNA digivolving into a blue Digimon should gain 1 memory — BLOCKED.
#[test]
#[ignore = "pending: G-BEFORE-PAY-COST-DIGIVOLVE-TARGET + G-BEFORE-PAY-COST-GAIN-MEMORY from qa/dsl-vocab-gaps.md"]
fn bt12_050_dna_digivolving_into_blue_gains_one_memory() {
    // When gap closes: set up BT12-050 on field, initiate DNA digivolve into
    // a blue Digimon card from hand, assert memory increases by 1.
    //
    // Suggested setup:
    //   let mut runner = DebugRunner::builder()
    //       .dsl_card(CARD_ID)
    //       .add_card(make_blue_digimon("BLUE-DNA"))
    //       .hand(0, &["BLUE-DNA"])
    //       .memory(0)
    //       .start();
    //   let carrier = runner.place_on_field(0, CARD_ID, Some(0));
    //   let mat_b = runner.place_on_field(0, "MAT-B", None);
    //   // initiate DNA digivolve on carrier + mat_b into BLUE-DNA
    //   assert_eq!(runner.memory(), 1, "should gain 1 memory for blue DNA target");
}

/// DNA digivolving into a NON-blue Digimon should NOT gain memory — BLOCKED.
#[test]
#[ignore = "pending: G-BEFORE-PAY-COST-DIGIVOLVE-TARGET + G-BEFORE-PAY-COST-GAIN-MEMORY from qa/dsl-vocab-gaps.md"]
fn bt12_050_dna_digivolving_into_non_blue_does_not_gain_memory() {
    // When gap closes: DNA digivolve into a green/red Digimon → no memory gain.
}

/// On opponent's turn, the memory gain should NOT trigger — BLOCKED.
#[test]
#[ignore = "pending: G-BEFORE-PAY-COST-DIGIVOLVE-TARGET + G-BEFORE-PAY-COST-GAIN-MEMORY from qa/dsl-vocab-gaps.md"]
fn bt12_050_clause_0_does_not_fire_on_opponents_turn() {
    // When gap closes: end_turn to opponent; opponent DNA digivolves BT12-050
    // into blue Digimon → memory gain should NOT fire (your_turn: true gate).
}

// ─── Section 3 — Clause 1 behavioral (inherited Piercing) ────────────────────
//
// Clause 1: "[Your Turn] While this Digimon has [Imperialdramon] in its name
// or the [Free] trait, it gains <Piercing>"
//
// Over-fires: `active_when` is compiled but not consumed by lowering.
// Positive tests pass; negative tests are #[ignore]'d.

/// When BT12-050 is stacked under a carrier with [Imperialdramon] in its name,
/// the carrier should have Piercing. (Over-fires: any carrier gets Piercing due
/// to G-DSL-GRANT-KEYWORD-ACTIVE-WHEN-NOT-CONSUMED.)
#[test]
fn bt12_050_inherited_piercing_granted_when_carrier_has_imperialdramon_name() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-050 in embedded DSL pack")
        .add_card(make_imperialdramon_carrier("IMPERIALDRAMON"))
        .memory(0)
        .start();

    // Stack: [BT12-050 (bottom), IMPERIALDRAMON (top)]
    let carrier = runner.place_stack(0, &[CARD_ID, "IMPERIALDRAMON"]);

    let has_piercing = runner.game.has_keyword(carrier, Keyword::Piercing);
    assert!(
        has_piercing,
        "carrier with [Imperialdramon] in name should have Piercing from BT12-050 inherited effect"
    );
}

/// When BT12-050 is stacked under a carrier with the [Free] trait, the carrier
/// should have Piercing. (Over-fires regardless of trait due to
/// G-DSL-GRANT-KEYWORD-ACTIVE-WHEN-NOT-CONSUMED; this positive test passes
/// for the wrong reason — the trait check is silently dropped.)
#[test]
fn bt12_050_inherited_piercing_granted_when_carrier_has_free_trait() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-050 in embedded DSL pack")
        .add_card(make_free_trait_carrier("FREE-CARRIER"))
        .memory(0)
        .start();

    // Stack: [BT12-050 (bottom), FREE-CARRIER (top)]
    let carrier = runner.place_stack(0, &[CARD_ID, "FREE-CARRIER"]);

    let has_piercing = runner.game.has_keyword(carrier, Keyword::Piercing);
    assert!(
        has_piercing,
        "carrier with [Free] trait should have Piercing from BT12-050 inherited effect"
    );
}

/// When BT12-050 is stacked under a carrier WITHOUT [Imperialdramon] in name
/// or [Free] trait, the carrier should NOT have Piercing.
///
/// FAILS (over-fires): `active_when` condition is not consumed by lowering,
/// so Piercing is granted unconditionally to any carrier.
/// Ignored pending G-DSL-GRANT-KEYWORD-ACTIVE-WHEN-NOT-CONSUMED.
#[test]
#[ignore = "pending: G-DSL-GRANT-KEYWORD-ACTIVE-WHEN-NOT-CONSUMED from qa/dsl-vocab-gaps.md — active_when is compiled but discarded in lower_grant_keyword::lower; Piercing over-fires for all carriers"]
fn bt12_050_inherited_piercing_not_granted_when_carrier_has_no_matching_name_or_trait() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-050 in embedded DSL pack")
        .add_card(make_unrelated_carrier("TYRANNO"))
        .memory(0)
        .start();

    // Stack: [BT12-050 (bottom), TYRANNO (top)]
    let carrier = runner.place_stack(0, &[CARD_ID, "TYRANNO"]);

    let has_piercing = runner.game.has_keyword(carrier, Keyword::Piercing);
    assert!(
        !has_piercing,
        "carrier without [Imperialdramon] name or [Free] trait should NOT have Piercing"
    );
}

/// When BT12-050 is the only card on the field (no carrier), it should not
/// grant itself Piercing (it's an inherited effect, not a self-grant).
/// The `source_permanent` from `lower_grant_keyword` would resolve to None
/// for a top-card slot, so the process closure returns early without granting.
#[test]
fn bt12_050_no_piercing_when_alone_on_field_as_top_card() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-050 in embedded DSL pack")
        .memory(0)
        .start();

    let handle = runner.place_on_field(0, CARD_ID, Some(0));

    // BT12-050 is the top card — its inherited effect is not active for itself.
    let data_idx = runner.game.player(0).battle_area[0].top_card().data_index;
    let top_card_id = runner.game.card_data[data_idx].card_id.clone();
    assert_eq!(
        top_card_id, CARD_ID,
        "BT12-050 should be the top card when placed alone"
    );
    // Inherited effects from BT12-050 only apply when it's UNDER another card.
    // With only one card in the stack, no inherited grant fires.
    let _ = handle;
}

/// Carrier with [Imperialdramon] in name inherits Piercing even on opponent's
/// turn — FAILS (over-fires: your_turn gate not enforced due to active_when
/// being dropped).
#[test]
#[ignore = "pending: G-DSL-GRANT-KEYWORD-ACTIVE-WHEN-NOT-CONSUMED from qa/dsl-vocab-gaps.md — your_turn gate in active_when is silently dropped; Piercing fires on both turns"]
fn bt12_050_inherited_piercing_not_active_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-050 in embedded DSL pack")
        .add_card(make_imperialdramon_carrier("IMPERIALDRAMON"))
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "IMPERIALDRAMON"]);
    runner.end_turn(); // switch to player 1's turn

    // Now it is player 1's turn — BT12-050's [Your Turn] gate should block Piercing.
    let has_piercing = runner.game.has_keyword(carrier, Keyword::Piercing);
    assert!(
        !has_piercing,
        "Piercing should not be active on opponent's turn (your_turn gate)"
    );
}
