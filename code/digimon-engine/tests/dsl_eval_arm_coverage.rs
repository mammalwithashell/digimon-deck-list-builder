//! DSL eval-arm coverage lint tests.
//!
//! These tests enforce the substrate invariant that every variant of
//! `CompiledPredicate` (a struct with optional fields), `CompiledFormula`,
//! `CompiledStep`, and `CompiledTiming` is consumed by an explicit match
//! arm in its evaluator/executor. They forbid wildcard catch-alls
//! (`_ => false`, `_ => None`, `_ => Ok(())`, `_ => unreachable!()`) in
//! the top-level eval functions, where a silent default would cause a
//! card to fall through with the wrong behavior.
//!
//! Wildcard exemption policy: lints scoped to the listed top-level eval
//! functions only (not inner helpers). Inner helpers MAY use narrow
//! defaults when documented with an inline comment explaining the
//! intent — see the per-test exemption logic for the precise function
//! boundary. This scope was chosen because top-level dispatchers are
//! where silent gaps in DSL vocabulary surface; inner helpers usually
//! operate on a subset of variants by design.
//!
//! Re-classification protocol: when a "DSL eval-arm" tag from the
//! 2026-05-14 audit turns out to be a substrate-edge issue (the engine
//! genuinely cannot represent the behavior), defer the corresponding
//! test with `#[ignore = "BLOCKED: G-… (Phase 2)"]` and file a fresh
//! `qa/dsl-vocab-gaps.md` entry prefixed "Phase 2".
//!
//! See `openspec/changes/phase-1-dsl-pipeline-completion/` for the
//! motivating audit and the four-batch closure plan.

use std::collections::HashSet;

use digimon_dsl::compiled::{
    CompiledFormula, CompiledFormulaDiscriminant, CompiledStepDiscriminant, CompiledTiming,
};
use strum::IntoEnumIterator;

const PREDICATE_SRC: &str = include_str!("../src/dsl_cards/predicate.rs");
const FORMULA_SRC: &str = include_str!("../src/dsl_cards/formula_eval.rs");
const TIMING_MAP_SRC: &str = include_str!("../src/dsl_cards/timing_map.rs");
const COMPILED_SRC: &str = include_str!("../../digimon-dsl/src/compiled.rs");

const STEP_MOD_SRC: &str = include_str!("../src/dsl_cards/step/mod.rs");
const STEP_AS_SELECTING: &str = include_str!("../src/dsl_cards/step/as_selecting_player.rs");
const STEP_COMBAT: &str = include_str!("../src/dsl_cards/step/combat.rs");
const STEP_CONTROL_FLOW: &str = include_str!("../src/dsl_cards/step/control_flow.rs");
// 2026-06-11 (judge-quiz Q29): the BT25-era DigiXros transaction + DNA
// step families were missing from this scan list, producing FALSE-POSITIVE
// "missing arm" failures for AllowDigixrosMaterialZone / AddDigixrosCostDelta /
// PreattachDigixrosMaterial / RegisterDigixrosWildcardForTurn /
// AddDigixrosWildcardToPendingTransaction / MayDnaDigivolveNow — all of
// which have real arms in digixros_transaction.rs / dna_digivolve.rs.
const STEP_DIGIXROS_TRANSACTION: &str =
    include_str!("../src/dsl_cards/step/digixros_transaction.rs");
const STEP_DNA_DIGIVOLVE: &str = include_str!("../src/dsl_cards/step/dna_digivolve.rs");
const STEP_RETURN_RESULTS: &str = include_str!("../src/dsl_cards/step/return_results.rs");
const STEP_DELETION_RESULTS: &str = include_str!("../src/dsl_cards/step/deletion_results.rs");
const STEP_SIMULTANEOUS_PLAY: &str = include_str!("../src/dsl_cards/step/simultaneous_play.rs");
const STEP_SOURCE_PLAY_BATCH: &str = include_str!("../src/dsl_cards/step/source_play_batch.rs");
const STEP_FIELD_SOURCE_RESULTS: &str =
    include_str!("../src/dsl_cards/step/field_source_results.rs");
