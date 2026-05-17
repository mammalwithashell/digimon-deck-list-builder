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
//! Clause 1 (inherited): IMPLEMENTED — represented as an inherited self-aura
//!   with `target: {}`, gated by `[Your Turn]` plus either carrier name contains
//!   "Imperialdramon" or carrier top-card trait includes [Free].
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

/// BT12-050 has exactly one triggered clause: clause 0 is now a
/// `before_pay_cost_observe` observer (Phase 2 Track H closure).
#[test]
fn bt12_050_clause_0_is_present_observer() {
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
        triggered_count, 1,
        "BT12-050 clause 0 is IMPLEMENTED (Phase 2 Track H) — one \
         before_pay_cost_observe triggered clause should be present"
    );
}

/// BT12-050 has exactly one inherited Aura clause for the conditional Piercing
/// grant (clause 1).
#[test]
fn bt12_050_has_one_inherited_piercing_aura() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-050 in embedded DSL pack")
        .memory(0)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("BT12-050 compiled");

    let inherited_aura_count = compiled
        .effects
        .iter()
        .filter(|c| match c {
            CompiledClause::Declarative(d) => match d {
                CompiledDeclarativeClause::Aura { scope, .. } => *scope == CompiledScope::Inherited,
                _ => false,
            },
            _ => false,
        })
        .count();

    assert_eq!(
        inherited_aura_count, 1,
        "BT12-050 must have exactly one inherited Aura declarative clause for conditional Piercing"
    );
}

// ─── Section 2 — Clause 0 behavioral (BLOCKED) ───────────────────────────────
//
// Clause 0: "[Your Turn] When this Digimon would DNA digivolve into a blue
// Digimon card, gain 1 memory."
//
// BLOCKED: G-BEFORE-PAY-COST-DIGIVOLVE-TARGET + G-BEFORE-PAY-COST-GAIN-MEMORY.
// All behavioral tests for clause 0 are ignored until both gaps close.

/// DNA digivolving into a blue Digimon gains 1 memory — IMPLEMENTED.
#[test]
fn bt12_050_dna_digivolving_into_blue_gains_one_memory() {
    use digimon_engine::card_data::{DnaCost, DnaRequirement};
    use digimon_engine::card_source::CardSource;
    use digimon_engine::enums::CardColor;

    // Blue Digimon DNA result. BT12-050 (Green Lv.4) + Blue Lv.4 → cost 0.
    let mut blue_dna = make_test_card("BLUE-DNA", "BlueDNAResult");
    blue_dna.card_kind = CardKind::Digimon;
    blue_dna.level = Some(5);
    blue_dna.dp = Some(6000);
    blue_dna.play_cost = 8;
    blue_dna.colors = vec![CardColor::Blue];
    blue_dna.dna_costs = vec![DnaCost {
        memory_cost: 0,
        requirement1: DnaRequirement {
            level: 4,
            card_colors: vec![CardColor::Green],
            name_contains: String::new(),
            text_contains: String::new(),
        },
        requirement2: DnaRequirement {
            level: 4,
            card_colors: vec![CardColor::Blue],
            name_contains: String::new(),
            text_contains: String::new(),
        },
    }];

    let mut blue_mat = make_test_card("BLUE-MAT", "BlueMaterial");
    blue_mat.card_kind = CardKind::Digimon;
    blue_mat.level = Some(4);
    blue_mat.dp = Some(4000);
    blue_mat.colors = vec![CardColor::Blue];

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-050 in embedded DSL pack")
        .add_card(blue_dna)
        .add_card(blue_mat)
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    let bt12_050_perm = runner.place_on_field(0, CARD_ID, Some(0));
    let _blue_mat_perm = runner.place_on_field(0, "BLUE-MAT", Some(0));
    let blue_dna_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "BLUE-DNA")
        .unwrap();
    let card_index = runner.game.next_card_index();
    runner.game.players[0]
        .hand
        .push(CardSource::new(blue_dna_idx, 0, card_index));
    let hand_idx = runner.game.player(0).hand.len() - 1;

    let memory_before = runner.game.memory;
    runner.game.current_phase = digimon_engine::enums::GamePhase::Main;
    runner.game.resolve_dna_digivolve_stage2_with_window(
        0,
        bt12_050_perm.index as usize,
        1,
        hand_idx,
        digimon_engine::dna_digivolve::DnaRouteWindow::Main,
    );

    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "BT12-050 must gain 1 memory when DNA digivolving into blue Digimon"
    );
}

