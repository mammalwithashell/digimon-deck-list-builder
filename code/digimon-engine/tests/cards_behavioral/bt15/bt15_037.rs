//! BT15-037 Gatomon — Digimon, Lv.4, Yellow, DP 4000, Cost 5.
//!
//! Printed text (card face authoritative):
//!  - When an effect trashes this card from the security stack, you may play it
//!    without paying the cost.  (BLOCKED — no DSL OnDiscardSecurity trigger;
//!    G-DSL-ON-DISCARD-SECURITY-TRIGGER. Omitted, see YAML header.)
//!  - <Barrier>
//!  - [All Turns][Once Per Turn] When a card is removed from your security stack,
//!    gain 1 memory.  (DCGO gates on IsExistOnBattleArea(card) — only fires while
//!    Gatomon is in the battle area; the judge-quiz Q9 crux.)
//!  - Inherited <Barrier>.

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const GATOMON_YAML: &str = include_str!("../../../cards/bt15/BT15-037.yaml");

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(3000);
    c.colors = vec![CardColor::Red];
    c
}

fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(GATOMON_YAML)
        .expect("BT15-037 YAML parses")
        .add_card(filler("ATTACKER"))
        .add_card(filler("SEC"))
        .add_card(filler("DECK"))
        .security(0, &["SEC", "SEC"])
        .deck(0, &["DECK"])
        .deck(1, &["DECK"])
        .memory(0)
        .start()
}

// ─── Structural ──────────────────────────────────────────────────────────────

#[test]
fn bt15_037_loads() {
    DebugRunner::builder()
        .from_dsl_yaml(GATOMON_YAML)
        .expect("BT15-037 must load from DSL YAML")
        .start();
}

#[test]
fn bt15_037_grants_barrier_face_and_inherited() {
    let runner = base_runner();
    let card = runner.compiled_card("BT15-037").expect("compiled card");

    let barrier_clauses: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope,
                ..
            }) if keyword == "Barrier" => Some(scope.clone()),
            _ => None,
        })
        .collect();

    assert!(
        barrier_clauses.iter().any(|s| *s == CompiledScope::FaceUp),
        "Gatomon must grant <Barrier> on its own face"
    );
    assert!(
        barrier_clauses
            .iter()
            .any(|s| *s == CompiledScope::Inherited),
        "Gatomon must grant inherited <Barrier>"
    );
}

#[test]
fn bt15_037_has_all_turns_own_security_removed_memory_clause() {
    let runner = base_runner();
    let card = runner.compiled_card("BT15-037").expect("compiled card");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnOwnSecurityRemoved) =>
            {
                Some(t)
            }
            _ => None,
        })
        .next()
        .expect("Gatomon must have an on_own_security_removed clause");

    assert!(
        clause.once_per_turn,
        "the gain-memory clause is [Once Per Turn]"
    );
    assert!(
        !clause.optional,
        "the gain-memory clause is mandatory (no 'may') — DCGO isOptional=false"
    );
}

// ─── Behavioral ──────────────────────────────────────────────────────────────

/// POSITIVE: Gatomon ON THE FIELD gains 1 memory when a card is removed from its
/// owner's security stack. (The Q9 NEGATIVE — Gatomon in security, no memory —
/// is pinned in the judge_quiz suite, where Mastemon's trim removes Gatomon while
/// it sits in security and is therefore not a battle-area trigger source.)
#[test]
fn bt15_037_on_field_gains_memory_when_own_security_removed() {
    let mut runner = base_runner();
    // Gatomon on P0's field; P1 has an attacker.
    let _gatomon = runner.place_on_field(0, "BT15-037", Some(0));
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));

    let before = runner.memory();
    // P1 attacks P0 (the player) → removes a card from P0's security → fires
    // Gatomon's [All Turns] on_own_security_removed → +1 memory for P0.
    runner.attack_player(attacker, 0, true);
    let _ = runner.auto_resolve();

    // P0 gaining 1 memory moves the shared gauge 1 step toward P0.
    let delta = (runner.memory() as i32 - before as i32).abs();
    assert_eq!(
        delta, 1,
        "Gatomon on the field must gain its owner +1 memory when their security is removed"
    );
}
