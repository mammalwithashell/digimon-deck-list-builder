//! unify-dsl-scalar-and-comparators §1 — canonical `FormulaSpec` scalars.
//!
//! Every magnitude field (memory deltas, aura `dp_modifier`/`security_attack`,
//! `cost_reduction.amount`, `de_digivolve.amount`, `add_modifier`/`add_dp_modifier`
//! `value`, `cost_delta.reduce`) accepts EITHER a bare int OR a formula through
//! one `FormulaSpec` type. A bare int keeps the integer fast-path in the
//! compiled IR; any other formula compiles to the formula variant — so the
//! compiled output is byte-identical to the pre-unification `_fn`-twin encoding.
//! The retired `_fn`/`{ formula: }` spellings survive only as deserialize
//! aliases (covered by `parse_zone_movement_steps.rs` /
//! `parse_modifier_formula_value.rs` / the in-tree card corpus).

use digimon_dsl::compile::compile;
use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledDpConstraint, CompiledModifierValue,
    CompiledPredicate, CompiledStep,
};
use digimon_dsl::formula::FormulaSpec;
use digimon_dsl::spec::CardSpec;

fn parse(yaml: &str) -> CardSpec {
    serde_yml::from_str(yaml).expect("YAML parse")
}

fn first_triggered_step(spec: &CardSpec) -> CompiledStep {
    let compiled = compile(spec).expect("compile");
    match &compiled.effects[0] {
        CompiledClause::Triggered(t) => t.process[0].clone(),
        other => panic!("expected triggered clause, got {other:?}"),
    }
}

fn first_declarative(spec: &CardSpec) -> CompiledDeclarativeClause {
    let compiled = compile(spec).expect("compile");
    match &compiled.effects[0] {
        CompiledClause::Declarative(d) => d.clone(),
        other => panic!("expected declarative clause, got {other:?}"),
    }
}

// ── §1.1 — FormulaSpec deserializes a bare int to Literal ──────────────

#[test]
fn bare_int_deserializes_to_formula_literal() {
    let f: FormulaSpec = serde_yml::from_str("7").unwrap();
    assert_eq!(f, FormulaSpec::Literal(7));
    // Round-trip: Literal serializes back to a bare scalar.
    let s = serde_yml::to_string(&f).unwrap();
    let back: FormulaSpec = serde_yml::from_str(&s).unwrap();
    assert_eq!(back, FormulaSpec::Literal(7));
}

// ── §1.2 — memory verbs take a FormulaSpec directly ───────────────────

const MEMORY_HEADER: &str = r#"
card: X-1
name: Test
kind: option
color: [red]
cost: 0
effects:
  - when: main_from_hand
    process:
"#;

#[test]
fn gain_memory_literal_keeps_integer_fast_path() {
    let yaml = format!("{MEMORY_HEADER}      - gain_memory: 2\n");
    assert!(matches!(
        first_triggered_step(&parse(&yaml)),
        CompiledStep::GainMemory(2)
    ));
}

#[test]
fn gain_memory_accepts_a_formula_without_the_fn_twin() {
    // The canonical unified form: a formula directly in `gain_memory`, no
    // `gain_memory_fn` verb and no `{ formula: }` wrapper.
    let yaml =
        format!("{MEMORY_HEADER}      - gain_memory: {{ base: 0, per: ally_count, delta: 1 }}\n");
    assert!(matches!(
        first_triggered_step(&parse(&yaml)),
        CompiledStep::GainMemoryFn { .. }
    ));
}

#[test]
fn lose_memory_accepts_literal_and_formula() {
    let lit = format!("{MEMORY_HEADER}      - lose_memory: 1\n");
    assert!(matches!(
        first_triggered_step(&parse(&lit)),
        CompiledStep::LoseMemory(1)
    ));
    let formula =
        format!("{MEMORY_HEADER}      - lose_memory: {{ base: 0, per: ally_count, delta: 1 }}\n");
    assert!(matches!(
        first_triggered_step(&parse(&formula)),
        CompiledStep::LoseMemoryFn { .. }
    ));
}

// ── §1.3 — aura dp_modifier / cost_reduction.amount take a FormulaSpec ─

