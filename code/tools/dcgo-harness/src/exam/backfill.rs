//! Writes state the oracle **confirmed** back into a scenario's `assert:`
//! block, so the finding survives after the oracle is gone.
//!
//! A scenario is authored to ask a question; once DCGO has answered it and our
//! engine agreed, the answer has to become a durable guard. That guard is the
//! `assert:` block, which the Unity-free CI job re-checks on every PR — the
//! only half of the exam GitHub can run.
//!
//! # The refusal is the point
//!
//! [`backfill_from_diff`] **refuses** unless the diff was clean. Backfilling a
//! diverged run would take DCGO's disagreement and write it in as our expected
//! value, and the scenario would then pass forever — converting an open finding
//! into a permanent, invisible endorsement of the very behavior under
//! suspicion. A truncated run is refused for the same reason at one remove: it
//! has no divergences either, and "we only got through 2 of 5 steps" must never
//! be recorded as "all 5 agreed".
//!
//! [`backfill`] itself, which sees projections but no report, enforces the half
//! of that it *can* see structurally: the confirmed set must cover every step
//! of the line. It cannot detect a divergence — only the differ knows that —
//! which is exactly why the report-taking entry point exists and why callers
//! should prefer it.
//!
//! # Generated entries are marked, so preservation and idempotency are both
//! mechanical
//!
//! Every generated assertion carries the [`GENERATED_MARKER`] key. Backfill
//! drops every marked assertion and regenerates it, and never touches an
//! unmarked one. That makes "preserve what a human wrote" and "running twice
//! does not accumulate duplicates" decidable by inspection rather than by
//! heuristics over the assertion contents.
//!
//! # Security is a count, never contents
//!
//! The projection models security as a count because the contents are hidden
//! information. Backfill inherits that and asserts only the count: an assertion
//! over security *contents* would encode knowledge no player at the table has,
//! and a scenario that depends on it is checking something the game never
//! showed either engine.

use std::collections::BTreeMap;

use serde_yml::Value;

use crate::exam::differ::DiffReport;
use crate::exam::projection::{SeatProjection, StateProjection};
use crate::exam::scenario::{Assertion, Scenario};

/// Key stamped into every assertion this module generates.
///
/// Leading underscore so it sorts ahead of the real keys in the `BTreeMap`,
/// and so it reads as metadata rather than as a projection path.
pub const GENERATED_MARKER: &str = "_backfilled";

/// Backfill, gated on the differ's verdict. **Prefer this entry point.**
///
/// Refuses a report that is not clean — see the module docs for why a diverged
/// or truncated run must never become an expected value.
pub fn backfill_from_diff(
    scenario_yaml: &str,
    confirmed: &[StateProjection],
    report: &DiffReport,
) -> Result<String, String> {
    if !report.is_clean() {
        let why = if report.divergences.is_empty() {
            "the run was TRUNCATED".to_string()
        } else {
            format!(
                "the run DIVERGED at step {}",
                report
                    .first()
                    .map(|d| d.step.to_string())
                    .unwrap_or_else(|| "?".to_string())
            )
        };
        return Err(format!(
            "refusing to backfill: {why} ({}). Writing an unconfirmed state in as \
             the expected value would make this scenario pass forever and bury \
             the finding.",
            report.denominator()
        ));
    }
    backfill(scenario_yaml, confirmed)
}

/// Write `confirmed` into the scenario's `assert:` block and return the new
/// YAML.
///
/// Idempotent: previously generated assertions are replaced, not appended to.
/// Hand-authored assertions are left exactly as they were.
pub fn backfill(scenario_yaml: &str, confirmed: &[StateProjection]) -> Result<String, String> {
    let mut scenario = Scenario::from_yaml(scenario_yaml)?;
    let steps = scenario.steps.len() as u32;

    if confirmed.is_empty() {
        return Err("refusing to backfill: no confirmed state was supplied, so there \
                    is nothing the oracle actually established"
            .to_string());
    }

    // A row for a step the line does not have cannot be asserted -- and
    // `Scenario::validate` would reject the `at:` we would write for it.
    if let Some(bad) = confirmed.iter().find(|p| p.step > steps) {
        return Err(format!(
            "refusing to backfill: confirmed state names step {} but the line is \
             {steps} steps long",
            bad.step
        ));
    }

    // Two rows for one step would silently let one of them win.
    let mut seen: Vec<u32> = confirmed.iter().map(|p| p.step).collect();
    seen.sort_unstable();
    if seen.windows(2).any(|w| w[0] == w[1]) {
        return Err("refusing to backfill: the confirmed state has two rows for the \
                    same step, so which one is the expected value is undecidable"
            .to_string());
    }

    // Coverage: every step of the line must have been projected. This is the
    // truncation half of "the diff was not clean", and it is the only half
    // this entry point can see without a `DiffReport`.
    let missing: Vec<u32> = (1..=steps).filter(|s| !seen.contains(s)).collect();
    if !missing.is_empty() {
        return Err(format!(
            "refusing to backfill: the confirmed state does not cover step(s) \
             {missing:?} of a {steps}-step line -- a partial run must never be \
             written in as if the whole line agreed"
        ));
    }

    // Drop what we generated last time; keep what a human wrote.
    scenario
        .assertions
        .retain(|a| !a.that.contains_key(GENERATED_MARKER));

    let mut rows: Vec<&StateProjection> = confirmed.iter().collect();
    rows.sort_by_key(|p| p.step);
    for row in rows {
        scenario.assertions.push(assertion_for(row)?);
    }

    let text = serde_yml::to_string(&scenario)
        .map_err(|e| format!("failed to re-serialize the backfilled scenario: {e}"))?;

    // Round-trip guard: a backfilled scenario that no longer parses is worse
    // than no backfill, because the failure would surface later as a missing
    // scenario rather than as this error.
    Scenario::from_yaml(&text).map_err(|e| format!("backfilled scenario no longer parses: {e}"))?;

    Ok(text)
}

