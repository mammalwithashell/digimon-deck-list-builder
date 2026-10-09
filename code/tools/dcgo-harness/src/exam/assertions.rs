//! Check a scenario's `assert:` block against the projected trace.
//!
//! Moved out of `main.rs` (2026-08-28, MCP task 6) so `exam::run_one` (the
//! MCP probe's core) and the CLI's sim-only path share ONE definition of what
//! an assertion means. Two copies could disagree about, say, whether `5` and
//! `5.0` compare equal, and a probe that says a line is clean by a looser
//! reading than the CLI later enforces is worse than not probing at all.

use crate::exam::backfill::GENERATED_MARKER;
use crate::exam::projection::StateProjection;
use crate::exam::scenario::{Assertion, Scenario};

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
    check_rows(&s.assertions, projections)
}

/// Check a Q&A scenario's `expect_ruling:` -- the publisher's answer -- against
/// the projected trace with the SAME checker `assert:` uses, so "ours agrees
/// with the ruling" (the third leg of design D7) means exactly what an
/// assertion pass means. `None` when the scenario carries no ruling.
pub fn check_ruling(
    s: &Scenario,
    projections: &[StateProjection],
) -> Option<(u32, Vec<String>)> {
    s.expect_ruling
        .as_ref()
        .map(|r| check_rows(&r.assertions, projections))
}

