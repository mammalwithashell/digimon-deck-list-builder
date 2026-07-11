//! Store-champs predicate/formula leaves — batch I, leaves 3/4/5.
//!
//! * G-DSL-TOTAL-SECURITY-COUNT-PREDICATE — `total_security_count_{lte,gte,eq}`
//!   (both players' security stacks summed). Driver BT13-106 Odin's Breath.
//! * G-DSL-FORMULA-PLAYER-MEMORY — `per: { player_memory: { of } }` formula
//!   selector (opponent clamped ≥0). Driver BT25-086 Dan Yuki.
//! * G-DSL-FORMULA-SUBTRACT + G-DSL-TRASH-TOP-SECURITY-LEAVE — `subtract`
//!   compound formula and `leave: N` on `trash_top_security`. Driver BT21-098
//!   Ragnarok Cannon.

use digimon_dsl::compile::compile;
use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledDpConstraint, CompiledFormula,
    CompiledPerSelector, CompiledPlayerRef, CompiledPredicate, CompiledStep,
};
use digimon_dsl::spec::CardSpec;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::predicate::{eval_predicate_with_bindings, PredicateSubject};
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::{EffectContext, EffectReadContext};
use digimon_engine::enums::CardKind;
use digimon_engine::permanent::PermanentHandle;

fn digimon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c
}

fn compile_steps(yaml: &str) -> Vec<CompiledStep> {
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("yaml compiles");
    let CompiledClause::Triggered(clause) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    clause.process.clone()
}

// ===========================================================================
// Leaf 3 — total_security_count
// ===========================================================================

fn security_runner(own: usize, opp: usize) -> DebugRunner {
    let sec: Vec<CardData> = (0..(own.max(opp)))
        .map(|i| digimon(&format!("SEC{i}"), &format!("Sec {i}")))
        .collect();
    let mut b = DebugRunner::builder().add_card(digimon("SRC", "Source"));
    for c in sec {
        b = b.add_card(c);
    }
    let own_ids: Vec<String> = (0..own).map(|i| format!("SEC{i}")).collect();
    let opp_ids: Vec<String> = (0..opp).map(|i| format!("SEC{i}")).collect();
    let own_refs: Vec<&str> = own_ids.iter().map(|s| s.as_str()).collect();
    let opp_refs: Vec<&str> = opp_ids.iter().map(|s| s.as_str()).collect();
    b.hand(0, &["SRC"])
        .security(0, &own_refs)
        .security(1, &opp_refs)
        .build()
}

fn total_security(op: &str, n: i32) -> CompiledPredicate {
    let dp = Some(CompiledDpConstraint::Literal(n));
    match op {
        "lte" => CompiledPredicate {
            total_security_count_lte: dp,
            ..Default::default()
        },
        "gte" => CompiledPredicate {
            total_security_count_gte: dp,
            ..Default::default()
        },
        "eq" => CompiledPredicate {
            total_security_count_eq: dp,
            ..Default::default()
        },
        _ => unreachable!(),
    }
}

#[test]
fn parse_compile_total_security_count() {
    let yaml = r#"
card: TEST-TOTAL-SEC
name: Total Sec
kind: option
color: [yellow]
cost: 0
effects:
  - when: main_from_hand
    process:
      - if:
          condition: { total_security_count_lte: 6 }
          then:
            - draw: { of: you, count: 1 }
"#;
    // Just prove it parses + compiles (the predicate lives inside a conditional).
    let spec: CardSpec = serde_yml::from_str(yaml).expect("parse");
    assert!(compile(&spec).is_ok(), "total_security_count_lte compiles");
}

#[test]
fn total_security_count_sums_both_players() {
    // 4 own + 2 opp = 6 total.
    let runner = security_runner(4, 2);
    let src = runner.game.players[0].hand[0].handle();
    let rctx = EffectReadContext::new(&runner.game, src, None, 0);
    let subject = PredicateSubject::Card(src);

    // <= 6 : true at 6.
    assert!(eval_predicate_with_bindings(
        &total_security("lte", 6),
        &rctx,
        subject,
        None
    ));
    // <= 5 : false at 6.
    assert!(!eval_predicate_with_bindings(
        &total_security("lte", 5),
        &rctx,
        subject,
        None
    ));
    // >= 6 : true at 6.
    assert!(eval_predicate_with_bindings(
        &total_security("gte", 6),
        &rctx,
        subject,
        None
    ));
    // >= 7 : false at 6.
    assert!(!eval_predicate_with_bindings(
        &total_security("gte", 7),
        &rctx,
        subject,
        None
    ));
    // == 6 : true.
    assert!(eval_predicate_with_bindings(
        &total_security("eq", 6),
        &rctx,
        subject,
        None
    ));
    // == 5 : false.
    assert!(!eval_predicate_with_bindings(
        &total_security("eq", 5),
        &rctx,
        subject,
        None
    ));
}

