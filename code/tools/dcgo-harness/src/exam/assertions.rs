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
    "p{0,1}.legal_option_plays",
    "p{0,1}.selection_can_decline",
    "p{0,1}.selection_candidates",
    "p{0,1}.movement_observations",
];

/// Check every `assert:` block against the projected trace.
///
/// Returns `(checks_made, failures)`. An unknown key is a **failure**, not a
/// skip: silently ignoring it would let a typo'd assertion report a pass while
/// checking nothing.
pub fn check_assertions(s: &Scenario, projections: &[StateProjection]) -> (u32, Vec<String>) {
    check_scenario_rows(s, &s.assertions, projections)
}

/// Check a Q&A scenario's `expect_ruling:` -- the publisher's answer -- against
/// the projected trace with the SAME checker `assert:` uses, so "ours agrees
/// with the ruling" (the third leg of design D7) means exactly what an
/// assertion pass means. `None` when the scenario carries no ruling.
pub fn check_ruling(s: &Scenario, projections: &[StateProjection]) -> Option<(u32, Vec<String>)> {
    s.expect_ruling
        .as_ref()
        .map(|r| check_scenario_rows(s, &r.assertions, projections))
}

fn check_scenario_rows(s: &Scenario, rows: &[Assertion], projections: &[StateProjection]) -> (u32, Vec<String>) {
    let context: Vec<_> = s.assertions.iter()
        .chain(s.expect_ruling.iter().flat_map(|r| r.assertions.iter()))
        .cloned().collect();
    check_rows_with_context(rows, projections, &context)
}

/// A raw identity is compared only within one engine's trace.
pub(super) fn movement_binding(
    seat: &crate::exam::projection::SeatProjection,
    id: &str,
    owner: u8,
) -> Option<(u64, u32)> {
    let mut found = seat
        .movement_audit
        .iter()
        .filter(|a| a.kind == "public_watch_bound" && a.card_id == id && a.owner == owner);
    let first = found.next()?;
    if found.next().is_some() {
        return None;
    }
    if first.binding_start > seat.movement_observation_at? {
        return None;
    }
    Some((first.native_top, first.binding_start))
}
fn movement_intervals(rows: &[Assertion], projections: &[StateProjection], context: &[Assertion]) -> Vec<String> {
    let mut claims =
        std::collections::BTreeMap::<(usize, String), std::collections::BTreeSet<u32>>::new();
    for row in rows {
        for seat in 0..2 {
            if let Some(serde_yml::Value::Mapping(map)) =
                row.that.get(&format!("p{seat}.movement_observations"))
            {
                for id in map.keys().filter_map(|k| k.as_str()) {
                    claims.entry((seat, id.into())).or_default().insert(row.at);
                }
            }
        }
    }
    // A baseline and its later observation may be authored in different blocks.
    // Import anchors only for watches claimed by this block: ordinary assertions
    // must not inherit unrelated ruling checks or failure counts.
    for row in context {
        for seat in 0..2 {
            if let Some(serde_yml::Value::Mapping(map)) = row.that.get(&format!("p{seat}.movement_observations")) {
                for id in map.keys().filter_map(|k| k.as_str()) {
                    if let Some(anchors) = claims.get_mut(&(seat, id.into())) {
                        anchors.insert(row.at);
                    }
                }
            }
        }
    }
    let mut errors = vec![];
    for ((seat, id), anchors) in claims {
        if anchors.len() < 2 {
            errors.push(format!("p{seat}.{id}: movement history requires an earlier public baseline and later observation"));
            continue;
        }
        let mut binding = None;
        let mut prior_observed = None;
        for at in anchors {
            let value = projections.iter().find(|p| p.step == at).and_then(|p| {
                let observed = if seat == 0 { &p.p0 } else { &p.p1 };
                Some((
                    movement_binding(observed, &id, seat as u8)?,
                    observed.movement_observation_at?,
                ))
            });
            match value {Some((v,observed)) if binding.map_or(true,|old|old==v) && prior_observed.map_or(true,|old|old<observed)=>{binding=Some(v);prior_observed=Some(observed);},_=>errors.push(format!("at {at}: p{seat}.{id} movement binding missing, reset or rebound across baseline"))}
        }
    }
    errors
}
fn check_rows(rows: &[Assertion], projections: &[StateProjection]) -> (u32, Vec<String>) {
    check_rows_with_context(rows, projections, rows)
}