#[test]
fn aura_dp_modifier_literal_and_formula_route_to_split_compiled_fields() {
    let lit = parse(
        r#"
card: BT5-093
name: Tai & Matt
kind: tamer
color: [red]
cost: 4
effects:
  - kind: aura
    active_when: { your_turn: true }
    target: { of: you, zone: [battle_area], name_contains: Omnimon }
    dp_modifier: 1000
"#,
    );
    match first_declarative(&lit) {
        CompiledDeclarativeClause::Aura {
            dp_modifier,
            dp_modifier_fn,
            ..
        } => {
            assert_eq!(dp_modifier, Some(1000));
            assert!(dp_modifier_fn.is_none());
        }
        other => panic!("expected Aura, got {other:?}"),
    }

    // Formula directly in dp_modifier — the canonical replacement for the
    // retired `dp_modifier_fn` twin field.
    let formula = parse(
        r#"
card: X-DYNDP
name: Dynamic DP
kind: digimon
level: 6
color: [red]
cost: 8
dp: 9000
effects:
  - kind: aura
    target: {}
    dp_modifier: { base: 0, per: material_count, delta: 1000 }
"#,
    );
    match first_declarative(&formula) {
        CompiledDeclarativeClause::Aura {
            dp_modifier,
            dp_modifier_fn,
            ..
        } => {
            assert!(dp_modifier.is_none());
            assert!(dp_modifier_fn.is_some());
        }
        other => panic!("expected Aura, got {other:?}"),
    }
}

#[test]
fn cost_reduction_amount_literal_and_formula() {
    let formula = parse(
        r#"
card: BT13-007
name: King Drasil clone
kind: digimon
level: 7
color: [yellow]
cost: 15
dp: 15000
effects:
  - kind: cost_reduction
    when_playing_this: true
    amount: { base: 4, per: material_count, delta: 1 }
"#,
    );
    match first_declarative(&formula) {
        CompiledDeclarativeClause::CostReduction {
            amount, amount_fn, ..
        } => {
            assert!(amount.is_none());
            assert!(amount_fn.is_some());
        }
        other => panic!("expected CostReduction, got {other:?}"),
    }
}

// ── §1.5 — modifier value takes a FormulaSpec directly (no { formula: }) ─

#[test]
fn add_dp_modifier_value_formula_direct_and_literal() {
    let lit = parse(
        r#"
card: X-2
name: Test
kind: digimon
level: 6
color: [red]
cost: 8
dp: 9000
effects:
  - when: when_attacking
    process:
      - add_dp_modifier: { target: self, value: 3000, expiry: end_of_attack }
"#,
    );
    match first_triggered_step(&lit) {
        CompiledStep::AddDpModifier { value, .. } => {
            assert_eq!(value, CompiledModifierValue::Literal(3000))
        }
        other => panic!("expected AddDpModifier, got {other:?}"),
    }

    // Canonical: a formula directly in `value`, NOT wrapped in `{ formula: }`.
    let formula = parse(
        r#"
card: X-3
name: Test
kind: digimon
level: 6
color: [red]
cost: 8
dp: 9000
effects:
  - when: when_attacking
    process:
      - add_dp_modifier:
          target: self
          value: { base: 0, per: stack_size, delta: 1000 }
          expiry: end_of_attack
"#,
    );
    match first_triggered_step(&formula) {
        CompiledStep::AddDpModifier { value, .. } => {
            assert!(matches!(value, CompiledModifierValue::Formula(_)))
        }
        other => panic!("expected AddDpModifier, got {other:?}"),
    }
}

// ── §2 — canonical uniform comparator (dp), folded byte-identically ───

fn dp_aura(dp_clause: &str) -> CardSpec {
    parse(&format!(
        r#"
card: X-DP
name: DP Test
kind: digimon
level: 6
color: [red]
cost: 8
dp: 9000
effects:
  - kind: aura
    target: {{ of: opponent, zone: [battle_area], {dp_clause} }}
    dp_modifier: -1000
"#
    ))
}

fn aura_target(spec: &CardSpec) -> CompiledPredicate {
    match first_declarative(spec) {
        CompiledDeclarativeClause::Aura { target, .. } => target,
        other => panic!("expected Aura, got {other:?}"),
    }
}

#[test]
fn canonical_dp_comparator_matches_legacy_flat_key() {
    // The uniform `dp: { op, value }` lowers to the SAME compiled field as the
    // legacy `dp_lte:` flat key — byte-identical compiled output (§2.5).
    let legacy = aura_target(&dp_aura("dp_lte: 5000"));
    let canonical = aura_target(&dp_aura("dp: { op: lte, value: 5000 }"));
    assert_eq!(legacy.dp_lte, canonical.dp_lte);
    assert_eq!(canonical.dp_lte, Some(CompiledDpConstraint::Literal(5000)));
    assert!(canonical.dp_gte.is_none());
    assert!(canonical.dp_eq.is_none());
}