const STEP_DRAW: &str = include_str!("../src/dsl_cards/step/draw.rs");
const STEP_EFFECTS: &str = include_str!("../src/dsl_cards/step/effects.rs");
const STEP_GRANT_TRIGGERED: &str = include_str!("../src/dsl_cards/step/grant_triggered.rs");
const STEP_ITERATION: &str = include_str!("../src/dsl_cards/step/iteration.rs");
const STEP_LINK_CARDS: &str = include_str!("../src/dsl_cards/step/link_cards.rs");
const STEP_MEMORY: &str = include_str!("../src/dsl_cards/step/memory.rs");
const STEP_MODIFIERS: &str = include_str!("../src/dsl_cards/step/modifiers.rs");
const STEP_PERMANENT_MUTATIONS: &str = include_str!("../src/dsl_cards/step/permanent_mutations.rs");
const STEP_PERMANENT_SCAN: &str = include_str!("../src/dsl_cards/step/permanent_scan.rs");
const STEP_PLAY_DIGIVOLVE: &str = include_str!("../src/dsl_cards/step/play_digivolve.rs");
const STEP_REPLACEMENT_OUTCOME: &str = include_str!("../src/dsl_cards/step/replacement_outcome.rs");
const STEP_SCHEDULE_DELAYED: &str = include_str!("../src/dsl_cards/step/schedule_delayed.rs");
const STEP_SELECTIONS: &str = include_str!("../src/dsl_cards/step/selections.rs");
const STEP_ZONE_MOVES: &str = include_str!("../src/dsl_cards/step/zone_moves.rs");
const STEP_ZONE_CARDS: &str = include_str!("../src/dsl_cards/step/zone_cards.rs");

fn step_corpus() -> String {
    let mut s = String::new();
    for chunk in [
        STEP_MOD_SRC,
        STEP_AS_SELECTING,
        STEP_COMBAT,
        STEP_CONTROL_FLOW,
        STEP_DIGIXROS_TRANSACTION,
        STEP_DNA_DIGIVOLVE,
        STEP_DELETION_RESULTS,
        STEP_RETURN_RESULTS,
        STEP_SOURCE_PLAY_BATCH,
        STEP_SIMULTANEOUS_PLAY,
        STEP_FIELD_SOURCE_RESULTS,
        STEP_DRAW,
        STEP_EFFECTS,
        STEP_GRANT_TRIGGERED,
        STEP_ITERATION,
        STEP_LINK_CARDS,
        STEP_MEMORY,
        STEP_MODIFIERS,
        STEP_PERMANENT_MUTATIONS,
        STEP_PERMANENT_SCAN,
        STEP_PLAY_DIGIVOLVE,
        STEP_REPLACEMENT_OUTCOME,
        STEP_SCHEDULE_DELAYED,
        STEP_SELECTIONS,
        STEP_ZONE_MOVES,
        STEP_ZONE_CARDS,
    ] {
        s.push_str(chunk);
        s.push('\n');
    }
    s
}

/// Extract field names from `pub struct CompiledPredicate { ... }` in
/// `compiled.rs`. Returns the snake-case field identifiers (without the
/// `pub` keyword or the type annotation).
fn predicate_field_names() -> Vec<String> {
    let needle = "pub struct CompiledPredicate {";
    let start = COMPILED_SRC
        .find(needle)
        .expect("compiled.rs must contain `pub struct CompiledPredicate`");
    let body_start = start + needle.len();
    // Find the struct's closing brace. CompiledPredicate is a top-level item,
    // so its `}` sits at column 0 — match `"\n}"` rather than the first `}`,
    // which would wrongly stop at a brace inside a field's doc comment (e.g. the
    // `<metric>: { op: eq, value }` comparator note added by the scalar-unify).
    let body_end = COMPILED_SRC[body_start..]
        .find("\n}")
        .expect("compiled.rs CompiledPredicate body must be terminated by `}`");
    let body = &COMPILED_SRC[body_start..body_start + body_end];
    let mut names = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        let stripped = match line.strip_prefix("pub ") {
            Some(rest) => rest,
            None => continue,
        };
        if let Some(colon) = stripped.find(':') {
            let name = stripped[..colon].trim().to_string();
            if !name.is_empty() {
                names.push(name);
            }
        }
    }
    assert!(
        names.len() > 50,
        "expected CompiledPredicate to have 50+ fields, got {}",
        names.len()
    );
    names
}

/// Extract the body of a top-level function from a Rust source file.
/// Returns `None` if the function is not found. Performs naive
/// brace-matching from the first `{` after `fn <name>(` to its matching
/// `}`. Suitable for the well-formed eval functions in this crate.
fn extract_fn_body<'a>(src: &'a str, fn_name: &str) -> Option<&'a str> {
    let needle = format!("fn {}(", fn_name);
    let fn_start = src.find(&needle)?;
    let body_start = src[fn_start..].find('{')? + fn_start;
    let bytes = src.as_bytes();
    let mut depth: i32 = 0;
    let mut idx = body_start;
    while idx < bytes.len() {
        match bytes[idx] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&src[body_start..=idx]);
                }
            }
            _ => {}
        }
        idx += 1;
    }
    None
}