fn check_rows_with_context(rows: &[Assertion], projections: &[StateProjection], context: &[Assertion]) -> (u32, Vec<String>) {
    let mut checked = 0u32;
    let mut failures = movement_intervals(rows, projections, context);

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
            if key == "p0.movement_observations" || key == "p1.movement_observations" {
                let s = if key.starts_with("p0") { &p.p0 } else { &p.p1 };
                let wanted = serde_yml::from_value::<
                    std::collections::BTreeMap<
                        String,
                        digimon_engine::movement_observation::MovementWatch,
                    >,
                >(expected.clone());
                match wanted {
                    Ok(wanted)
                        if !wanted.is_empty()
                            && wanted
                                .iter()
                                .all(|(id, w)| !id.trim().is_empty() && w.is_valid_claim()) =>
                    {
                        for (id, w) in wanted {
                            match s.movement_observations.as_ref().and_then(|o|o.get(&id)) {
                                None => failures.push(format!("at {}: {key}.{id} is unmeasured",a.at)),
                                Some(actual) if actual != &w => failures.push(format!("at {}: {key}.{id} movement history differs: expected {:?}, actual {:?}",a.at,w,actual)),
                                _ => {}
                            }
                        }
                    }
                    _ => {
                        failures.push(format!("at {}: {key} malformed movement watch claim", a.at))
                    }
                }
                continue;
            }
            let missing_measurement = match key.as_str() {
                "p0.legal_option_plays" => p.p0.legal_option_plays.is_none(),
                "p1.legal_option_plays" => p.p1.legal_option_plays.is_none(),
                "p0.selection_candidates" => p.p0.selection_candidates.is_none(),
                "p1.selection_candidates" => p.p1.selection_candidates.is_none(),
                "p0.selection_can_decline" => p.p0.selection_can_decline.is_none(),
                "p1.selection_can_decline" => p.p1.selection_can_decline.is_none(),
                _ => false,
            };
            if missing_measurement {
                failures.push(format!("at {}: {key} is unmeasured; requires the corresponding live decision for that seat", a.at));
                continue;
            }
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
                "selection_can_decline" => v(serde_yml::to_value(s.selection_can_decline)),
                "selection_candidates" => v(serde_yml::to_value(&s.selection_candidates)),
                "movement_observations" => v(serde_yml::to_value(&s.movement_observations)),
                "legal_option_plays" => Some(
                    s.legal_option_plays
                        .as_ref()
                        .map(|ids| serde_yml::to_value(ids).expect("string list"))
                        .unwrap_or(serde_yml::Value::Null),
                ),
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
        (Value::String(id), Value::Mapping(x)) => x
            .get("card_id")
            .and_then(|v| v.as_str())
            .map_or(false, |c| c.eq_ignore_ascii_case(id)),
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
        "SelectTarget"
            | "SelectMaterial"
            | "SelectTrash"
            | "SelectSource"
            | "SelectHand"
            | "SelectReveal"
            | "SelectSecurity"
            | "SelectEffectChoice"
            | "SelectUnion"
            | "SelectPermutation"
            | "SelectBudgeted"
            | "SelectBreedingPermanent"
            | "SelectPlayOrder"
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
        SeatProjection {
            security: 5,
            hand: vec![],
            trash: vec![],
            field: vec![],
            legal_option_plays: None,
            selection_can_decline: None,
            movement_observations: None,
            movement_audit: vec![],
            movement_observation_at: None,
            selection_candidates: None,
        }
    }

    pub(super) fn trace() -> Vec<StateProjection> {
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
    fn movement_baseline_can_span_assert_and_ruling_blocks() {
        use digimon_engine::movement_observation::{MovementAudit, MovementWatch};
        let watch = MovementWatch { version: 1, deletion_dispatches: 0, rule_trashes: vec![] };
        let mut s = scenario(&RULING.replace("MEM", "2"));
        let claim = |at| Assertion { at, that: [("p0.movement_observations".into(), serde_yml::to_value(
            [("X", watch.clone())].into_iter().collect::<std::collections::BTreeMap<_, _>>()
        ).unwrap())].into_iter().collect() };
        s.assertions = vec![claim(0)];
        s.expect_ruling.as_mut().unwrap().assertions = vec![claim(2)];
        let mut rows = trace();
        for row in &mut rows {
            row.p0.movement_observation_at = Some(row.step);
            row.p0.movement_observations = Some([("X".into(), watch.clone())].into_iter().collect());
            row.p0.movement_audit = vec![MovementAudit { binding_start: 0, owner: 0, card_id: "X".into(), native_top: 91, native_lower: vec![], native_linked: vec![], kind: "public_watch_bound".into() }];
        }
        assert_eq!(check_assertions(&s, &rows), (1, vec![]));
        assert_eq!(check_ruling(&s, &rows), Some((1, vec![])));
        rows[2].p0.movement_audit[0].native_top = 92;
        assert!(!check_ruling(&s, &rows).unwrap().1.is_empty(), "cross-block baseline still rejects rebound physical identity");
        s.assertions.clear();
        assert!(!check_ruling(&s, &rows).unwrap().1.is_empty(), "a lone post-event claim is not a baseline");
    }

    #[test]
    fn cross_block_movement_failures_stay_scoped_to_the_claimed_watch() {
        use digimon_engine::movement_observation::{MovementAudit, MovementWatch};
        let watch = MovementWatch { version: 1, deletion_dispatches: 0, rule_trashes: vec![] };
        let claim = |at, id: &str| Assertion { at, that: [("p0.movement_observations".into(), serde_yml::to_value(
            [(id, watch.clone())].into_iter().collect::<std::collections::BTreeMap<_, _>>()
        ).unwrap())].into_iter().collect() };
        let mut s = scenario(&RULING.replace("MEM", "99"));
        s.assertions = vec![claim(0, "X"), claim(2, "X")];
        let mut rows = trace();
        for row in &mut rows {
            row.p0.movement_observation_at = Some(row.step);
            row.p0.movement_observations = Some([("X".into(), watch.clone()), ("Y".into(), watch.clone())].into_iter().collect());
            row.p0.movement_audit = ["X", "Y"].into_iter().enumerate().map(|(i,id)| MovementAudit {
                binding_start: 0, owner: 0, card_id: id.into(), native_top: 91+i as u64,
                native_lower: vec![], native_linked: vec![], kind: "public_watch_bound".into()
            }).collect();
        }
        assert_eq!(check_assertions(&s, &rows), (2, vec![]));
        assert!(!check_ruling(&s, &rows).unwrap().1.is_empty(), "non-movement ruling failure stays separate");
        s.expect_ruling.as_mut().unwrap().assertions = vec![claim(0, "Y"), claim(2, "Y")];
        rows[2].p0.movement_audit[1].native_top = 999;
        assert_eq!(check_assertions(&s, &rows), (2, vec![]));
        assert!(!check_ruling(&s, &rows).unwrap().1.is_empty(), "Y rebound cannot poison X");
        s.assertions = vec![claim(2, "X")];
        assert!(!check_assertions(&s, &rows).1.is_empty(), "Y baseline cannot supply X history");
    }

    #[test]
    fn cross_block_movement_continuity_rejects_missing_reset_and_repeated_anchors() {
        use digimon_engine::movement_observation::{MovementAudit, MovementWatch};
        let watch = MovementWatch { version: 1, deletion_dispatches: 0, rule_trashes: vec![] };
        let claim = |at| Assertion { at, that: [("p0.movement_observations".into(), serde_yml::to_value(
            [("X", watch.clone())].into_iter().collect::<std::collections::BTreeMap<_, _>>()
        ).unwrap())].into_iter().collect() };
        let mut s = scenario(&RULING.replace("MEM", "2"));
        s.assertions = vec![claim(0)];
        s.expect_ruling.as_mut().unwrap().assertions = vec![claim(2)];
        let mut baseline = trace();
        for row in &mut baseline {
            row.p0.movement_observation_at = Some(row.step);
            row.p0.movement_observations = Some([("X".into(), watch.clone())].into_iter().collect());
            row.p0.movement_audit = vec![MovementAudit { binding_start: 0, owner: 0, card_id: "X".into(), native_top: 91, native_lower: vec![], native_linked: vec![], kind: "public_watch_bound".into() }];
        }
        for mode in 0..5 {
            let mut rows = baseline.clone();
            match mode {
                0 => rows[2].p0.movement_audit.clear(),
                1 => rows[2].p0.movement_observation_at = None,
                2 => rows[2].p0.movement_audit[0].binding_start = 2,
                3 => rows[2].p0.movement_observation_at = Some(0),
                _ => rows[2].p0.movement_audit[0].native_top = 92,
            }
            assert!(!check_assertions(&s, &rows).1.is_empty(), "assert side rejects mode {mode}");
            assert!(!check_ruling(&s, &rows).unwrap().1.is_empty(), "ruling side rejects mode {mode}");
        }
    }

    #[test]
    fn projection_equality_excludes_native_movement_audit_only() {
        use digimon_engine::movement_observation::{MovementAudit, MovementWatch};
        let mut a = seat();
        a.movement_observations = Some([("X".into(), MovementWatch { version: 1, deletion_dispatches: 0, rule_trashes: vec![] })].into_iter().collect());
        a.movement_observation_at = Some(12);
        a.movement_audit = vec![MovementAudit { binding_start: 1, owner: 0, card_id: "X".into(), native_top: 91, native_lower: vec![], native_linked: vec![], kind: "public_watch_bound".into() }];
        let mut b = a.clone();
        b.movement_observation_at = Some(19);
        b.movement_audit[0].native_top = 500;
        b.movement_audit[0].binding_start = 7;
        assert_eq!(a, b, "native handles and native decision indices are local audit data");
        b.movement_observations.as_mut().unwrap().get_mut("X").unwrap().deletion_dispatches = 1;
        assert_ne!(a, b, "semantic movement counts must still compare");
    }

    #[test]
    fn no_ruling_means_no_third_leg() {
        assert!(check_ruling(&scenario(""), &trace()).is_none());
    }

    #[test]
    fn the_ruling_is_checked_with_the_assert_checker() {
        let (checked, failures) =
            check_ruling(&scenario(&RULING.replace("MEM", "2")), &trace()).unwrap();
        assert_eq!((checked, failures.len()), (1, 0));

        let (_, failures) = check_ruling(&scenario(&RULING.replace("MEM", "5")), &trace()).unwrap();
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

    const PERMANENT: &str =
        "{card_id: BT21-071, dp: 4000, suspended: false, sources: [EX7-008, ST1-01]}";

    #[test]
    fn an_authored_permanent_claims_only_the_keys_it_names() {
        // Q4577: the encoder wrote {card_id, sources}; the projection also
        // carries dp and suspended. Same permanent, not a contradiction.
        assert!(values_equal(
            &y("{card_id: BT21-071, sources: [EX7-008, ST1-01]}"),
            &y(PERMANENT)
        ));
        assert!(
            !values_equal(&y("{card_id: BT21-071, sources: [EX7-008]}"), &y(PERMANENT)),
            "a named key that differs still contradicts"
        );
        assert!(!values_equal(
            &y("{card_id: BT21-071, dp: 5000}"),
            &y(PERMANENT)
        ));
        assert!(
            !values_equal(&y("{card_id: BT21-071, level: 4}"), &y(PERMANENT)),
            "a key the projection does not carry cannot be claimed"
        );
    }

    #[test]
    fn a_permanent_named_by_card_id_alone_matches_its_entry() {
        // Q6389: `p0.field: [BT25-085, BT25-092]` against two full entries.
        let field = y(
            "[{card_id: BT25-085, dp: 12000, suspended: false, sources: [BT25-082]}, \
                        {card_id: BT25-092, dp: -1, suspended: true, sources: []}]",
        );
        assert!(values_equal(&y("[BT25-085, BT25-092]"), &field));
        assert!(
            values_equal(&y("[BT25-092, BT25-085]"), &field),
            "zone order is representation"
        );
        assert!(
            !values_equal(&y("[BT25-085, BT25-085]"), &field),
            "each claim needs its own entry"
        );
        assert!(
            !values_equal(&y("[BT25-085]"), &field),
            "a shorter list claims a smaller field"
        );
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
        assert!(
            failures[0].contains("not observable while a selection is open")
                && failures[0].contains("SelectPermutation"),
            "{failures:?}"
        );
        // a plain phase mismatch keeps the plain message
        let s2 = scenario("interaction: { id: \"qa:Q2669\", source: qa, kind: positive }\nexpect_ruling:\n  q_id: Q2669\n  assert:\n    - at: 1\n      that: { phase: breeding }\n");
        let (_, f2) = check_ruling(&s2, &t).unwrap();
        assert!(
            f2[0].contains("expected breeding but our engine has main"),
            "{f2:?}"
        );
    }

    #[test]
    fn full_entries_still_compare_as_before() {
        assert!(values_equal(&y(PERMANENT), &y(PERMANENT)));
        assert!(!values_equal(
            &y(PERMANENT),
            &y(&PERMANENT.replace("4000", "5000"))
        ));
        assert!(values_equal(
            &y("[BT1-010, BT1-009]"),
            &y("[BT1-009, BT1-010]")
        ));
        assert!(!values_equal(&y("[BT1-010]"), &y("[BT1-009]")));
    }
}

#[cfg(test)]
mod legal_option_observation_tests {
    use super::*;
    use crate::exam::differ::diff;

    fn row(measurement: &str) -> StateProjection {
        StateProjection::from_sidecar_line(&format!(
            r#"{{"step":0,"turn":1,"phase":"Main","memory":3,"p0":{{"security":5,"hand":["ST1-16"],"trash":[],"field":[]{measurement}}},"p1":{{"security":5,"hand":[],"trash":[],"field":[]}}}}"#
        )).unwrap()
    }

    fn requested(expected: &str) -> Scenario {
        Scenario::from_yaml(&format!(
            "card: ST1-16\nclause: ST1-16#effect#0\nseed: 1\ndecks:\n  p0: {{ rest: x }}\n  p1: {{ rest: x }}\nsteps:\n  - actor: 0\n    do: {{ pass: {{}} }}\nassert:\n  - at: 0\n    that: {{ p0.legal_option_plays: {expected} }}\n"
        )).unwrap()
    }

    #[test]
    fn legal_option_measurement_measured_empty_satisfies_empty_request() {
        let (checks, failures) =
            check_assertions(&requested("[]"), &[row(r#", "legal_option_plays": []"#)]);
        assert_eq!(checks, 1);
        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn legal_option_measurement_missing_is_not_an_empty_list() {
        let (_, failures) = check_assertions(&requested("[]"), &[row("")]);
        assert!(!failures.is_empty());
    }

    #[test]
    fn legal_option_measurement_normalizes_order_but_preserves_duplicates() {
        let measured = row(r#", "legal_option_plays": ["ST1-16", "ST1-13", "ST1-16"]"#);
        let (_, failures) =
            check_assertions(&requested("[ST1-16, ST1-16, ST1-13]"), &[measured.clone()]);
        assert!(failures.is_empty(), "{failures:?}");
        let (_, failures) = check_assertions(&requested("[ST1-16, ST1-13]"), &[measured]);
        assert!(!failures.is_empty());
    }

    #[test]
    fn legal_option_measurement_different_legal_sets_diverge_on_equal_boards() {
        let ours = row(r#", "legal_option_plays": ["ST1-16"]"#);
        let theirs = row(r#", "legal_option_plays": []"#);
        let report = diff(&[ours], &[theirs]);
        assert!(!report.is_clean());
        assert_eq!(report.divergences[0].diffs[0].path, "p0.legal_option_plays");
    }
}

#[cfg(test)]
mod selection_decline_observation_tests {
    use super::*;
    use crate::exam::differ::diff;

    fn row(measurement: &str) -> StateProjection {
        StateProjection::from_sidecar_line(&format!(
            r#"{{"step":0,"turn":1,"phase":"Main","memory":3,"p0":{{"security":5,"hand":[],"trash":[],"field":[]{measurement}}},"p1":{{"security":5,"hand":[],"trash":[],"field":[]}}}}"#
        )).unwrap()
    }
    fn requested(expected: &str) -> Scenario {
        Scenario::from_yaml(&format!("card: EX3-037\nclause: EX3-037#effect#1\nseed: 1\ndecks:\n  p0: {{rest: x}}\n  p1: {{rest: x}}\nsteps:\n  - actor: 0\n    do: {{pass: {{}}}}\nassert:\n  - at: 0\n    that: {{p0.selection_can_decline: {expected}}}\n")).unwrap()
    }
    #[test]
    fn selection_decline_observation_measured_false_proves_mandatory_pick() {
        let (checks, failures) = check_assertions(
            &requested("false"),
            &[row(r#", "selection_can_decline": false"#)],
        );
        assert_eq!(checks, 1);
        assert!(failures.is_empty(), "{failures:?}");
    }
    #[test]
    fn selection_decline_observation_measured_true_proves_optional_pick() {
        let (_, failures) = check_assertions(
            &requested("true"),
            &[row(r#", "selection_can_decline": true"#)],
        );
        assert!(failures.is_empty(), "{failures:?}");
    }
    #[test]
    fn selection_decline_observation_missing_never_satisfies_false_or_null() {
        for expected in ["false", "true", "null"] {
            let (_, failures) = check_assertions(&requested(expected), &[row("")]);
            assert!(!failures.is_empty());
        }
    }
    #[test]
    fn selection_decline_observation_actual_optional_cannot_satisfy_mandatory() {
        let (_, failures) = check_assertions(
            &requested("false"),
            &[row(r#", "selection_can_decline": true"#)],
        );
        assert!(!failures.is_empty());
    }
    #[test]
    fn selection_decline_observation_equal_boards_different_capability_diverge() {
        let report = diff(
            &[row(r#", "selection_can_decline": false"#)],
            &[row(r#", "selection_can_decline": true"#)],
        );
        assert!(!report.is_clean());
        assert_eq!(
            report.divergences[0].diffs[0].path,
            "p0.selection_can_decline"
        );
    }
}

#[cfg(test)]
mod selection_candidate_observation_tests {
    use super::*;
    use crate::exam::differ::diff;

    fn observed(measurement: &str) -> StateProjection {
        StateProjection::from_sidecar_line(&format!(
            r#"{{"step":0,"turn":3,"phase":"SelectHand","memory":1,"p0":{{"security":5,"hand":["ST1-05","ST1-09"],"trash":[],"field":[]{measurement}}},"p1":{{"security":5,"hand":[],"trash":[],"field":[]}}}}"#
        )).unwrap()
    }
    fn ruling(expected: &str) -> Scenario {
        Scenario::from_yaml(&format!("card: LM-027\nclause: LM-027#effect#0\nseed: 1\ninteraction: {{id: 'qa:Q4033', source: qa, kind: positive}}\ndecks:\n  p0: {{rest: x}}\n  p1: {{rest: x}}\nsteps:\n  - actor: 0\n    do: {{pass: {{}}}}\nexpect_ruling:\n  q_id: Q4033\n  assert:\n  - at: 0\n    that: {{p0.selection_candidates: {expected}}}\n")).unwrap()
    }
    #[test]
    fn selection_candidates_exact_observed_legal_set_satisfies_publisher_ruling() {
        let (checks, failures) = check_ruling(
            &ruling("[ST1-05]"),
            &[observed(r#", "selection_candidates": ["ST1-05"]"#)],
        )
        .unwrap();
        assert_eq!(checks, 1);
        assert!(failures.is_empty(), "{failures:?}");
    }
    #[test]
    fn selection_candidates_extra_illegal_choice_contradicts_ruling_with_same_memory_and_board() {
        let (_, failures) = check_ruling(
            &ruling("[ST1-05]"),
            &[observed(
                r#", "selection_candidates": ["ST1-05", "ST1-09"]"#,
            )],
        )
        .unwrap();
        assert!(!failures.is_empty());
        assert!(
            !failures[0].contains("unknown assertion"),
            "the supported observable must catch the actual extra choice: {failures:?}"
        );
    }
    #[test]
    fn selection_candidates_missing_is_not_an_observed_empty_set_or_null() {
        for expected in ["[]", "null", "[ST1-05]"] {
            let (_, failures) = check_ruling(&ruling(expected), &[observed("")]).unwrap();
            assert!(
                failures.iter().any(|s| s.contains("unmeasured")),
                "{failures:?}"
            );
        }
    }
    #[test]
    fn selection_candidates_compare_as_exact_multisets() {
        let row = observed(r#", "selection_candidates": ["ST1-09", "ST1-05", "ST1-05"]"#);
        let (_, failures) =
            check_ruling(&ruling("[ST1-05, ST1-09, ST1-05]"), &[row.clone()]).unwrap();
        assert!(failures.is_empty(), "{failures:?}");
        let (_, failures) = check_ruling(&ruling("[ST1-05, ST1-09]"), &[row]).unwrap();
        assert!(!failures.is_empty());
    }
    #[test]
    fn selection_candidates_equal_board_but_different_legal_choices_diverge() {
        let report = diff(
            &[observed(r#", "selection_candidates": ["ST1-05"]"#)],
            &[observed(
                r#", "selection_candidates": ["ST1-05", "ST1-09"]"#,
            )],
        );
        assert!(!report.is_clean());
        assert_eq!(
            report.divergences[0].diffs[0].path,
            "p0.selection_candidates"
        );
    }

    #[test]
    fn movement_missing_is_not_zero_and_exact_lower_order_is_required() {
        use digimon_engine::movement_observation::{
            MovementAudit, MovementWatch, PublicCard, RuleTrash,
        };
        let watch = MovementWatch {
            version: 1,
            deletion_dispatches: 0,
            rule_trashes: vec![],
        };
        let mut expected = watch.clone();
        expected.rule_trashes.push(RuleTrash {
            lower_sources: vec![
                PublicCard {
                    owner: 0,
                    card_id: "A".into(),
                },
                PublicCard {
                    owner: 0,
                    card_id: "B".into(),
                },
            ],
            linked: vec![],
        });
        let make = |at, value| Assertion {
            at,
            that: [(
                "p0.movement_observations".into(),
                serde_yml::to_value(
                    [("BT9-109", value)]
                        .into_iter()
                        .collect::<std::collections::BTreeMap<_, _>>(),
                )
                .unwrap(),
            )]
            .into_iter()
            .collect(),
        };
        let claims = [make(0, watch.clone()), make(1, expected.clone())];
        let mut rows = super::ruling_tests::trace();
        for row in &mut rows {
            row.p0.movement_observation_at = Some(row.step);
            row.p0.movement_audit = vec![MovementAudit {
                binding_start: 0,
                owner: 0,
                card_id: "BT9-109".into(),
                native_top: 1,
                native_lower: vec![],
                native_linked: vec![],
                kind: "public_watch_bound".into(),
            }];
        }
        rows[0].p0.movement_observations = Some([("BT9-109".into(), watch)].into_iter().collect());
        assert_eq!(
            check_rows(&claims, &rows).1.len(),
            1,
            "missing result is not empty history"
        );
        rows[1].p0.movement_observations =
            Some([("BT9-109".into(), expected)].into_iter().collect());
        assert!(check_rows(&claims, &rows).1.is_empty());
        rows[1]
            .p0
            .movement_observations
            .as_mut()
            .unwrap()
            .get_mut("BT9-109")
            .unwrap()
            .rule_trashes[0]
            .lower_sources
            .reverse();
        assert_eq!(
            check_rows(&claims, &rows).1.len(),
            1,
            "source ordering remains exact after valid interval"
        );
        rows[1]
            .p0
            .movement_observations
            .as_mut()
            .unwrap()
            .get_mut("BT9-109")
            .unwrap()
            .rule_trashes[0]
            .lower_sources
            .reverse();
        rows[1]
            .p0
            .movement_observations
            .as_mut()
            .unwrap()
            .get_mut("BT9-109")
            .unwrap()
            .deletion_dispatches = 1;
        assert_eq!(
            check_rows(&claims, &rows).1.len(),
            1,
            "real deletion contradicts zero"
        );
    }

    #[test]
    fn movement_interval_requires_prior_binding_and_rejects_reset() {
        use digimon_engine::movement_observation::{MovementAudit, MovementWatch};
        let w = MovementWatch {
            version: 1,
            deletion_dispatches: 0,
            rule_trashes: vec![],
        };
        let map: [(&str, MovementWatch); 1] = [("X", w.clone())];
        let value = serde_yml::to_value(
            map.into_iter()
                .collect::<std::collections::BTreeMap<_, _>>(),
        )
        .unwrap();
        let claims: Vec<_> = [0, 2]
            .into_iter()
            .map(|at| Assertion {
                at,
                that: [("p0.movement_observations".into(), value.clone())]
                    .into_iter()
                    .collect(),
            })
            .collect();
        let mut rows = super::ruling_tests::trace();
        for row in &mut rows {
            row.p0.movement_observation_at = Some(row.step);
            row.p0.movement_observations = Some([("X".into(), w.clone())].into_iter().collect());
            row.p0.movement_audit = vec![MovementAudit {
                binding_start: 0,
                owner: 0,
                card_id: "X".into(),
                native_top: 11,
                native_lower: vec![],
                native_linked: vec![],
                kind: "public_watch_bound".into(),
            }];
        }
        assert!(check_rows(&claims, &rows).1.is_empty());
        rows[2].p0.movement_audit[0].binding_start = 2;
        assert!(
            !check_rows(&claims, &rows).1.is_empty(),
            "same physical ID rebound after reset cannot inherit baseline"
        );
        rows[2].p0.movement_audit[0].binding_start = 0;
        rows[2].p0.movement_audit[0].native_top = 12;
        assert!(!check_rows(&claims, &rows).1.is_empty());
        assert!(
            !check_rows(&claims[1..], &rows).1.is_empty(),
            "new post-event watch is not historical zero"
        );
    }

    #[test]
    fn native_observation_anchor_survives_scenario_step_remapping() {
        use digimon_engine::movement_observation::{MovementAudit, MovementWatch};
        let w = MovementWatch {
            version: 1,
            deletion_dispatches: 0,
            rule_trashes: vec![],
        };
        let claim = |at| Assertion {
            at,
            that: [(
                "p0.movement_observations".into(),
                serde_yml::to_value(
                    [("X", w.clone())]
                        .into_iter()
                        .collect::<std::collections::BTreeMap<_, _>>(),
                )
                .unwrap(),
            )]
            .into_iter()
            .collect(),
        };
        let mut rows = super::ruling_tests::trace();
        for row in &mut rows {
            row.p0.movement_observation_at = Some(14 + row.step);
            row.p0.movement_observations = Some([("X".into(), w.clone())].into_iter().collect());
            row.p0.movement_audit = vec![MovementAudit {
                binding_start: 14,
                owner: 0,
                card_id: "X".into(),
                native_top: 91,
                native_lower: vec![],
                native_linked: vec![],
                kind: "public_watch_bound".into(),
            }];
        }
        assert!(check_rows(&[claim(0), claim(2)], &rows).1.is_empty());
        rows[2].p0.movement_observation_at = None;
        assert!(!check_rows(&[claim(0), claim(2)], &rows).1.is_empty());
    }
}
