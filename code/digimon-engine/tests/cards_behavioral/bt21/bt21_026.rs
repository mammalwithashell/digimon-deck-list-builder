//! BT21-026 WarGreymon — Digimon, Lv.6, Red, Cost 11, DP 11000.
//! Traits: Dragonkin.
//!
//! # Card text (cards.json)
//!
//! ```text
//! When this card would be played, reduce the play cost by 2 for each of your
//! opponent's Digimon.
//! ＜Rush＞ (This Digimon can attack the turn it comes into play.)
//! ＜Raid＞ (When this Digimon attacks, you may switch the target of attack to
//!  1 of your opponent's unsuspended Digimon with the highest DP.)
//! ＜Blocker＞ (This Digimon can block in the blocker timing.)
//! [All Turns] [Once Per Turn] When any of your opponent's Digimon are deleted,
//! this Digimon may unsuspend.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Red/BT21_026.cs
//!
//! # DSL YAML
//! code/digimon-engine/cards/bt21/BT21-026.yaml
//!
//! # Patterns this test covers
//! - D2 cost_reduction with formula (card_count_in_zone opponent battle_area)
//! - H1 Rush keyword grant
//! - H5 Blocker keyword grant
//! - H9 Raid keyword grant
//! - [All Turns][OPT] on_any_deletion unsuspend self — G-EVENT-TARGET-OWNER BLOCKED
//!
//! # Known gaps and test status
//!
//! | Clause | Gap | Status |
//! |--------|-----|--------|
//! | Cost reduction by 2 x opp Digimon count | card_count_in_zone counts all BA perms | PARTIAL — Tamers over-counted; acceptable approximation |
//! | Rush | G-DECLARATIVE-KEYWORD | PARTIAL — compiled, not installed at runtime |
//! | Raid | G-DECLARATIVE-KEYWORD | PARTIAL — compiled, not installed at runtime |
//! | Blocker | G-DECLARATIVE-KEYWORD | PARTIAL — compiled, not installed at runtime |
//! | [All Turns][OPT] deletion -> unsuspend | G-EVENT-TARGET-OWNER | BLOCKED — omitted from YAML |

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind};

// --------------------------------------------------------------------------
// Helper: a simple Lv.5 Digimon filler card for the opponent side
// --------------------------------------------------------------------------

fn make_digimon_filler(id: &str) -> digimon_engine::card_data::CardData {
    digimon_engine::card_data::CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(5),
        dp: Some(5000),
        play_cost: 5,
        colors: vec![CardColor::Red],
        traits: vec!["Dragonkin".to_string()],
        evo_costs: vec![],
        dna_costs: vec![],
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: vec![],
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
    }
}

// --------------------------------------------------------------------------
// Helper: build the card-loaded runner
// --------------------------------------------------------------------------

fn wargreymon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT21-026")
        .expect("BT21-026 in embedded DSL pack")
        .memory(15)
        .start()
}

// ============================================================================
// Section 1 — Structural assertions
// ============================================================================

/// BT21-026 has exactly three declarative `grant_keyword` clauses:
/// Rush, Raid, and Blocker.
#[test]
fn bt21_026_has_three_declarative_grant_keyword_clauses() {
    let runner = wargreymon_runner();
    let card = runner
        .compiled_card("BT21-026")
        .expect("BT21-026 in embedded pack");

    let gk_count = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { .. })
            )
        })
        .count();
    assert_eq!(
        gk_count, 3,
        "expected 3 grant_keyword declaratives (Rush, Raid, Blocker)"
    );
}

/// The three declarative keyword grants are Rush, Raid, and Blocker.
#[test]
fn bt21_026_declarative_keywords_are_rush_raid_blocker() {
    let runner = wargreymon_runner();
    let card = runner
        .compiled_card("BT21-026")
        .expect("BT21-026 in embedded pack");

    let keywords: Vec<&str> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                ..
            }) => Some(keyword.as_str()),
            _ => None,
        })
        .collect();

    assert!(
        keywords.contains(&"Rush"),
        "Rush grant keyword must be present; found: {keywords:?}"
    );
    assert!(
        keywords.contains(&"Raid"),
        "Raid grant keyword must be present; found: {keywords:?}"
    );
    assert!(
        keywords.contains(&"Blocker"),
        "Blocker grant keyword must be present; found: {keywords:?}"
    );
}

/// BT21-026 has exactly one `cost_reduction` declarative clause.
#[test]
fn bt21_026_has_one_cost_reduction_clause() {
    let runner = wargreymon_runner();
    let card = runner
        .compiled_card("BT21-026")
        .expect("BT21-026 in embedded pack");

    let cr_count = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
            )
        })
        .count();
    assert_eq!(cr_count, 1, "expected 1 cost_reduction declarative");
}

