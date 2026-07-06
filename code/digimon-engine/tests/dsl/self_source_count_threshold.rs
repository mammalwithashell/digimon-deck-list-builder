//! G-DSL-SELF-SOURCE-COUNT-THRESHOLD — `self_source_count` predicate leaf.
//!
//! Compares the count of the effect carrier's OWN digivolution source cards
//! matching `filter` against `value` under `op`. Gates a conditional self-aura.
//!
//! Driver: BT21-006 Tsumemon inherited "[All Turns] This Digimon with 4 or more
//! [Vemmon] digivolution cards gets +3000 DP" — DCGO
//! `card.PermanentOfThisCard().DigivolutionCards.Count(EqualsCardName("Vemmon"))
//! >= 4`.

use digimon_dsl::compiled::{
    CompiledClause, CompiledComparatorOp, CompiledDeclarativeClause, CompiledDpConstraint,
    CompiledPredicate,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::permanent::PermanentHandle;

fn digimon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c
}

// ---------------------------------------------------------------------------
// Parse / compile
// ---------------------------------------------------------------------------

#[test]
fn parse_and_compile_self_source_count_leaf() {
    let yaml = r#"
card: TEST-SELF-SOURCE-COUNT
name: Self Source Count
kind: digimon
color: [black]
level: 4
cost: 4
dp: 4000
effects:
  - kind: aura
    target: {}
    active_when:
      self_source_count:
        filter: { name_is: Vemmon }
        op: gte
        value: 4
    dp_modifier: 3000
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile yaml");

    let CompiledClause::Declarative(CompiledDeclarativeClause::Aura { active_when, .. }) =
        &compiled.effects[0]
    else {
        panic!("expected aura clause");
    };
    let ssc = active_when
        .as_ref()
        .and_then(|p| p.self_source_count.as_ref())
        .expect("active_when must carry self_source_count");
    assert_eq!(ssc.op, CompiledComparatorOp::Gte);
    assert_eq!(ssc.value, CompiledDpConstraint::Literal(4));
    let filter = ssc.filter.as_deref().expect("filter present");
    assert_eq!(filter.name_is.as_deref(), Some("Vemmon"));
}

// ---------------------------------------------------------------------------
// Inline-fixture behavioral proof — BT21-006 shape
// ---------------------------------------------------------------------------

const BT21_006_SHAPE: &str = r#"
card: TEST-TSUMEMON
name: Test Tsumemon
kind: digimon
color: [black]
level: 4
cost: 4
dp: 4000
effects:
  # Self-aura: +3000 DP while the carrier has 4+ [Vemmon] digivolution sources.
  # (BT21-006 prints this as an inherited effect; the `self_source_count` leaf
  # under test reads `ctx.source_permanent`'s own sources either way — this
  # non-inherited self-aura exercises the predicate directly, independent of
  # the orthogonal inherited-vs-carrier plumbing tested elsewhere.)
  - kind: aura
    target: {}
    active_when:
      self_source_count:
        filter: { name_is: Vemmon }
        op: gte
        value: 4
    dp_modifier: 3000
"#;

fn build(vemmon_sources: usize) -> (DebugRunner, PermanentHandle) {
    let mut builder = DebugRunner::builder()
        .from_dsl_yaml(BT21_006_SHAPE)
        .expect("register DSL card");
    // Register Vemmon + a non-Vemmon filler source.
    builder = builder
        .add_card(digimon("VEMMON", "Vemmon"))
        .add_card(digimon("FILLER", "Filler"));

    let mut runner = builder.start();

    // Build the stack: `vemmon_sources` Vemmon sources, then enough filler to
    // reach 4 sources total (so total source count is constant and only the
    // Vemmon count varies), then the TEST-TSUMEMON top card.
    let mut stack: Vec<&str> = Vec::new();
    for _ in 0..vemmon_sources {
        stack.push("VEMMON");
    }
    while stack.len() < 4 {
        stack.push("FILLER");
    }
    stack.push("TEST-TSUMEMON");
    let handle = runner.place_stack(0, &stack);
    (runner, handle)
}

#[test]
fn self_source_count_gte_4_grants_dp_when_met() {
    // 4 Vemmon sources → condition met → +3000.
    let (runner, handle) = build(4);
    assert_eq!(
        runner.game.effective_dp(handle),
        Some(7000),
        "4 [Vemmon] sources ⇒ base 4000 + 3000 aura"
    );
}

#[test]
fn self_source_count_gte_4_no_dp_when_not_met() {
    // 3 Vemmon sources (+1 filler to keep 4 total) → condition NOT met → base only.
    let (runner, handle) = build(3);
    assert_eq!(
        runner.game.effective_dp(handle),
        Some(4000),
        "only 3 [Vemmon] sources ⇒ no aura, base 4000"
    );
}

#[test]
fn self_source_count_leaf_evaluates_directly() {
    // Direct predicate eval — independent of the aura plumbing.
    use digimon_engine::dsl_cards::predicate::{eval_predicate_with_bindings, PredicateSubject};
    use digimon_engine::effect_context::EffectReadContext;

    let (runner, handle) = build(4);
    let top = runner.top_card(handle);
    let rctx = EffectReadContext::new(&runner.game, top, Some(handle), 0);

    let met = CompiledPredicate {
        self_source_count: Some(digimon_dsl::compiled::CompiledSelfSourceCountPredicate {
            filter: Some(Box::new(CompiledPredicate {
                name_is: Some("Vemmon".to_string()),
                ..Default::default()
            })),
            op: CompiledComparatorOp::Gte,
            value: CompiledDpConstraint::Literal(4),
        }),
        ..Default::default()
    };
    // Subject is ignored (no-subject global predicate); pass the carrier.
    assert!(eval_predicate_with_bindings(
        &met,
        &rctx,
        PredicateSubject::Permanent(handle),
        None
    ));

    let unmet = CompiledPredicate {
        self_source_count: Some(digimon_dsl::compiled::CompiledSelfSourceCountPredicate {
            filter: Some(Box::new(CompiledPredicate {
                name_is: Some("Vemmon".to_string()),
                ..Default::default()
            })),
            op: CompiledComparatorOp::Gte,
            value: CompiledDpConstraint::Literal(5),
        }),
        ..Default::default()
    };
    assert!(!eval_predicate_with_bindings(
        &unmet,
        &rctx,
        PredicateSubject::Permanent(handle),
        None
    ));
}
