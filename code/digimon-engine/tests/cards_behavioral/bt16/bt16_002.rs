//! BT16-002 DemiVeemon — Digi-Egg, Lv.2, Blue.
//! Traits: Baby Dragon.
//!
//! # Card text (cards.json effect_description_eng)
//!
//! ```text
//! Inherited Effect [All Turns] While this Digimon has 2 or more colors,
//! it gets +1000 DP.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT16/Blue/BT16_002.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4)
//! - D4 Declarative aura (inherited passive +DP with runtime condition)
//! - G4 Inherited effect on Digi-Egg (fires from digivolution-source slot)
//!
//! # Faithfulness notes
//! - "this Digimon" = the carrier permanent (the Digimon stack containing
//!   BT16-002 as a digivolution source), not the BT16-002 egg itself.
//! - DCGO: ChangeSelfDPStaticEffect(isInheritedEffect: true, changeValue: +1000),
//!   condition: card.PermanentOfThisCard().TopCard.CardColors.Count >= 2.
//! - `while_condition: { self_color_count_gte: 2 }` evaluates the carrier's
//!   top-card color set using PredicateSubject::Permanent (correct path).
//! - `active_when` cannot be used because it evaluates with
//!   PredicateSubject::None → eval_no_subject_fields → always false for
//!   self_color_count_gte.
//! - The aura is installed once at field-entry (OnPlay / OnDigivolve, inherited)
//!   with Expiry::UntilCondition. The modifier lives in ModifierRegistry.ChangeDp.
//! - No once_per_turn, no optional: this is a static declarative aura, always
//!   active when the condition holds; no trigger or player choice involved.
//!
//! # Behavioral test approach
//! `place_stack` places the stack without firing triggers. To activate the
//! `while_condition` install, we call `enqueue_triggered(OnPlay, ...)` +
//! `drain_effect_queue()` after placement to simulate field-entry. This fires
//! the inherited OnPlay effect from BT16-002, which installs the ChangeDp
//! modifier with Expiry::UntilCondition. `reevaluate_until_condition_modifiers`
//! then runs and keeps or evicts the modifier based on the carrier's color count.
//! `effective_dp` (reading ModifierRegistry.ChangeDp sum) reflects the result.

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;

// ─── Card-data factories ─────────────────────────────────────────────────────

/// Build a carrier Digimon with exactly one color (blue). Single-color carrier
/// — the aura condition (2+ colors) must NOT be satisfied.
fn make_single_color_carrier(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Blue];
    card
}

/// Build a carrier Digimon with two colors (blue + red). Two-color carrier
/// — the aura condition (2+ colors) MUST be satisfied.
fn make_two_color_carrier(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Blue, CardColor::Red];
    card
}

/// Build a carrier Digimon with three colors (blue + red + green). Three-color
/// carrier — the aura condition (2+ colors) MUST be satisfied.
fn make_three_color_carrier(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Blue, CardColor::Red, CardColor::Green];
    card
}

// ─── Runner factories ─────────────────────────────────────────────────────────

fn base_runner_with_single_color() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT16-002")
        .expect("BT16-002 in embedded DSL pack")
        .add_card(make_single_color_carrier("SINGLE-CARRIER"))
        .memory(0)
        .start()
}

fn base_runner_with_two_colors() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT16-002")
        .expect("BT16-002 in embedded DSL pack")
        .add_card(make_two_color_carrier("TWO-COLOR-CARRIER"))
        .memory(0)
        .start()
}

fn base_runner_with_three_colors() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT16-002")
        .expect("BT16-002 in embedded DSL pack")
        .add_card(make_three_color_carrier("THREE-COLOR-CARRIER"))
        .memory(0)
        .start()
}