/// The [All Turns][OPT] deletion arm is BLOCKED by G-EVENT-TARGET-OWNER and
/// is intentionally omitted from the YAML. BT21-026 therefore has ZERO triggered
/// clauses in v1.
#[test]
fn bt21_026_has_zero_triggered_clauses_deletion_arm_omitted() {
    let runner = wargreymon_runner();
    let card = runner
        .compiled_card("BT21-026")
        .expect("BT21-026 in embedded pack");

    let triggered_count = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    assert_eq!(
        triggered_count, 0,
        "deletion arm omitted (G-EVENT-TARGET-OWNER); zero triggered clauses expected"
    );
}

/// Rush keyword grant clause has FaceUp scope (printed as own-card keyword).
#[test]
fn bt21_026_rush_is_face_up_scope() {
    let runner = wargreymon_runner();
    let card = runner
        .compiled_card("BT21-026")
        .expect("BT21-026 in embedded pack");

    let found = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope,
                ..
            }) if keyword == "Rush" && *scope == CompiledScope::FaceUp
        )
    });
    assert!(
        found,
        "Rush must be a FaceUp-scope grant_keyword declarative"
    );
}

/// Blocker keyword grant clause has FaceUp scope.
#[test]
fn bt21_026_blocker_is_face_up_scope() {
    let runner = wargreymon_runner();
    let card = runner
        .compiled_card("BT21-026")
        .expect("BT21-026 in embedded pack");

    let found = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope,
                ..
            }) if keyword == "Blocker" && *scope == CompiledScope::FaceUp
        )
    });
    assert!(
        found,
        "Blocker must be a FaceUp-scope grant_keyword declarative"
    );
}

/// Raid keyword grant clause has FaceUp scope.
#[test]
fn bt21_026_raid_is_face_up_scope() {
    let runner = wargreymon_runner();
    let card = runner
        .compiled_card("BT21-026")
        .expect("BT21-026 in embedded pack");

    let found = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope,
                ..
            }) if keyword == "Raid" && *scope == CompiledScope::FaceUp
        )
    });
    assert!(
        found,
        "Raid must be a FaceUp-scope grant_keyword declarative"
    );
}

// ============================================================================
// Section 2 — Cost reduction behavioral tests
// ============================================================================

/// When the opponent has NO permanents in battle area, BT21-026 costs full 11.
#[test]
fn bt21_026_cost_reduction_zero_with_no_opp_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-026")
        .expect("BT21-026 in embedded pack")
        .hand(0, &["BT21-026"])
        .memory(15)
        .start();

    // P1 has empty battle area.
    let mem_before = runner.memory();
    runner.play(0, 0);
    let cost = mem_before - runner.memory();
    assert_eq!(
        cost, 11,
        "with no opp permanents, full 11 memory cost applies; got cost={cost}"
    );
}

/// When the opponent has 1 permanent in battle area, play cost is reduced by 2 (to 9).
#[test]
fn bt21_026_cost_reduction_minus_2_with_one_opp_permanent() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-026")
        .expect("BT21-026 in embedded pack")
        .add_card(make_digimon_filler("OPP-DIGI-1"))
        .hand(0, &["BT21-026"])
        .memory(15)
        .start();

    runner.place_on_field(1, "OPP-DIGI-1", None);

    let mem_before = runner.memory();
    runner.play(0, 0);
    let cost = mem_before - runner.memory();
    assert_eq!(
        cost, 9,
        "with 1 opp permanent, cost should be 11-2=9; got cost={cost}"
    );
}

/// When the opponent has 3 permanents in battle area, play cost is reduced by 6 (to 5).
#[test]
fn bt21_026_cost_reduction_minus_6_with_three_opp_permanents() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-026")
        .expect("BT21-026 in embedded pack")
        .add_card(make_digimon_filler("OPP-DIGI-A"))
        .add_card(make_digimon_filler("OPP-DIGI-B"))
        .add_card(make_digimon_filler("OPP-DIGI-C"))
        .hand(0, &["BT21-026"])
        .memory(15)
        .start();

    runner.place_on_field(1, "OPP-DIGI-A", None);
    runner.place_on_field(1, "OPP-DIGI-B", None);
    runner.place_on_field(1, "OPP-DIGI-C", None);

    let mem_before = runner.memory();
    runner.play(0, 0);
    let cost = mem_before - runner.memory();
    assert_eq!(
        cost, 5,
        "with 3 opp permanents, cost should be 11-6=5; got cost={cost}"
    );
}