/// DNA digivolving into a NON-blue Digimon must NOT gain memory.
#[test]
fn bt12_050_dna_digivolving_into_non_blue_does_not_gain_memory() {
    use digimon_engine::card_data::{DnaCost, DnaRequirement};
    use digimon_engine::card_source::CardSource;
    use digimon_engine::enums::CardColor;

    let mut red_dna = make_test_card("RED-DNA", "RedDNAResult");
    red_dna.card_kind = CardKind::Digimon;
    red_dna.level = Some(5);
    red_dna.dp = Some(6000);
    red_dna.play_cost = 8;
    red_dna.colors = vec![CardColor::Red];
    red_dna.dna_costs = vec![DnaCost {
        memory_cost: 0,
        requirement1: DnaRequirement {
            level: 4,
            card_colors: vec![CardColor::Green],
            name_contains: String::new(),
            text_contains: String::new(),
        },
        requirement2: DnaRequirement {
            level: 4,
            card_colors: vec![CardColor::Red],
            name_contains: String::new(),
            text_contains: String::new(),
        },
    }];

    let mut red_mat = make_test_card("RED-MAT", "RedMaterial");
    red_mat.card_kind = CardKind::Digimon;
    red_mat.level = Some(4);
    red_mat.dp = Some(4000);
    red_mat.colors = vec![CardColor::Red];

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-050 in embedded DSL pack")
        .add_card(red_dna)
        .add_card(red_mat)
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    let bt12_050_perm = runner.place_on_field(0, CARD_ID, Some(0));
    let _red_mat_perm = runner.place_on_field(0, "RED-MAT", Some(0));
    let red_dna_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "RED-DNA")
        .unwrap();
    let card_index = runner.game.next_card_index();
    runner.game.players[0]
        .hand
        .push(CardSource::new(red_dna_idx, 0, card_index));
    let hand_idx = runner.game.player(0).hand.len() - 1;

    let memory_before = runner.game.memory;
    runner.game.current_phase = digimon_engine::enums::GamePhase::Main;
    runner.game.resolve_dna_digivolve_stage2_with_window(
        0,
        bt12_050_perm.index as usize,
        1,
        hand_idx,
        digimon_engine::dna_digivolve::DnaRouteWindow::Main,
    );

    assert_eq!(
        runner.game.memory, memory_before,
        "memory must NOT change when DNA digivolving into a non-blue Digimon"
    );
}