/// Fire the inherited OnPlay batch for a permanent. Simulates field-entry
/// to trigger the while_condition install for BT16-002's inherited aura.
fn fire_on_play(runner: &mut DebugRunner, handle: digimon_engine::permanent::PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// BT16-002 YAML must parse and compile cleanly.
#[test]
fn bt16_002_yaml_parses_and_compiles() {
    use digimon_dsl::{compile::compile, spec::CardSpec};
    let yaml = include_str!("../../../cards/bt16/BT16-002.yaml");
    let spec: CardSpec = serde_yml::from_str(yaml).expect("BT16-002 YAML must parse as CardSpec");
    let _compiled = compile(&spec).expect("BT16-002 YAML must compile without errors");
}

/// BT16-002 has exactly one declarative clause and zero triggered clauses in the
/// compiled DSL representation. `while_condition` is stored inside the Aura
/// variant (not as a separate Triggered clause), so the CompiledClause vector
/// has exactly one Declarative(Aura) entry.
#[test]
fn bt16_002_has_exactly_one_declarative_aura_clause() {
    let runner = base_runner_with_two_colors();
    let compiled = runner
        .compiled_card("BT16-002")
        .expect("BT16-002 compiled card present");

    // No triggered clauses (no when: field in the YAML).
    let triggered_count = compiled
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    assert_eq!(
        triggered_count, 0,
        "BT16-002 must have no triggered clauses — it is a static declarative aura; \
         found {triggered_count}"
    );

    // Exactly one declarative Aura clause.
    let aura_count = compiled
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Declarative(CompiledDeclarativeClause::Aura { .. })))
        .count();
    assert_eq!(
        aura_count, 1,
        "BT16-002 must have exactly one Aura declarative clause; found {aura_count}"
    );
}

/// The single declarative clause must have scope: Inherited.
#[test]
fn bt16_002_aura_clause_has_scope_inherited() {
    let runner = base_runner_with_two_colors();
    let compiled = runner
        .compiled_card("BT16-002")
        .expect("BT16-002 compiled card present");

    let aura = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura { scope, .. }) => {
                Some(*scope)
            }
            _ => None,
        })
        .expect("Aura clause must exist");

    assert_eq!(
        aura,
        CompiledScope::Inherited,
        "BT16-002 aura must have scope: Inherited (printed 'Inherited Effect')"
    );
}

/// The aura clause must declare dp_modifier: Some(1000).
#[test]
fn bt16_002_aura_clause_dp_modifier_is_1000() {
    let runner = base_runner_with_two_colors();
    let compiled = runner
        .compiled_card("BT16-002")
        .expect("BT16-002 compiled card present");

    let dp_modifier = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura { dp_modifier, .. }) => {
                Some(*dp_modifier)
            }
            _ => None,
        })
        .expect("Aura clause must exist");

    assert_eq!(
        dp_modifier,
        Some(1000),
        "BT16-002 aura must have dp_modifier: Some(1000); got {dp_modifier:?}"
    );
}

