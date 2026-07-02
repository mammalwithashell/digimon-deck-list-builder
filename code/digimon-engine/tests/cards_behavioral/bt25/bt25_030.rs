//! BT25-030 Elecmon — Digimon, Lv.3, Yellow, DP 2000, Cost 3.
//! Traits: Mammal, Iliad, TS. Attribute: Data.
//!
//! Printed effect text (cards.json):
//!   [Start of Your Main Phase] By adding your top security card to the hand,
//!   gain 1 memory.
//!
//! Printed inherited text:
//!   [When Attacking] [Once Per Turn] You may add your top security card to the
//!   hand. Then, if you have 0 security cards, <Recovery +1> (Place the top card
//!   of your deck as your top security card.)
//!
//! DCGO C# reference: DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_030.cs
//!
//! Pattern rows (RUST_DSL_TEST_API §4.3): start-of-main optional cost-for-memory
//! (Group D), inherited When Attacking may-add-security + conditional Recovery
//! (Group E/G), alt-digivolve recipe registration.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledStep, CompiledTiming};
use digimon_engine::action::space::{PASS, SEL_MY_SECURITY_START};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::{EffectTiming, TriggerSource};

fn zone_ids(
    cards: &[digimon_engine::card_source::CardSource],
    data: &[digimon_engine::card_data::CardData],
) -> Vec<String> {
    cards.iter().map(|c| c.card_id(data).to_string()).collect()
}

// ── Section 1: Structural ──────────────────────────────────────────────────

#[test]
fn bt25_030_has_start_of_main_optional_memory_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("BT25-030")
        .expect("BT25-030 YAML loads")
        .build();
    let card = runner.compiled_card("BT25-030").expect("compiled");

    let start_main = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::FaceUp
                    && t.when.contains(&CompiledTiming::StartOfYourMainPhase) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("BT25-030 must have a face [Start of Your Main Phase] clause");

    assert!(
        start_main.optional,
        "DCGO SetUpActivateClass isOptional: true → the start-of-main effect must be optional"
    );
    let adds_security = start_main
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::AddTopSecurityToHand { .. }));
    let gains_memory = start_main
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::GainMemory { .. }));
    assert!(adds_security, "must add top security to hand (the cost)");
    assert!(gains_memory, "must gain memory");
}

#[test]
fn bt25_030_has_inherited_when_attacking_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("BT25-030")
        .expect("BT25-030 YAML loads")
        .build();
    let card = runner.compiled_card("BT25-030").expect("compiled");

    let inherited = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("BT25-030 must have an inherited [When Attacking] clause");

    assert!(
        inherited.once_per_turn,
        "[Once Per Turn] → the inherited clause must be once_per_turn"
    );
    let may_add = inherited
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::MayAddTopSecurityToHand { .. }));
    assert!(
        may_add,
        "inherited clause must surface the optional add-top-security ('you may')"
    );
}

#[test]
fn bt25_030_registers_ts_trait_alt_digivolve() {
    use digimon_dsl::compiled::CompiledAltPathKind;
    let runner = DebugRunner::builder()
        .dsl_card("BT25-030")
        .expect("BT25-030 YAML loads")
        .build();
    let card = runner.compiled_card("BT25-030").expect("compiled");
    let digivolve_paths = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert_eq!(
        digivolve_paths, 2,
        "BT25-030 must register standard Yellow and alt TS-trait digivolve paths"
    );
}

// ── Section 3: Behavioral — Start of Your Main Phase ───────────────────────

#[test]
fn bt25_030_start_of_main_adds_top_security_and_gains_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-030")
        .expect("BT25-030 YAML loads")
        .add_card(make_test_card("SEC", "Security"))
        .security(0, &["SEC"])
        .memory(0)
        .start();

    let carrier = runner.place_on_field(0, "BT25-030", None);
    let security_before = runner.security_count(0);
    let memory_before = runner.memory();

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    // Optional activation → the engine surfaces an accept/decline gate.
    // Accept by taking the first valid (non-PASS) action it offers.
    let sel = runner
        .pending_selection()
        .expect("start-of-main optional gate surfaces");
    assert!(sel.is_optional, "start-of-main effect is optional");
    let accept = sel
        .valid_action_ids
        .iter()
        .copied()
        .find(|a| *a != PASS)
        .expect("an accept action is offered");
    runner
        .execute_action(0, accept)
        .expect("accept security-for-memory");
    // The add-security/gain-memory body may chain further selections; drain.
    let _ = runner.auto_resolve();

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"SEC".to_string()),
        "top security moved to hand: {hand_ids:?}"
    );
    assert_eq!(
        runner.security_count(0),
        security_before - 1,
        "one security card was added to hand"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "gained exactly 1 memory"
    );
}