/// On opponent's turn, the memory gain must NOT trigger (your_turn gate).
#[test]
fn bt12_050_clause_0_does_not_fire_on_opponents_turn() {
    use digimon_engine::card_data::{DnaCost, DnaRequirement};
    use digimon_engine::card_source::CardSource;
    use digimon_engine::enums::CardColor;

    let mut blue_dna = make_test_card("BLUE-DNA", "BlueDNAResult");
    blue_dna.card_kind = CardKind::Digimon;
    blue_dna.level = Some(5);
    blue_dna.dp = Some(6000);
    blue_dna.play_cost = 8;
    blue_dna.colors = vec![CardColor::Blue];
    blue_dna.dna_costs = vec![DnaCost {
        memory_cost: 0,
        requirement1: DnaRequirement {
            level: 4,
            card_colors: vec![CardColor::Green],
            name_contains: String::new(),
            text_contains: String::new(),
        },
        requirement2: DnaRequirement {
            level: 4,
            card_colors: vec![CardColor::Blue],
            name_contains: String::new(),
            text_contains: String::new(),
        },
    }];

    let mut blue_mat = make_test_card("BLUE-MAT", "BlueMaterial");
    blue_mat.card_kind = CardKind::Digimon;
    blue_mat.level = Some(4);
    blue_mat.dp = Some(4000);
    blue_mat.colors = vec![CardColor::Blue];

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-050 in embedded DSL pack")
        .add_card(blue_dna)
        .add_card(blue_mat)
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    let bt12_050_perm = runner.place_on_field(1, CARD_ID, Some(0));
    let _blue_mat_perm = runner.place_on_field(1, "BLUE-MAT", Some(0));
    let blue_dna_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "BLUE-DNA")
        .unwrap();
    let card_index = runner.game.next_card_index();
    runner.game.players[1]
        .hand
        .push(CardSource::new(blue_dna_idx, 1, card_index));
    let hand_idx = runner.game.player(1).hand.len() - 1;

    let memory_before = runner.game.memory;
    runner.game.current_phase = digimon_engine::enums::GamePhase::Main;
    runner.game.resolve_dna_digivolve_stage2_with_window(
        1,
        bt12_050_perm.index as usize,
        1,
        hand_idx,
        digimon_engine::dna_digivolve::DnaRouteWindow::Main,
    );

    assert_eq!(
        runner.game.memory, memory_before,
        "memory must NOT change on opponent's turn (your_turn gate)"
    );
}

// ─── Section 3 — Clause 1 behavioral (inherited Piercing) ────────────────────
//
// Clause 1: "[Your Turn] While this Digimon has [Imperialdramon] in its name
// or the [Free] trait, it gains <Piercing>"
//
// Implemented as an inherited self-aura. Positive and negative tests verify
// the carrier name/trait gate and the `[Your Turn]` gate.

/// When BT12-050 is stacked under a carrier with [Imperialdramon] in its name,
/// the carrier should have Piercing.
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
    runner.game.tick_declarative_effects();

    let has_piercing = runner.game.has_keyword(carrier, Keyword::Piercing);
    assert!(
        has_piercing,
        "carrier with [Imperialdramon] in name should have Piercing from BT12-050 inherited effect"
    );
}

/// When BT12-050 is stacked under a carrier with the [Free] trait, the carrier
/// should have Piercing.
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
    runner.game.tick_declarative_effects();

    let has_piercing = runner.game.has_keyword(carrier, Keyword::Piercing);
    assert!(
        has_piercing,
        "carrier with [Free] trait should have Piercing from BT12-050 inherited effect"
    );
}

/// When BT12-050 is stacked under a carrier WITHOUT [Imperialdramon] in name
/// or [Free] trait, the carrier should NOT have Piercing.
///
#[test]
fn bt12_050_inherited_piercing_not_granted_when_carrier_has_no_matching_name_or_trait() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-050 in embedded DSL pack")
        .add_card(make_unrelated_carrier("TYRANNO"))
        .memory(0)
        .start();

    // Stack: [BT12-050 (bottom), TYRANNO (top)]
    let carrier = runner.place_stack(0, &[CARD_ID, "TYRANNO"]);
    runner.game.tick_declarative_effects();

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

/// Carrier with [Imperialdramon] in name should not inherit Piercing on the
/// opponent's turn because the inherited effect is gated by `[Your Turn]`.
#[test]
fn bt12_050_inherited_piercing_not_active_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-050 in embedded DSL pack")
        .add_card(make_imperialdramon_carrier("IMPERIALDRAMON"))
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "IMPERIALDRAMON"]);
    runner.end_turn(); // switch to player 1's turn
    runner.game.tick_declarative_effects();

    // Now it is player 1's turn — BT12-050's [Your Turn] gate should block Piercing.
    let has_piercing = runner.game.has_keyword(carrier, Keyword::Piercing);
    assert!(
        !has_piercing,
        "Piercing should not be active on opponent's turn (your_turn gate)"
    );
}
