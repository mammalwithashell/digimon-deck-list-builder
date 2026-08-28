//! Check a scenario's `assert:` block against the projected trace.
//!
//! Moved out of `main.rs` (2026-08-28, MCP task 6) so `exam::run_one` (the
//! MCP probe's core) and the CLI's sim-only path share ONE definition of what
//! an assertion means. Two copies could disagree about, say, whether `5` and
//! `5.0` compare equal, and a probe that says a line is clean by a looser
//! reading than the CLI later enforces is worse than not probing at all.

use crate::exam::backfill::GENERATED_MARKER;
use crate::exam::projection::StateProjection;
use crate::exam::scenario::Scenario;

pub const ASSERTION_KEYS: &[&str] = &[
    "turn",
    "phase",
    "memory",
    "p0.memory",
    "p1.memory",
    "p{0,1}.security",
    "p{0,1}.hand",
    "p{0,1}.trash",
    "p{0,1}.field",
];

/// Check every `assert:` block against the projected trace.
///
/// Returns `(checks_made, failures)`. An unknown key is a **failure**, not a
/// skip: silently ignoring it would let a typo'd assertion report a pass while
/// checking nothing.
pub fn check_assertions(s: &Scenario, projections: &[StateProjection]) -> (u32, Vec<String>) {
    let mut checked = 0u32;
    let mut failures = Vec::new();

    for a in &s.assertions {
        let Some(p) = projections.iter().find(|p| p.step == a.at) else {
            failures.push(format!(
                "at {}: no projected state for that step (the trace has {:?})",
                a.at,
                projections.iter().map(|p| p.step).collect::<Vec<_>>()
            ));
            continue;
        };
        for (key, expected) in &a.that {
            if key == GENERATED_MARKER {
                continue; // provenance metadata, not a projection path
            }
            checked += 1;
            match projected_value(p, key) {
                None => failures.push(format!(
                    "at {}: unknown assertion key `{key}` -- supported: {}",
                    a.at,
                    ASSERTION_KEYS.join(", ")
                )),
                Some(actual) => {
                    if !values_equal(expected, &actual) {
                        failures.push(format!(
                            "at {}: {key} expected {} but our engine has {}",
                            a.at,
                            render_value(expected),
                            render_value(&actual)
                        ));
                    }
                }
            }
        }
    }

    (checked, failures)
}

fn projected_value(p: &StateProjection, key: &str) -> Option<serde_yml::Value> {
    let v = |x: Result<serde_yml::Value, _>| x.ok();
    match key {
        "turn" => v(serde_yml::to_value(p.turn)),
        "phase" => v(serde_yml::to_value(&p.phase)),
        // The projection pins memory to player 0's perspective, so `p1.memory`
        // is its negation rather than a second stored field.
        "memory" | "p0.memory" => v(serde_yml::to_value(p.memory)),
        "p1.memory" => v(serde_yml::to_value(-p.memory)),
        _ => {
            let (seat, field) = key.split_once('.')?;
            let s = match seat {
                "p0" => &p.p0,
                "p1" => &p.p1,
                _ => return None,
            };
            match field {
                "security" => v(serde_yml::to_value(s.security)),
                "hand" => v(serde_yml::to_value(&s.hand)),
                "trash" => v(serde_yml::to_value(&s.trash)),
                "field" => v(serde_yml::to_value(&s.field)),
                _ => None,
            }
        }
    }
}

/// Compare an authored value with a projected one.
///
/// Sequences compare as **multisets**: the projection sorts zones on
/// construction because zone order is representation, not semantics, and an
/// author who wrote a hand in a different order is making the same claim.
/// Numbers compare numerically so `5` and `5.0` are not a divergence.
fn values_equal(expected: &serde_yml::Value, actual: &serde_yml::Value) -> bool {
    use serde_yml::Value;
    match (expected, actual) {
        (Value::Sequence(a), Value::Sequence(b)) => {
            if a.len() != b.len() {
                return false;
            }
            let mut a: Vec<String> = a.iter().map(render_value).collect();
            let mut b: Vec<String> = b.iter().map(render_value).collect();
            a.sort();
            b.sort();
            a == b
        }
        (Value::Number(a), Value::Number(b)) => match (a.as_f64(), b.as_f64()) {
            (Some(x), Some(y)) => x == y,
            _ => a == b,
        },
        _ => expected == actual,
    }
}

fn render_value(v: &serde_yml::Value) -> String {
    serde_yml::to_string(v)
        .unwrap_or_else(|_| format!("{v:?}"))
        .trim_end()
        .replace('\n', " ")
}