#[test]
fn canonical_dp_comparator_supports_range_eq_and_formula() {
    // A list expresses a range (gte AND lte together).
    let range = aura_target(&dp_aura(
        "dp: [{ op: gte, value: 3000 }, { op: lte, value: 8000 }]",
    ));
    assert_eq!(range.dp_gte, Some(CompiledDpConstraint::Literal(3000)));
    assert_eq!(range.dp_lte, Some(CompiledDpConstraint::Literal(8000)));

    // `eq` is available through the uniform comparator (no bespoke key).
    let eq = aura_target(&dp_aura("dp: { op: eq, value: 4000 }"));
    assert_eq!(eq.dp_eq, Some(CompiledDpConstraint::Literal(4000)));

    // A formula threshold resolves through CompiledDpConstraint::Formula
    // (read-safe at eval — required for mask + future search contexts).
    let formula = aura_target(&dp_aura("dp: { op: lte, value: { source_dp: {} } }"));
    assert!(matches!(
        formula.dp_lte,
        Some(CompiledDpConstraint::Formula(_))
    ));
}

#[test]
fn canonical_comparator_completes_missing_eq_for_identity_metrics() {
    // play_cost / stack_size / materials_count / security_count had no legacy
    // `_eq` key; the uniform comparator supplies it via a NEW compiled `_eq`
    // field (§2.4). `dp_aura` inserts the clause into the aura's target predicate.
    let pc = aura_target(&dp_aura("play_cost: { op: eq, value: 3 }"));
    assert_eq!(pc.play_cost_eq, Some(CompiledDpConstraint::Literal(3)));

    let ss = aura_target(&dp_aura("stack_size: { op: eq, value: 2 }"));
    assert_eq!(ss.stack_size_eq, Some(CompiledDpConstraint::Literal(2)));

    let sc = aura_target(&dp_aura("security_count: { op: eq, value: 0 }"));
    assert_eq!(sc.security_count_eq, Some(CompiledDpConstraint::Literal(0)));

    // Non-eq ops still route to their existing compiled fields; eq stays None.
    let mc = aura_target(&dp_aura("materials_count: { op: gte, value: 1 }"));
    assert_eq!(
        mc.materials_count_gte,
        Some(CompiledDpConstraint::Literal(1))
    );
    assert!(mc.materials_count_eq.is_none());

    // Legacy flat key and the canonical lte lower to the IDENTICAL compiled field.
    let legacy = aura_target(&dp_aura("play_cost_lte: 5"));
    let canon = aura_target(&dp_aura("play_cost: { op: lte, value: 5 }"));
    assert_eq!(legacy.play_cost_lte, canon.play_cost_lte);
    assert_eq!(canon.play_cost_lte, Some(CompiledDpConstraint::Literal(5)));
}

#[test]
fn canonical_event_target_dp_matches_legacy() {
    let legacy = aura_target(&dp_aura("event_target_dp_gte: 6000"));
    let canon = aura_target(&dp_aura("event_target_dp: { op: gte, value: 6000 }"));
    assert_eq!(legacy.event_target_dp_gte, canon.event_target_dp_gte);
    assert_eq!(
        canon.event_target_dp_gte,
        Some(CompiledDpConstraint::Literal(6000))
    );
}

#[test]
fn canonical_level_literal_eq_and_formula_bounds() {
    // level `eq` is literal-only (compiled `level_eq: u8`); `lte`/`gte` are
    // formula-capable (CompiledDpConstraint).
    let eq = aura_target(&dp_aura("level: { op: eq, value: 6 }"));
    assert_eq!(eq.level_eq, Some(6));

    let legacy = aura_target(&dp_aura("level_gte: 5"));
    let canon = aura_target(&dp_aura("level: { op: gte, value: 5 }"));
    assert_eq!(legacy.level_gte, canon.level_gte);
    assert_eq!(canon.level_gte, Some(CompiledDpConstraint::Literal(5)));

    let etl = aura_target(&dp_aura("event_target_level: { op: lte, value: 4 }"));
    assert_eq!(
        etl.event_target_level_lte,
        Some(CompiledDpConstraint::Literal(4))
    );
}

#[test]
fn canonical_level_eq_rejects_formula() {
    // A formula `eq` on level has no `u8` compiled landing → compile error
    // (the documented level gap, hard-errored rather than silently dropped).
    let spec = dp_aura("level: { op: eq, value: { source_dp: {} } }");
    assert!(
        compile(&spec).is_err(),
        "a formula-valued level eq must be rejected"
    );
}