const FORBIDDEN_WILDCARDS: &[&str] = &[
    "_ => false",
    "_ => None",
    "_ => Ok(())",
    "_ => unreachable!()",
    "_ => unreachable!(",
];

fn assert_no_wildcards(body: &str, fn_name: &str, file_name: &str) {
    let mut hits = Vec::new();
    for needle in FORBIDDEN_WILDCARDS {
        if body.contains(needle) {
            hits.push(*needle);
        }
    }
    assert!(
        hits.is_empty(),
        "{} contains forbidden wildcard catch-alls in `{}`: {:?}",
        file_name,
        fn_name,
        hits
    );
}

/// Keep only direct arms of each `match step` in the dispatcher. Defaults in
/// nested matches over bindings are not defaults over CompiledStep. Preserve
/// the existing forbidden-return policy; do not exempt a named step variant.
fn step_dispatch_arm_surface(body: &str) -> String {
    let mut surface = String::new();
    let mut remaining = body;
    while let Some(start) = remaining.find("match step {") {
        let tail = &remaining[start + "match step {".len()..];
        let mut depth = 0usize;
        let mut end = None;
        for (i, ch) in tail.char_indices() {
            match ch {
                '{' => depth += 1,
                '}' if depth == 0 => {
                    end = Some(i);
                    break;
                }
                '}' => depth -= 1,
                _ if depth == 0 => surface.push(ch),
                _ => {}
            }
        }
        let end = end.expect("match step must have a closing brace");
        surface.push('\n');
        remaining = &tail[end + 1..];
    }
    assert!(
        !surface.is_empty(),
        "dispatcher must contain an explicit match step"
    );
    surface
}

#[test]
fn step_wildcard_lint_ignores_nested_binding_defaults_but_rejects_dispatch_defaults() {
    let nested = r#"match step {
        CompiledStep::PlaceCardsAsBottomSources { target, .. } => {
            match resolve(target) { Some(value) => Some(value), _ => None }
        }
        _ => {}
    }"#;
    assert_no_wildcards(&step_dispatch_arm_surface(nested), "fixture", "fixture");
    for forbidden in FORBIDDEN_WILDCARDS {
        let dispatcher =
            format!("match step {{ CompiledStep::Draw {{ .. }} => {{}}, {forbidden}, }}");
        assert!(
            std::panic::catch_unwind(|| {
                assert_no_wildcards(
                    &step_dispatch_arm_surface(&dispatcher),
                    "fixture",
                    "fixture",
                );
            })
            .is_err(),
            "must reject direct dispatcher default: {forbidden}"
        );
    }
}

#[test]
fn zone_card_dispatch_arms_are_in_the_scanned_corpus() {
    for variant in ["SelectZoneCards", "ReturnTopStackedToDeck"] {
        let marker = format!("CompiledStep::{variant}");
        assert!(STEP_ZONE_CARDS.contains(&marker));
        assert!(step_corpus().contains(&marker));
    }
}