#[test]
fn total_security_count_is_not_own_only() {
    // 5 own + 5 opp = 10 total; the BT13-106 "6 or fewer" gate must be FALSE
    // even though each player alone is <= 6. Proves it sums both, not one side.
    let runner = security_runner(5, 5);
    let src = runner.game.players[0].hand[0].handle();
    let rctx = EffectReadContext::new(&runner.game, src, None, 0);
    let subject = PredicateSubject::Card(src);
    assert!(!eval_predicate_with_bindings(
        &total_security("lte", 6),
        &rctx,
        subject,
        None
    ));
}

// ===========================================================================
// Leaf 4 — player_memory formula
// ===========================================================================

fn memory_dp_formula(of: CompiledPlayerRef) -> CompiledFormula {
    // base 0 + player_memory * 1000  (BT25-086 shape).
    CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::PlayerMemory { of },
        delta: 1000,
    }
}

/// A permanent to serve as the formula target / effect source.
fn memory_runner() -> (DebugRunner, PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .add_card(digimon("CARRIER", "Carrier"))
        .start();
    let handle = runner.place_on_field(0, "CARRIER", None);
    (runner, handle)
}

#[test]
fn player_memory_opponent_clamped_at_zero() {
    let (mut runner, handle) = memory_runner();
    let top = runner.top_card(handle);

    // Turn player is player 0 (the carrier's controller). `game.memory` is
    // stored from the turn player's perspective. Set +3 (controller has +3;
    // opponent's MemoryForPlayer is -3 → clamped to 0).
    runner.game.set_memory(3);
    {
        let ctx = EffectContext::new(&mut runner.game, top, Some(handle), 0);
        // opponent memory = max(0, -3) = 0 → +0 DP.
        assert_eq!(
            digimon_engine::dsl_cards::formula_eval::evaluate(
                &memory_dp_formula(CompiledPlayerRef::Opponent),
                &ctx,
                handle
            ),
            0,
            "opponent memory clamped to 0 when negative"
        );
        // controller (you) memory = +3 (unclamped) → +3000 DP.
        assert_eq!(
            digimon_engine::dsl_cards::formula_eval::evaluate(
                &memory_dp_formula(CompiledPlayerRef::You),
                &ctx,
                handle
            ),
            3000
        );
    }
}

#[test]
fn player_memory_opponent_positive_scales_dp() {
    let (mut runner, handle) = memory_runner();
    let top = runner.top_card(handle);
    // Controller's turn, memory -2 (controller is at -2; opponent MemoryForPlayer
    // = +2). Opponent memory = max(0, +2) = 2 → +2000 DP (BT25-086 payoff).
    runner.game.set_memory(-2);
    let ctx = EffectContext::new(&mut runner.game, top, Some(handle), 0);
    assert_eq!(
        digimon_engine::dsl_cards::formula_eval::evaluate(
            &memory_dp_formula(CompiledPlayerRef::Opponent),
            &ctx,
            handle
        ),
        2000,
        "opponent has +2 memory ⇒ +1000 × 2 = +2000 DP"
    );
}

#[test]
fn parse_compile_player_memory_selector() {
    let yaml = r#"
card: TEST-MEM
name: Memory DP
kind: digimon
color: [red]
level: 5
cost: 5
dp: 5000
effects:
  - kind: aura
    target: {}
    dp_modifier_fn: { base: 0, per: { player_memory: { of: opponent } }, delta: 1000 }
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("parse");
    let compiled = compile(&spec).expect("compile");
    let CompiledClause::Declarative(CompiledDeclarativeClause::Aura { dp_modifier_fn, .. }) =
        &compiled.effects[0]
    else {
        panic!("expected aura");
    };
    let CompiledFormula::BasePerDelta { per, delta, .. } =
        dp_modifier_fn.as_ref().expect("dp_modifier_fn present")
    else {
        panic!("expected base_per_delta");
    };
    assert_eq!(*delta, 1000);
    assert!(matches!(
        per,
        CompiledPerSelector::PlayerMemory {
            of: CompiledPlayerRef::Opponent
        }
    ));
}