/// The cost reduction only applies to BT21-026 itself, not other cards played
/// from hand while BT21-026 is on the field.
#[test]
fn bt21_026_cost_reduction_does_not_leak_to_other_cards() {
    let other_card = digimon_engine::card_data::CardData {
        card_id: "OTHER-DIGI".to_string(),
        card_name: "Other Digimon".to_string(),
        card_kind: CardKind::Digimon,
        level: Some(5),
        dp: Some(5000),
        play_cost: 8,
        colors: vec![CardColor::Red],
        traits: vec![],
        evo_costs: vec![],
        dna_costs: vec![],
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: vec![],
        dual: None,
        effect_class_name: "OTHER_DIGI".to_string(),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
    };

    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-026")
        .expect("BT21-026 in embedded pack")
        .add_card(make_digimon_filler("OPP-DIGI-A"))
        .add_card(other_card)
        .hand(0, &["OTHER-DIGI"])
        .memory(15)
        .start();

    // BT21-026 is on field (placed, bypassing play cost).
    // Opponent has 1 permanent.
    runner.place_on_field(0, "BT21-026", None);
    runner.place_on_field(1, "OPP-DIGI-A", None);

    let mem_before = runner.memory();
    runner.play(0, 0); // play OTHER-DIGI (cost 8) from hand
    let cost = mem_before - runner.memory();
    assert_eq!(
        cost, 8,
        "BT21-026's cost reduction must NOT apply to OTHER-DIGI; expected cost=8, got={cost}"
    );
}

// ============================================================================
// Section 3 — Keyword behavioral tests (pending G-DECLARATIVE-KEYWORD)
// ============================================================================

/// Rush is installed on BT21-026 when it enters the field.
///
/// Skipped pending G-DECLARATIVE-KEYWORD: DSL declarative `grant_keyword`
/// clauses compile to an `Effect::declarative(...)` with
/// `EffectTiming::Declarative`, but `Declarative` timing is never enqueued
/// or fired by the engine. The modifier is therefore never installed at runtime.
#[test]
#[ignore = "pending: G-DECLARATIVE-KEYWORD — EffectTiming::Declarative not yet fired; Rush modifier not installed at runtime"]
fn bt21_026_rush_installed_on_field() {
    todo!("pending G-DECLARATIVE-KEYWORD: Rush grant not installed at runtime")
}

/// Blocker is installed on BT21-026 when it enters the field.
///
/// Skipped pending G-DECLARATIVE-KEYWORD.
#[test]
#[ignore = "pending: G-DECLARATIVE-KEYWORD — EffectTiming::Declarative not yet fired; Blocker modifier not installed at runtime"]
fn bt21_026_blocker_installed_on_field() {
    todo!("pending G-DECLARATIVE-KEYWORD: Blocker grant not installed at runtime")
}

// ============================================================================
// Section 4 — [All Turns][OPT] deletion arm (BLOCKED: G-EVENT-TARGET-OWNER)
// ============================================================================

/// [All Turns][OPT] When any of your opponent's Digimon are deleted,
/// BT21-026 may unsuspend.
///
/// BLOCKED by G-EVENT-TARGET-OWNER: `on_any_deletion` fires for ALL deletions
/// (own and opponent). The DSL has no `event_target_owner: opponent` predicate
/// to filter "deleted permanent belonged to opponent". Omitted from YAML until
/// G-EVENT-TARGET-OWNER is closed.
///
/// Once closed, implement as:
///   - when: on_any_deletion
///     active_when: { all_turns: true }
///     once_per_turn: true
///     optional: true
///     condition: { event_target_owner: opponent }
///     process:
///       - unsuspend: { target: self }
///
/// Scaffolding:
///   1. Place BT21-026 on P0's field, suspend it (simulate attack).
///   2. Delete an opponent's Digimon via delete_permanent_with_effects.
///   3. Optional-unsuspend prompt installs.
///   4. Accept -> WarGreymon is unsuspended.
#[test]
#[ignore = "pending: G-EVENT-TARGET-OWNER — on_any_deletion has no predicate for event-target owner; deletion arm omitted from YAML"]
fn bt21_026_unsuspend_on_opp_digimon_deleted() {
    todo!("pending G-EVENT-TARGET-OWNER: deletion arm omitted from YAML")
}

/// [OPT] lockout: second opponent deletion in same turn should NOT re-trigger.
///
/// BLOCKED by G-EVENT-TARGET-OWNER.
///
/// Scaffolding:
///   1. Place BT21-026 + 2 opp Digimon.
///   2. Delete opp Digimon #1 -> prompt fires -> accept.
///   3. Delete opp Digimon #2 in same turn -> OPT lock -> no prompt.
#[test]
#[ignore = "pending: G-EVENT-TARGET-OWNER — deletion arm omitted from YAML; OPT lockout untestable until clause lands"]
fn bt21_026_unsuspend_opt_blocks_second_deletion_trigger() {
    todo!("pending G-EVENT-TARGET-OWNER: deletion arm omitted from YAML")
}

/// [OPT] lockout resets after end_turn.
///
/// BLOCKED by G-EVENT-TARGET-OWNER.
///
/// Scaffolding:
///   1. Trigger + accept unsuspend on turn N.
///   2. end_turn x2 to cycle back to P0's turn.
///   3. Delete another opp Digimon -> prompt fires again (lockout cleared).
#[test]
#[ignore = "pending: G-EVENT-TARGET-OWNER — deletion arm omitted from YAML; OPT reset untestable until clause lands"]
fn bt21_026_unsuspend_opt_resets_after_end_turn() {
    todo!("pending G-EVENT-TARGET-OWNER: deletion arm omitted from YAML")
}