/// The aura clause must have a while_condition predicate (for the 2+ colors
/// condition). `while_condition` is the correct DSL field for `self_color_count_gte`
/// in an inherited aura — it evaluates against PredicateSubject::Permanent, while
/// `active_when` would use PredicateSubject::None and always return false.
#[test]
fn bt16_002_aura_clause_has_while_condition_predicate() {
    let runner = base_runner_with_two_colors();
    let compiled = runner
        .compiled_card("BT16-002")
        .expect("BT16-002 compiled card present");

    let while_condition = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                while_condition,
                ..
            }) => Some(while_condition.clone()),
            _ => None,
        })
        .expect("Aura clause must exist");

    assert!(
        while_condition.is_some(),
        "BT16-002 aura must have a while_condition predicate (self_color_count_gte: 2); \
         got None"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating (positive + negative)
// ═══════════════════════════════════════════════════════════════════════════════

/// POSITIVE: Carrier with 2 colors → effective_dp must include +1000 bonus.
///
/// BT16-002 is stacked as a digivolution source under a two-color carrier.
/// After firing the OnPlay trigger (which installs the while_condition modifier),
/// the carrier's effective_dp must be 1000 higher than its base DP.
///
/// `make_test_card` sets base_dp = Some(2000). With BT16-002's +1000 bonus the
/// expected effective_dp is 3000. Without the bonus it would be 2000.
///
/// The while_condition predicate (`self_color_count_gte: 2`) is satisfied for
/// a 2-color carrier → the UntilCondition modifier survives reevaluation.
#[test]
fn bt16_002_effective_dp_active_for_two_color_carrier() {
    let mut runner = base_runner_with_two_colors();

    // place_stack: card_ids[last] = top card, card_ids[0..last-1] = sources below.
    // BT16-002 at index 0 (bottom digivolution source), TWO-COLOR-CARRIER on top.
    let handle = runner.place_stack(0, &["BT16-002", "TWO-COLOR-CARRIER"]);

    // Fire inherited OnPlay to install the while_condition modifier.
    fire_on_play(&mut runner, handle);

    // make_test_card base_dp = 2000; bonus = 1000 → expected = 3000.
    let effective = runner.effective_dp(handle).unwrap_or(0);
    assert_eq!(
        effective, 3000,
        "BT16-002 must add +1000 DP when carrier has 2 colors; \
         expected base(2000) + bonus(1000) = 3000, got {effective}"
    );
}

/// POSITIVE: Carrier with 3 colors → the condition (≥2) is also satisfied.
/// Verifies the condition is ≥2, not strictly =2.
#[test]
fn bt16_002_effective_dp_active_for_three_color_carrier() {
    let mut runner = base_runner_with_three_colors();

    let handle = runner.place_stack(0, &["BT16-002", "THREE-COLOR-CARRIER"]);
    fire_on_play(&mut runner, handle);

    // make_test_card base_dp = 2000; bonus = 1000 → expected = 3000.
    let effective = runner.effective_dp(handle).unwrap_or(0);
    assert_eq!(
        effective, 3000,
        "BT16-002 must add +1000 DP when carrier has 3 colors (≥2 condition); \
         expected base(2000) + bonus(1000) = 3000, got {effective}"
    );
}

/// NEGATIVE: Carrier with 1 color → condition not satisfied → no DP bonus.
///
/// The `self_color_count_gte: 2` condition requires at least 2 distinct colors.
/// A single-color carrier (blue only) does not satisfy the threshold.
/// The UntilCondition modifier is installed then immediately evicted by
/// `reevaluate_until_condition_modifiers` (called after drain_effect_queue).
///
/// `make_test_card` base_dp = 2000; no bonus → effective_dp stays at 2000.
#[test]
fn bt16_002_effective_dp_inactive_for_single_color_carrier() {
    let mut runner = base_runner_with_single_color();

    let handle = runner.place_stack(0, &["BT16-002", "SINGLE-CARRIER"]);
    fire_on_play(&mut runner, handle);

    // make_test_card base_dp = 2000; no bonus → effective_dp = 2000.
    let effective = runner.effective_dp(handle).unwrap_or(0);
    assert_eq!(
        effective, 2000,
        "BT16-002 must NOT add DP when carrier has only 1 color; \
         expected base(2000) only, got {effective}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: digivolve flow
// ═══════════════════════════════════════════════════════════════════════════════

/// BEHAVIORAL POSITIVE: Carrier with 2 colors has effective DP boosted by +1000.
///
/// Verifies the full install path: place_stack → fire_on_play → effective_dp.
/// The inherited OnPlay fires from BT16-002 as a digivolution source, installs
/// the ChangeDp modifier, which survives reevaluation for a 2-color carrier.
///
/// `make_test_card` base_dp = 2000. With BT16-002's +1000, expected = 3000.
#[test]
fn bt16_002_effective_dp_boosted_for_two_color_carrier() {
    let mut runner = base_runner_with_two_colors();

    let handle = runner.place_stack(0, &["BT16-002", "TWO-COLOR-CARRIER"]);
    fire_on_play(&mut runner, handle);

    let effective = runner.effective_dp(handle).unwrap_or(0);
    assert_eq!(
        effective, 3000,
        "effective_dp of the carrier stack must be base(2000) + bonus(1000) = 3000; \
         got {effective}"
    );
}

/// BEHAVIORAL NEGATIVE: Carrier with 1 color has NO DP boost from BT16-002.
///
/// The inherited aura is conditionally inactive — effective DP should remain
/// at the carrier's base DP (2000) with no bonus from BT16-002.
#[test]
fn bt16_002_effective_dp_not_boosted_for_single_color_carrier() {
    let mut runner = base_runner_with_single_color();

    let handle = runner.place_stack(0, &["BT16-002", "SINGLE-CARRIER"]);
    fire_on_play(&mut runner, handle);

    let effective = runner.effective_dp(handle).unwrap_or(0);
    assert_eq!(
        effective, 2000,
        "effective_dp of the carrier stack must stay at base(2000) when carrier \
         has only 1 color; condition not met, got {effective}"
    );
}

/// BEHAVIORAL: BT16-002 stacked as bottom source in a 3-card stack still works.
///
/// Verifies that the source index depth doesn't prevent the inherited OnPlay
/// from firing. All digivolution sources fire inherited effects when the carrier
/// fires OnPlay.
///
/// Stack: BT16-002 (bottom) / MID-CARD (single-color) / TOP-CARD (2-color, top).
/// The condition checks the top-card colors → 2-color → satisfied.
/// TOP-CARD base_dp = 2000; expected = 2000 + 1000 = 3000.
#[test]
fn bt16_002_dp_contribution_when_stacked_as_bottom_in_three_card_stack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-002")
        .expect("BT16-002 in embedded DSL pack")
        .add_card(make_single_color_carrier("MID-CARD"))
        .add_card(make_two_color_carrier("TOP-CARD"))
        .memory(0)
        .start();

    // Stack: [BT16-002 (index 0), MID-CARD (index 1), TOP-CARD (index 2, top)].
    let handle = runner.place_stack(0, &["BT16-002", "MID-CARD", "TOP-CARD"]);
    fire_on_play(&mut runner, handle);

    // TOP-CARD has 2 colors; the condition checks the carrier's top card → satisfied.
    // TOP-CARD base_dp = 2000; bonus = 1000 → expected = 3000.
    let effective = runner.effective_dp(handle).unwrap_or(0);
    assert_eq!(
        effective, 3000,
        "BT16-002 at bottom of a 3-card stack under a 2-color top card must \
         add +1000 DP; expected base(2000) + bonus(1000) = 3000, got {effective}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Event log / cost firing
// ═══════════════════════════════════════════════════════════════════════════════

/// BT16-002's while_condition install fires via inherited OnPlay. After the
/// trigger fires and the queue drains, there must be no pending selection.
/// The aura is automatic — no player choice is required for the DP grant.
///
/// `make_test_card` base_dp = 2000; with +1000 bonus expected effective = 3000.
#[test]
fn bt16_002_aura_fires_no_pending_selection_after_install() {
    let mut runner = base_runner_with_two_colors();

    let handle = runner.place_stack(0, &["BT16-002", "TWO-COLOR-CARRIER"]);
    fire_on_play(&mut runner, handle);

    assert!(
        runner.pending_selection().is_none(),
        "BT16-002 static aura must not install any pending selection; \
         got Some({:?})",
        runner.pending_selection()
    );

    // DP bonus is immediately visible after drain (no further action required).
    let effective = runner.effective_dp(handle).unwrap_or(0);
    assert_eq!(
        effective, 3000,
        "inherited aura bonus must be active immediately after OnPlay drain; \
         expected base(2000) + bonus(1000) = 3000, got {effective}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — OPT (not applicable for static aura)
// ═══════════════════════════════════════════════════════════════════════════════

/// BT16-002 has no once_per_turn restriction: it is a static declarative aura,
/// not a triggered effect. The compiled card must have no triggered clauses with
/// once_per_turn set.
///
/// This documents the absence of OPT intentionally, distinguishing BT16-002
/// from sibling eggs like BT12-002 (which has [When Attacking][Once Per Turn]).
#[test]
fn bt16_002_has_no_once_per_turn_restriction() {
    let runner = base_runner_with_two_colors();
    let compiled = runner
        .compiled_card("BT16-002")
        .expect("BT16-002 compiled card present");

    // No triggered clause with once_per_turn (there are no triggered clauses at all).
    let any_opt = compiled.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => t.once_per_turn,
        _ => false,
    });
    assert!(
        !any_opt,
        "BT16-002 must have no once_per_turn triggered clause — it is a static aura"
    );

    // The aura does not have once_per_turn semantics by definition.
    // Confirm by verifying the only clause is declarative (already confirmed in section 1).
    let only_declarative = compiled
        .effects
        .iter()
        .all(|c| matches!(c, CompiledClause::Declarative(_)));
    assert!(
        only_declarative,
        "all BT16-002 clauses must be Declarative (no Triggered); OPT is N/A for static auras"
    );
}