/// One generated assertion: the whole projected board at that step.
fn assertion_for(p: &StateProjection) -> Result<Assertion, String> {
    let mut that = BTreeMap::new();
    that.insert(GENERATED_MARKER.to_string(), Value::Bool(true));
    that.insert("turn".to_string(), to_value(p.turn)?);
    that.insert("phase".to_string(), to_value(&p.phase)?);
    that.insert("memory".to_string(), to_value(p.memory)?);
    seat_entries(&mut that, "p0", &p.p0)?;
    seat_entries(&mut that, "p1", &p.p1)?;
    Ok(Assertion { at: p.step, that })
}

fn seat_entries(
    that: &mut BTreeMap<String, Value>,
    seat: &str,
    s: &SeatProjection,
) -> Result<(), String> {
    // A COUNT. Security contents are hidden information -- see the module docs.
    that.insert(format!("{seat}.security"), to_value(s.security)?);
    that.insert(format!("{seat}.hand"), to_value(&s.hand)?);
    that.insert(format!("{seat}.trash"), to_value(&s.trash)?);
    that.insert(format!("{seat}.field"), to_value(&s.field)?);
    Ok(())
}

fn to_value<T: serde::Serialize>(v: T) -> Result<Value, String> {
    serde_yml::to_value(v).map_err(|e| format!("failed to encode an assertion value: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exam::differ::diff;

    const LINE: &str = r#"
card: EX12-035
clause: EX12-035#effect#0
seed: 424242
decks:
  p0: { stack: [ST1-02, EX12-035], rest: st1 }
  p1: { stack: [], rest: st1 }
steps:
  - actor: 0
    do: { pass: {} }
  - actor: 1
    do: { pass: {} }
"#;

    fn row(step: u32, memory: i64) -> StateProjection {
        StateProjection::from_sidecar_line(&format!(
            r#"{{"step":{step},"turn":1,"phase":"Main","memory":{memory},
               "p0":{{"security":5,"hand":["ST1-02"],"trash":[],
                      "field":[{{"card_id":"EX12-035","dp":4000,
                                 "suspended":false,"sources":[]}}]}},
               "p1":{{"security":5,"hand":[],"trash":[],"field":[]}}}}"#
        ))
        .unwrap()
    }

    /// Every step of the line, plus the initial state at step 0.
    fn full_run() -> Vec<StateProjection> {
        vec![row(0, 0), row(1, -3), row(2, 3)]
    }

    fn generated(s: &Scenario) -> Vec<&Assertion> {
        s.assertions
            .iter()
            .filter(|a| a.that.contains_key(GENERATED_MARKER))
            .collect()
    }

    #[test]
    fn backfill_writes_assertions_for_every_step() {
        let out = backfill(LINE, &full_run()).expect("a complete clean run should backfill");
        let s = Scenario::from_yaml(&out).expect("the result must still parse");

        let gen = generated(&s);
        let ats: Vec<u32> = gen.iter().map(|a| a.at).collect();
        assert_eq!(ats, vec![0, 1, 2], "one assertion per projected step");

        // The values are the observed ones, not placeholders.
        let step1 = gen.iter().find(|a| a.at == 1).unwrap();
        assert_eq!(
            step1.that.get("memory").unwrap(),
            &Value::Number((-3).into())
        );
        assert_eq!(
            step1.that.get("phase").unwrap(),
            &Value::String("Main".to_string())
        );
        assert_eq!(
            step1.that.get("p0.security").unwrap(),
            &Value::Number(5u64.into())
        );
        assert!(step1.that.contains_key("p0.field"));
        assert!(step1.that.contains_key("p1.hand"));
    }

    #[test]
    fn backfill_refuses_when_the_diff_was_not_clean() {
        // Backfilling a diverged run would bake DCGO's disagreement in as our
        // expected value and make the scenario pass forever.
        let ours = full_run();
        let mut dcgo = full_run();
        dcgo[1] = row(1, 99);
        let report = diff(&ours, &dcgo);
        assert!(!report.is_clean(), "fixture must actually diverge");

        let err = backfill_from_diff(LINE, &ours, &report).unwrap_err();
        assert!(err.contains("DIVERGED"), "got: {err}");
        assert!(err.contains("step 1"), "got: {err}");

        // A truncated run is refused too: it has no divergences either, and
        // "we got through 1 of 3 steps" must not read as "all 3 agreed".
        let truncated = diff(&ours, &ours[..1]);
        assert!(truncated.divergences.is_empty());
        let err = backfill_from_diff(LINE, &ours, &truncated).unwrap_err();
        assert!(err.contains("TRUNCATED"), "got: {err}");

        // And the report-free entry point still refuses the truncation shape
        // it CAN see on its own: a confirmed set that misses a step.
        let err = backfill(LINE, &ours[..2]).unwrap_err();
        assert!(err.contains('2'), "the missing step must be named: {err}");

        // Nothing at all is not a pass either.
        assert!(backfill(LINE, &[]).is_err());

        // ...and a clean report does go through, so the gate is a gate and not
        // a blanket refusal.
        let clean = diff(&ours, &ours);
        assert!(clean.is_clean());
        assert!(backfill_from_diff(LINE, &ours, &clean).is_ok());
    }

    #[test]
    fn backfill_preserves_hand_authored_assertions_it_did_not_generate() {
        let authored = format!("{LINE}assert:\n  - at: 2\n    that: {{ p0.memory: 3 }}\n");
        let out = backfill(&authored, &full_run()).unwrap();
        let s = Scenario::from_yaml(&out).unwrap();

        let hand: Vec<&Assertion> = s
            .assertions
            .iter()
            .filter(|a| !a.that.contains_key(GENERATED_MARKER))
            .collect();
        assert_eq!(hand.len(), 1, "the hand-authored assertion must survive");
        assert_eq!(hand[0].at, 2);
        assert_eq!(
            hand[0].that.get("p0.memory").unwrap(),
            &Value::Number(3.into()),
            "and must survive UNTOUCHED -- backfill owns only what it marked"
        );
        assert_eq!(generated(&s).len(), 3);
    }

    #[test]
    fn backfill_is_idempotent() {
        // Running it twice must not accumulate duplicate `at:` entries.
        let once = backfill(LINE, &full_run()).unwrap();
        let twice = backfill(&once, &full_run()).unwrap();
        assert_eq!(once, twice, "a second backfill must be a no-op");

        let s = Scenario::from_yaml(&twice).unwrap();
        assert_eq!(generated(&s).len(), 3, "no duplicated generated entries");

        // Idempotent even with a hand-authored assertion in the file, and even
        // when the observed state CHANGED: the second run replaces the
        // generated block rather than appending a second copy of it.
        let authored = format!("{LINE}assert:\n  - at: 2\n    that: {{ p0.memory: 3 }}\n");
        let a1 = backfill(&authored, &full_run()).unwrap();
        let mut changed = full_run();
        changed[2] = row(2, 7);
        let a2 = backfill(&a1, &changed).unwrap();
        let s = Scenario::from_yaml(&a2).unwrap();
        assert_eq!(generated(&s).len(), 3);
        assert_eq!(
            generated(&s)
                .iter()
                .find(|a| a.at == 2)
                .unwrap()
                .that
                .get("memory")
                .unwrap(),
            &Value::Number(7.into())
        );
        assert_eq!(
            s.assertions
                .iter()
                .filter(|a| !a.that.contains_key(GENERATED_MARKER))
                .count(),
            1
        );
    }

    #[test]
    fn backfill_does_not_assert_security_contents() {
        // Security is a COUNT in the projection precisely because contents are
        // hidden information; an assertion over them would encode knowledge no
        // player has.
        let out = backfill(LINE, &full_run()).unwrap();
        let s = Scenario::from_yaml(&out).unwrap();
        for a in generated(&s) {
            for (key, value) in &a.that {
                if !key.contains("security") {
                    continue;
                }
                assert!(
                    key == "p0.security" || key == "p1.security",
                    "the only security key may be the per-seat count, got {key}"
                );
                assert!(
                    value.is_number(),
                    "security must be asserted as a count, got {value:?}"
                );
            }
        }
        // Belt and braces over the raw text: nothing may render a security
        // zone as a list of cards.
        assert!(!out.contains("security_cards"));
        assert!(!out.contains("security:\n"));
    }
}
