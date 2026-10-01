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

// ─── Colour-gated standard-circle alt path (W1 regression guard) ────────────
//
// The `kind: digivolve` alt path mirroring the printed "Yellow Lv.3" circle
// used to say `from: { level_eq: 3, color: yellow }`. `color` is not a
// DSL predicate key, so it was silently dropped and the path accepted a
// level-3 base of ANY colour. Now `color_is` — a wrong-colour base must be
// rejected; a right-colour base still digivolves.

fn bt15_037_color_gate_base(id: &str, color: digimon_engine::enums::CardColor) -> digimon_engine::card_data::CardData {
    let mut c = digimon_engine::debug_runner::make_test_card(id, "Wrongcolormon");
    c.card_kind = digimon_engine::enums::CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.traits = Vec::new();
    c
}

/// Try to digivolve BT15-037 (from hand) onto a level-3 base of `color`.
fn bt15_037_color_gate_try(color: digimon_engine::enums::CardColor) -> bool {
    let mut r = digimon_engine::debug_runner::DebugRunner::builder()
        .dsl_card("BT15-037")
        .expect("BT15-037 in embedded DSL pack")
        .add_card(bt15_037_color_gate_base("CG-BASE", color))
        .add_card(digimon_engine::debug_runner::make_test_card("CG-FILL", "CG-FILL"))
        .hand(0, &["BT15-037"])
        .deck(0, &["CG-FILL"; 5])
        .deck(1, &["CG-FILL"])
        .memory(10)
        .start();
    let base = r.place_on_field(0, "CG-BASE", Some(0));
    let hand_idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "BT15-037")
        .expect("BT15-037 in hand");
    r.game.digivolve_from_hand(
        0,
        hand_idx,
        base.index as usize,
        digimon_engine::enums::PlaySource::ByDigivolve,
    )
}

#[test]
fn bt15_037_standard_alt_path_rejects_wrong_color_base() {
    assert!(
        !bt15_037_color_gate_try(digimon_engine::enums::CardColor::Blue),
        "a blue level-3 base must NOT digivolve into BT15-037 via the Yellow Lv.3 alt path"
    );
}

#[test]
fn bt15_037_standard_alt_path_accepts_right_color_base() {
    assert!(
        bt15_037_color_gate_try(digimon_engine::enums::CardColor::Yellow),
        "a yellow level-3 base digivolves into BT15-037 via the Yellow Lv.3 alt path"
    );
}