fn check_rows(rows: &[Assertion], projections: &[StateProjection]) -> (u32, Vec<String>) {
    let mut checked = 0u32;
    let mut failures = Vec::new();

    for a in rows {
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
                        let actual_s = actual.as_str().unwrap_or_default();
                        if key == "phase" && is_selection_pseudo_phase(actual_s) {
                            // The engine replaces the turn phase with the open
                            // selection's kind and keeps no record of the phase it
                            // interrupted, so `phase` is not observable here.
                            // Name that, so the author moves the check to a zone
                            // or a count (or a step with no selection open).
                            failures.push(format!(
                                "at {}: phase is not observable while a selection is open (our engine \
                                 reports `{actual_s}` there, not the turn phase) -- expected {}; assert a \
                                 zone or a count at this step, or the phase at a step with no selection open",
                                a.at,
                                render_value(expected)
                            ));
                            continue;
                        }
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
///
/// An authored mapping is a **claim about the keys it names**: `{card_id,
/// sources}` matches a projected permanent that also carries `dp` and
/// `suspended`, and a permanent named by its card id alone matches the entry
/// with that `card_id`. A ruling's encoder cannot know a DP it was never told
/// and does not decide it; three of the second pilot's first "our engine
/// contradicts the ruling" findings were this shape mismatch, not the rules.
/// Strings compare case-insensitively (`breeding` is the `Breeding` phase).
fn values_equal(expected: &serde_yml::Value, actual: &serde_yml::Value) -> bool {
    use serde_yml::Value;
    match (expected, actual) {
        (Value::Sequence(a), Value::Sequence(b)) => {
            if a.len() != b.len() {
                return false;
            }
            // Each authored element must match a distinct projected one.
            let mut used = vec![false; b.len()];
            'next: for e in a {
                for (i, x) in b.iter().enumerate() {
                    if !used[i] && values_equal(e, x) {
                        used[i] = true;
                        continue 'next;
                    }
                }
                return false;
            }
            true
        }
        (Value::Mapping(e), Value::Mapping(x)) => e
            .iter()
            .all(|(k, ev)| x.get(k).map_or(false, |xv| values_equal(ev, xv))),
        (Value::String(id), Value::Mapping(x)) => {
            x.get("card_id").and_then(|v| v.as_str()).map_or(false, |c| c.eq_ignore_ascii_case(id))
        }
        (Value::String(a), Value::String(b)) => a.eq_ignore_ascii_case(b),
        (Value::Number(a), Value::Number(b)) => match (a.as_f64(), b.as_f64()) {
            (Some(x), Some(y)) => x == y,
            _ => a == b,
        },
        _ => expected == actual,
    }
}

/// The engine's `GamePhase::py_name()` for a pending selection (`phases.rs`
/// `is_selection_phase`): a prompt kind standing in for the turn phase.
fn is_selection_pseudo_phase(name: &str) -> bool {
    matches!(
        name,
        "SelectTarget" | "SelectMaterial" | "SelectTrash" | "SelectSource" | "SelectHand" | "SelectReveal"
            | "SelectSecurity" | "SelectEffectChoice" | "SelectUnion" | "SelectPermutation" | "SelectBudgeted"
            | "SelectBreedingPermanent" | "SelectPlayOrder"
    )
}

fn render_value(v: &serde_yml::Value) -> String {
    serde_yml::to_string(v)
        .unwrap_or_else(|_| format!("{v:?}"))
        .trim_end()
        .replace('\n', " ")
}

#[cfg(test)]
mod ruling_tests {
    use super::*;
    use crate::exam::projection::SeatProjection;

    fn seat() -> SeatProjection {
        SeatProjection { security: 5, hand: vec![], trash: vec![], field: vec![] }
    }

    fn trace() -> Vec<StateProjection> {
        (0..3)
            .map(|step| StateProjection {
                step,
                turn: 1,
                phase: "main".to_string(),
                memory: i64::from(step),
                p0: seat(),
                p1: seat(),
            })
            .collect()
    }

    fn scenario(extra: &str) -> Scenario {
        Scenario::from_yaml(&format!(
            "card: BT7-056\nclause: BT7-056#effect#0\nseed: 1\ndecks:\n  p0: {{ rest: x }}\n  p1: {{ rest: x }}\nsteps:\n  - actor: 0\n    do: {{ pass: {{}} }}\n  - actor: 1\n    do: {{ pass: {{}} }}\n{extra}"
        ))
        .unwrap()
    }

    const RULING: &str = "interaction: { id: \"qa:Q1601\", source: qa, kind: positive }\nexpect_ruling:\n  q_id: Q1601\n  assert:\n    - at: 2\n      that: { p0.memory: MEM }\n";

    #[test]
    fn no_ruling_means_no_third_leg() {
        assert!(check_ruling(&scenario(""), &trace()).is_none());
    }

    #[test]
    fn the_ruling_is_checked_with_the_assert_checker() {
        let (checked, failures) =
            check_ruling(&scenario(&RULING.replace("MEM", "2")), &trace()).unwrap();
        assert_eq!((checked, failures.len()), (1, 0));

        let (_, failures) =
            check_ruling(&scenario(&RULING.replace("MEM", "5")), &trace()).unwrap();
        assert_eq!(failures.len(), 1);
        assert!(failures[0].contains("p0.memory"), "{failures:?}");
    }

    #[test]
    fn the_ruling_does_not_leak_into_the_assert_block() {
        let s = scenario(&RULING.replace("MEM", "5"));
        assert_eq!(check_assertions(&s, &trace()), (0, vec![]));
    }

    fn y(text: &str) -> serde_yml::Value {
        serde_yml::from_str(text).unwrap()
    }

    const PERMANENT: &str = "{card_id: BT21-071, dp: 4000, suspended: false, sources: [EX7-008, ST1-01]}";

    #[test]
    fn an_authored_permanent_claims_only_the_keys_it_names() {
        // Q4577: the encoder wrote {card_id, sources}; the projection also
        // carries dp and suspended. Same permanent, not a contradiction.
        assert!(values_equal(&y("{card_id: BT21-071, sources: [EX7-008, ST1-01]}"), &y(PERMANENT)));
        assert!(!values_equal(&y("{card_id: BT21-071, sources: [EX7-008]}"), &y(PERMANENT)),
                "a named key that differs still contradicts");
        assert!(!values_equal(&y("{card_id: BT21-071, dp: 5000}"), &y(PERMANENT)));
        assert!(!values_equal(&y("{card_id: BT21-071, level: 4}"), &y(PERMANENT)),
                "a key the projection does not carry cannot be claimed");
    }

    #[test]
    fn a_permanent_named_by_card_id_alone_matches_its_entry() {
        // Q6389: `p0.field: [BT25-085, BT25-092]` against two full entries.
        let field = y("[{card_id: BT25-085, dp: 12000, suspended: false, sources: [BT25-082]}, \
                        {card_id: BT25-092, dp: -1, suspended: true, sources: []}]");
        assert!(values_equal(&y("[BT25-085, BT25-092]"), &field));
        assert!(values_equal(&y("[BT25-092, BT25-085]"), &field), "zone order is representation");
        assert!(!values_equal(&y("[BT25-085, BT25-085]"), &field), "each claim needs its own entry");
        assert!(!values_equal(&y("[BT25-085]"), &field), "a shorter list claims a smaller field");
    }

    #[test]
    fn strings_compare_case_insensitively_for_phase_names() {
        assert!(values_equal(&y("breeding"), &y("Breeding")));
        assert!(!values_equal(&y("breeding"), &y("Main")));
    }

    #[test]
    fn a_phase_asserted_while_a_selection_is_open_is_named_unobservable() {
        // Q2669: `phase: breeding` at a step where our engine reports the open
        // trigger-order selection (`SelectPermutation`) instead of Breeding.
        let mut t = trace();
        t[2].phase = "SelectPermutation".to_string();
        let s = scenario("interaction: { id: \"qa:Q2669\", source: qa, kind: positive }\nexpect_ruling:\n  q_id: Q2669\n  assert:\n    - at: 2\n      that: { phase: breeding }\n");
        let (checked, failures) = check_ruling(&s, &t).unwrap();
        assert_eq!(checked, 1);
        assert_eq!(failures.len(), 1);
        assert!(failures[0].contains("not observable while a selection is open")
                && failures[0].contains("SelectPermutation"), "{failures:?}");
        // a plain phase mismatch keeps the plain message
        let s2 = scenario("interaction: { id: \"qa:Q2669\", source: qa, kind: positive }\nexpect_ruling:\n  q_id: Q2669\n  assert:\n    - at: 1\n      that: { phase: breeding }\n");
        let (_, f2) = check_ruling(&s2, &t).unwrap();
        assert!(f2[0].contains("expected breeding but our engine has main"), "{f2:?}");
    }

    #[test]
    fn full_entries_still_compare_as_before() {
        assert!(values_equal(&y(PERMANENT), &y(PERMANENT)));
        assert!(!values_equal(&y(PERMANENT), &y(&PERMANENT.replace("4000", "5000"))));
        assert!(values_equal(&y("[BT1-010, BT1-009]"), &y("[BT1-009, BT1-010]")));
        assert!(!values_equal(&y("[BT1-010]"), &y("[BT1-009]")));
    }
}