/// Negative: declining the optional start-of-main effect keeps security and
/// grants no memory.
#[test]
fn bt25_030_start_of_main_decline_keeps_security_no_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-030")
        .expect("BT25-030 YAML loads")
        .add_card(make_test_card("SEC", "Security"))
        .security(0, &["SEC"])
        .memory(0)
        .start();

    let carrier = runner.place_on_field(0, "BT25-030", None);
    let security_before = runner.security_count(0);
    let memory_before = runner.memory();

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    assert!(runner.pending_is_optional());
    runner
        .execute_action(0, PASS)
        .expect("decline start-of-main");

    assert_eq!(
        runner.security_count(0),
        security_before,
        "declining keeps the security card"
    );
    assert_eq!(runner.memory(), memory_before, "declining gains no memory");
    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(!hand_ids.contains(&"SEC".to_string()));
}

/// Negative (condition gate): with 0 security cards the start-of-main effect
/// must not fire at all (no pending selection, no memory) — the cost
/// "by adding your top security card" cannot be paid.
#[test]
fn bt25_030_start_of_main_does_nothing_at_zero_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-030")
        .expect("BT25-030 YAML loads")
        .memory(0)
        .start();

    let carrier = runner.place_on_field(0, "BT25-030", None);
    let memory_before = runner.memory();

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no security → effect cannot activate, no pending selection"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "no memory gained at 0 security"
    );
}

// ── Section 3: Behavioral — inherited When Attacking ───────────────────────

#[test]
fn bt25_030_inherited_adds_top_security_then_recovers_at_zero() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-030")
        .expect("BT25-030 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("SECURITY", "Security"))
        .add_card(make_test_card("RECOVER", "Recover"))
        .security(0, &["SECURITY"])
        .deck(0, &["RECOVER"])
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &["BT25-030", "CARRIER"]);
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    // The "you may add top security" surfaces as an optional pick.
    assert!(runner.pending_is_optional());
    runner
        .execute_action(0, SEL_MY_SECURITY_START)
        .expect("accept security add");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"SECURITY".to_string()));
    // Security dropped to 0, so Recovery +1 placed RECOVER as new top security.
    assert_eq!(runner.security_count(0), 1);
    assert_eq!(
        runner.game.players[0]
            .security
            .last()
            .unwrap()
            .card_id(&runner.game.card_data),
        "RECOVER"
    );
}

/// Negative: declining the inherited add-security leaves security intact, and
/// since security is NOT zero, Recovery does not fire.
#[test]
fn bt25_030_inherited_decline_keeps_security_no_recovery() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-030")
        .expect("BT25-030 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("SECURITY", "Security"))
        .add_card(make_test_card("RECOVER", "Recover"))
        .security(0, &["SECURITY"])
        .deck(0, &["RECOVER"])
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &["BT25-030", "CARRIER"]);
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    assert!(runner.pending_is_optional());
    runner
        .execute_action(0, PASS)
        .expect("decline security add");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(!hand_ids.contains(&"SECURITY".to_string()));
    assert_eq!(runner.security_count(0), 1);
    assert_eq!(
        runner.game.players[0]
            .security
            .last()
            .unwrap()
            .card_id(&runner.game.card_data),
        "SECURITY",
        "security unchanged, Recovery must not fire above 0 security"
    );
}

/// When the carrier starts at 0 security, the may-add fizzles (nothing to add)
/// and Recovery +1 fires unconditionally, placing the deck top as new security.
#[test]
fn bt25_030_inherited_recovers_when_security_already_zero() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-030")
        .expect("BT25-030 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("RECOVER", "Recover"))
        .deck(0, &["RECOVER"])
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &["BT25-030", "CARRIER"]);
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no security to add → no prompt; Recovery resolves directly"
    );
    assert_eq!(runner.security_count(0), 1);
    assert_eq!(
        runner.game.players[0]
            .security
            .last()
            .unwrap()
            .card_id(&runner.game.card_data),
        "RECOVER"
    );
}