#[test]
fn predicate_variants_have_eval_arms() {
    let fields = predicate_field_names();
    let mut missing = Vec::new();
    for field in &fields {
        // Each field is consumed by `pred.<field>` somewhere in
        // predicate.rs (either as `pred.field`, `let Some(_) = pred.field`,
        // `&pred.field`, etc.). A textual substring match catches all
        // these forms.
        let needle = format!("pred.{}", field);
        if !PREDICATE_SRC.contains(&needle) {
            missing.push(field.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "predicate.rs is missing eval coverage for {} CompiledPredicate field(s): {:?}\n\
         Each missing field is silently ignored by the evaluator. Wire an explicit\n\
         `if let Some(_) = pred.<field>` branch in `eval_predicate_with_bindings`\n\
         (or its scope-appropriate sibling: eval_card_fields, eval_permanent_fields,\n\
         eval_event_fields, eval_replacement_fields, eval_no_subject_fields,\n\
         eval_breeding_permanent_fields, eval_result_bound_fields).",
        missing.len(),
        missing
    );
}

#[test]
fn no_wildcard_catchalls_in_eval_predicate() {
    let body = extract_fn_body(PREDICATE_SRC, "eval_predicate_with_bindings")
        .expect("predicate.rs must contain `eval_predicate_with_bindings`");
    assert_no_wildcards(body, "eval_predicate_with_bindings", "predicate.rs");
}

#[test]
fn formula_variants_have_eval_arms() {
    let mut missing = Vec::new();
    for variant in CompiledFormulaDiscriminant::iter() {
        let name = format!("{:?}", variant);
        // `CompiledFormula::<Variant>` or `Self::<Variant>` should appear
        // in formula_eval.rs. The discriminant `Debug` impl yields the
        // variant name without the type prefix.
        let pat1 = format!("CompiledFormula::{}", name);
        let pat2 = format!("Self::{}", name);
        if !FORMULA_SRC.contains(&pat1) && !FORMULA_SRC.contains(&pat2) {
            missing.push(name);
        }
    }
    assert!(
        missing.is_empty(),
        "formula_eval.rs is missing match arms for {} CompiledFormula variant(s): {:?}",
        missing.len(),
        missing
    );
}

#[test]
fn no_wildcard_catchalls_in_eval_formula() {
    // The top-level public entry points are `evaluate_with_bindings`
    // and `evaluate_read_with_raw_and_bindings`. Both ultimately match
    // over `CompiledFormula` — neither may contain a wildcard catch-all.
    for fn_name in [
        "evaluate_with_bindings",
        "evaluate_read_with_raw_and_bindings",
    ] {
        let body = extract_fn_body(FORMULA_SRC, fn_name).unwrap_or_else(|| {
            panic!("formula_eval.rs must contain `{}`", fn_name);
        });
        assert_no_wildcards(body, fn_name, "formula_eval.rs");
    }
}

#[test]
fn step_variants_have_exec_arms() {
    let corpus = step_corpus();
    let mut missing = Vec::new();
    // Step-dispatcher variants that are explicitly handled inline in
    // `run_step_with_runtime` rather than via a `match` arm in a
    // step/*.rs module. Their textual marker is the inline `matches!`
    // or a prose comment in mod.rs.
    let inline_handled: HashSet<&str> = ["RawRust", "PlaceSelfAsDelayOption"]
        .iter()
        .copied()
        .collect();
    for variant in CompiledStepDiscriminant::iter() {
        let name = format!("{:?}", variant);
        let pat1 = format!("CompiledStep::{}", name);
        let pat2 = format!("Self::{}", name);
        if !corpus.contains(&pat1)
            && !corpus.contains(&pat2)
            && !inline_handled.contains(name.as_str())
        {
            missing.push(name);
        }
    }
    assert!(
        missing.is_empty(),
        "step/*.rs is missing match arms for {} CompiledStep variant(s): {:?}\n\
         Each missing variant means the step is silently no-op'd by the\n\
         dispatcher. Add an arm to the appropriate `try_run` in step/<family>.rs.",
        missing.len(),
        missing
    );
}

#[test]
fn no_wildcard_catchalls_in_step_dispatcher() {
    // The top-level dispatcher combines a try_run chain with explicit step
    // matches. Inspect those direct arms, not nested matches over bindings.
    // The empty default continues the chain; forbidden silent returns do not.
    let body = extract_fn_body(STEP_MOD_SRC, "run_step_with_runtime")
        .expect("step/mod.rs must contain `run_step_with_runtime`");
    assert_no_wildcards(
        &step_dispatch_arm_surface(body),
        "run_step_with_runtime",
        "step/mod.rs",
    );
}

#[test]
fn timing_variants_have_map_arms() {
    let mut missing = Vec::new();
    for variant in CompiledTiming::iter() {
        let name = format!("{:?}", variant);
        let pat = format!("CompiledTiming::{}", name);
        if !TIMING_MAP_SRC.contains(&pat) {
            missing.push(name);
        }
    }
    assert!(
        missing.is_empty(),
        "timing_map.rs is missing map arms for {} CompiledTiming variant(s): {:?}\n\
         Each missing variant returns `None` via wildcard fall-through, silently\n\
         dropping triggered effects with that timing. Add an explicit arm.",
        missing.len(),
        missing
    );
}

// Compile-time witness: prove the formula evaluator exists with its
// expected name so a rename or removal is caught at test-compile time
// rather than going silently undetected.
#[test]
fn formula_evaluator_signature_exists() {
    let _f: fn(
        &CompiledFormula,
        &digimon_engine::effect_context::EffectContext<'_>,
        digimon_engine::permanent::PermanentHandle,
    ) -> i32 = digimon_engine::dsl_cards::formula_eval::evaluate;
}