// ===========================================================================
// Leaf 5 — Subtract compound formula
// ===========================================================================

#[test]
fn subtract_formula_evaluates_left_associative() {
    let (mut runner, handle) = memory_runner();
    let top = runner.top_card(handle);
    let ctx = EffectContext::new(&mut runner.game, top, Some(handle), 0);

    // subtract(10, 3) = 7
    let f = CompiledFormula::Subtract(vec![
        CompiledFormula::Literal(10),
        CompiledFormula::Literal(3),
    ]);
    assert_eq!(
        digimon_engine::dsl_cards::formula_eval::evaluate(&f, &ctx, handle),
        7
    );

    // subtract(10, 3, 2) = 5  (left-associative)
    let f3 = CompiledFormula::Subtract(vec![
        CompiledFormula::Literal(10),
        CompiledFormula::Literal(3),
        CompiledFormula::Literal(2),
    ]);
    assert_eq!(
        digimon_engine::dsl_cards::formula_eval::evaluate(&f3, &ctx, handle),
        5
    );

    // subtract() = 0, subtract(4) = 4
    assert_eq!(
        digimon_engine::dsl_cards::formula_eval::evaluate(
            &CompiledFormula::Subtract(vec![]),
            &ctx,
            handle
        ),
        0
    );
    assert_eq!(
        digimon_engine::dsl_cards::formula_eval::evaluate(
            &CompiledFormula::Subtract(vec![CompiledFormula::Literal(4)]),
            &ctx,
            handle
        ),
        4
    );
}

#[test]
fn parse_compile_subtract_formula() {
    let yaml = r#"
card: TEST-SUB
name: Subtract
kind: digimon
color: [black]
level: 4
cost: 4
dp: 4000
effects:
  - kind: aura
    target: {}
    dp_modifier_fn:
      subtract:
        - 5000
        - { base: 0, per: material_count, delta: 1000 }
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("parse");
    let compiled = compile(&spec).expect("compile");
    let CompiledClause::Declarative(CompiledDeclarativeClause::Aura { dp_modifier_fn, .. }) =
        &compiled.effects[0]
    else {
        panic!("expected aura");
    };
    assert!(matches!(
        dp_modifier_fn.as_ref().expect("dp fn"),
        CompiledFormula::Subtract(args) if args.len() == 2
    ));
}

// ===========================================================================
// Leaf 5 — trash_top_security { leave: N }
// ===========================================================================

/// Run a single `trash_top_security { of: opponent, leave: N }` step against a
/// game where the opponent (player 1) holds `opp_security` security cards,
/// then assert how many remain.
fn run_leave_step(opp_security: usize, leave: i32) -> usize {
    let ids: Vec<String> = (0..opp_security).map(|i| format!("SEC{i}")).collect();
    let refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
    let yaml = format!(
        r#"
card: TEST-LEAVE
name: Leave N
kind: option
color: [black]
cost: 0
effects:
  - when: main_from_hand
    process:
      - trash_top_security: {{ of: opponent, leave: {leave} }}
"#
    );
    let steps = compile_steps(&yaml);

    let mut b = DebugRunner::builder().add_card(digimon("SRC", "Source"));
    for i in 0..opp_security {
        b = b.add_card(digimon(&format!("SEC{i}"), &format!("Sec {i}")));
    }
    let mut runner = b.hand(0, &["SRC"]).security(1, &refs).build();
    runner.game.start_game();

    let src = runner.game.players[0].hand[0].handle();
    let mut bindings = Bindings::new();
    let mut ctx = EffectContext::new(&mut runner.game, src, None, 0);
    let _ = run_steps(&steps, &mut ctx, &mut bindings);
    runner.game.player(1).security.len()
}

#[test]
fn trash_top_security_leave_trashes_down_to_n() {
    // 5 → leave 1 → 4 trashed → 1 left (BT21-098's exact shape).
    assert_eq!(run_leave_step(5, 1), 1);
    // 3 → leave 1 → 1 left.
    assert_eq!(run_leave_step(3, 1), 1);
}

#[test]
fn trash_top_security_leave_is_noop_when_already_short() {
    // 1 → leave 1 → nothing trashed (never underflows / over-trashes).
    assert_eq!(run_leave_step(1, 1), 1);
    // 0 → leave 1 → still 0 (clamped, no panic).
    assert_eq!(run_leave_step(0, 1), 0);
    // 2 → leave 3 → nothing trashed (leave exceeds stack).
    assert_eq!(run_leave_step(2, 3), 2);
}
