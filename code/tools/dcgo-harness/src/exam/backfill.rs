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

use crate::exam::differ::{phase_is_compared, DiffReport};
use crate::exam::projection::{SeatProjection, StateProjection, StepPairing};
use crate::exam::scenario::{Assertion, Scenario};

/// Key stamped into every assertion this module generates.
///
/// Leading underscore so it sorts ahead of the real keys in the `BTreeMap`,
/// and so it reads as metadata rather than as a projection path.
pub const GENERATED_MARKER: &str = "_backfilled";

/// Backfill, gated on the differ's verdict. **Prefer this entry point.**
///
/// Refuses a report that is not clean -- see the module docs for why a diverged
/// or truncated run must never become an expected value.
///
/// Writes ONLY the rows the differ paired (`pairing.pairs`): a step that put
/// nothing on the DCGO wire, or that only DCGO decides, was never observed by
/// the oracle and gets no assertion, while the steps around it still do.
/// `phase` is left out of a row whose phase the differ did not compare
/// ([`phase_is_compared`]). `ours` is indexed by scenario step and may carry
/// the trailing post-final-step row; no pairing ever names it.
pub fn backfill_from_diff(
    scenario_yaml: &str,
    ours: &[StateProjection],
    dcgo: &[StateProjection],
    pairing: &StepPairing,
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
    // Honesty rule: an `_backfilled` assertion may encode only state DCGO
    // actually observed -- exactly the paired rows, nothing else.
    let mut observed = Vec::with_capacity(pairing.pairs.len());
    for (oi, di) in &pairing.pairs {
        let (Some(o), Some(d)) = (ours.get(*oi), dcgo.get(*di)) else {
            return Err(format!(
                "refusing to backfill: the pairing names our row {oi} and DCGO row {di} \
                 but only {} and {} row(s) were supplied, so the rows do not match \
                 what the oracle compared.",
                ours.len(),
                dcgo.len()
            ));
        };
        observed.push(Observed { row: o, with_phase: phase_is_compared(&o.phase, &d.phase) });
    }
    if observed.len() != report.compared_steps as usize {
        return Err(format!(
            "refusing to backfill: the report compared {} step(s) but the pairing names \
             {}, so the rows do not match what the oracle compared.",
            report.compared_steps,
            observed.len()
        ));
    }
    write_assertions(scenario_yaml, &observed)
}

/// One oracle-observed row to write, and whether its `phase` was compared.
struct Observed<'a> {
    row: &'a StateProjection,
    with_phase: bool,
}

/// Write `confirmed` into the scenario's `assert:` block and return the new
/// YAML.
///
/// `confirmed` must be exactly the oracle-observed decision points: one row
/// for each step `0..steps` (the position before each scripted step). A row at
/// step `steps` or later is the trailing post-final-step state, which DCGO's
/// pre-decision trace never observes, so it is refused rather than written.
///
/// Idempotent: previously generated assertions are replaced, not appended to.
/// Everything else in the scenario text -- comments, `decks:`, `steps:`, and
/// hand-authored assertions -- is preserved byte-for-byte: only the generated
/// assertion entries are spliced.
pub fn backfill(scenario_yaml: &str, confirmed: &[StateProjection]) -> Result<String, String> {
    let scenario = Scenario::from_yaml(scenario_yaml)?;
    let steps = scenario.steps.len() as u32;

    // Coverage: every decision point of the line must have been projected.
    // This is the truncation half of "the diff was not clean", and it is the
    // only half this entry point can see without a `DiffReport`. Checked
    // after the shape refusals below would have fired, so an empty set or a
    // trailing row reports its own, more specific reason.
    let in_range = confirmed.iter().all(|p| p.step < steps);
    let missing: Vec<u32> = (0..steps)
        .filter(|s| !confirmed.iter().any(|p| p.step == *s))
        .collect();
    if !confirmed.is_empty() && in_range && !missing.is_empty() {
        return Err(format!(
            "refusing to backfill: the confirmed state does not cover step(s) \
             {missing:?} of a {steps}-step line -- a partial run must never be \
             written in as if the whole line agreed"
        ));
    }
    let rows: Vec<Observed> = confirmed
        .iter()
        .map(|row| Observed { row, with_phase: true })
        .collect();
    write_assertions(scenario_yaml, &rows)
}

/// The shared tail of both entry points: refuse rows no oracle row covers,
/// then splice one generated assertion per row.
fn write_assertions(scenario_yaml: &str, confirmed: &[Observed]) -> Result<String, String> {
    let scenario = Scenario::from_yaml(scenario_yaml)?;
    let steps = scenario.steps.len() as u32;

    if confirmed.is_empty() {
        return Err("refusing to backfill: no confirmed state was supplied, so there \
                    is nothing the oracle actually established"
            .to_string());
    }

    // A row at or past `steps` is the trailing state no oracle row covers.
    if let Some(bad) = confirmed.iter().find(|p| p.row.step >= steps) {
        return Err(format!(
            "refusing to backfill: confirmed state names step {} but the oracle only \
             observes the {steps} decision point(s) 0..{steps}; the post-final-step \
             state was never compared against DCGO",
            bad.row.step
        ));
    }

    // Two rows for one step would silently let one of them win.
    let mut seen: Vec<u32> = confirmed.iter().map(|p| p.row.step).collect();
    seen.sort_unstable();
    if seen.windows(2).any(|w| w[0] == w[1]) {
        return Err("refusing to backfill: the confirmed state has two rows for the \
                    same step, so which one is the expected value is undecidable"
            .to_string());
    }

    let mut rows: Vec<&Observed> = confirmed.iter().collect();
    rows.sort_by_key(|p| p.row.step);
    let mut generated = Vec::with_capacity(rows.len());
    for o in rows {
        generated.push(assertion_for(o.row, o.with_phase)?);
    }

    let text = splice_assertions(scenario_yaml, &generated)?;

    // Round-trip guard: a backfilled scenario that no longer parses is worse
    // than no backfill, because the failure would surface later as a missing
    // scenario rather than as this error.
    Scenario::from_yaml(&text).map_err(|e| format!("backfilled scenario no longer parses: {e}"))?;

    Ok(text)
}

fn is_blank_or_comment(line: &str) -> bool {
    let t = line.trim();
    t.is_empty() || t.starts_with('#')
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

/// Replace only the generated assertion entries inside the original text.
///
/// The top-level `assert:` section runs from its key line to the next
/// top-level key (a column-0 line that is not a `-` item, comment or blank) or
/// EOF. Hand-authored entries stay as their original text; entries carrying
/// [`GENERATED_MARKER`] are dropped and the freshly serialized ones appended
/// after them. Absent a section, one is appended at EOF.
fn splice_assertions(
    original: &str,
    generated: &[Assertion],
) -> Result<String, String> {
    let crlf = original.contains("\r\n");
    let lines: Vec<&str> = original.split_inclusive('\n').collect();
    let strip = |l: &str| l.trim_end_matches(['\r', '\n']).to_string();

    let new_block = |indent: usize| -> Result<String, String> {
        let body = serde_yml::to_string(&generated.to_vec())
            .map_err(|e| format!("failed to serialize the generated assertions: {e}"))?;
        let pad = " ".repeat(indent);
        let mut out = String::new();
        for l in body.lines() {
            if l.is_empty() {
                out.push('\n');
            } else {
                out.push_str(&pad);
                out.push_str(l);
                out.push_str(if crlf { "\r\n" } else { "\n" });
            }
        }
        Ok(out)
    };

    let key_idx = lines.iter().position(|l| {
        let s = strip(l);
        s == "assert:" || (s.starts_with("assert:") && s["assert:".len()..].trim_start().starts_with('#'))
    });
    let any_assert_key = lines.iter().any(|l| l.starts_with("assert:"));

    let Some(key_idx) = key_idx else {
        if any_assert_key {
            // Flow style (`assert: [...]`) or a value on the key line: there is
            // no item text to splice into. Refuse rather than rewrite the file.
            return Err("refusing to backfill: `assert:` carries an inline value; \
                        rewrite it as a block list so the entries can be spliced \
                        without re-serializing the scenario"
                .to_string());
        }
        let mut out = original.to_string();
        if !out.is_empty() && !out.ends_with('\n') {
            out.push_str(if crlf { "\r\n" } else { "\n" });
        }
        out.push_str(if crlf { "assert:\r\n" } else { "assert:\n" });
        out.push_str(&new_block(0)?);
        return Ok(out);
    };

    // End of the section: next column-0 key line, then walk back over trailing
    // blank/comment lines, which belong to whatever follows.
    let mut end = lines.len();
    for (i, l) in lines.iter().enumerate().skip(key_idx + 1) {
        let s = strip(l);
        if !s.is_empty() && !s.starts_with(' ') && !s.starts_with('#') && !s.starts_with('-') {
            end = i;
            break;
        }
    }
    while end > key_idx + 1 && is_blank_or_comment(&strip(lines[end - 1])) {
        end -= 1;
    }

    let body = &lines[key_idx + 1..end];
    let item_indent = body
        .iter()
        .map(|l| strip(l))
        .find(|s| !is_blank_or_comment(s) && s.trim_start().starts_with('-'))
        .map(|s| indent_of(&s))
        .unwrap_or(2);
    let is_item_start = |l: &str| {
        let s = strip(l);
        indent_of(&s) == item_indent && (s.trim_start() == "-" || s.trim_start().starts_with("- "))
    };

    // Group into items; blank/comment lines directly before an item start join
    // that item, so a human's comment above their assertion stays with it.
    let mut groups: Vec<Vec<&str>> = Vec::new();
    let mut pending: Vec<&str> = Vec::new();
    for l in body {
        if is_item_start(l) {
            let mut g = std::mem::take(&mut pending);
            g.push(l);
            groups.push(g);
        } else if is_blank_or_comment(&strip(l)) {
            pending.push(l);
        } else {
            // Continuation of the current item (preceded by any pending lines).
            match groups.last_mut() {
                Some(g) => {
                    g.append(&mut pending);
                    g.push(l);
                }
                None => pending.push(l),
            }
        }
    }
    // Lines before the first item that are not comments stay verbatim.
    let preamble = pending;

    let mut out = String::new();
    for l in &lines[..=key_idx] {
        out.push_str(l);
    }
    // The key line must end in a newline before anything is appended.
    if !out.ends_with('\n') {
        out.push_str(if crlf { "\r\n" } else { "\n" });
    }
    for l in preamble {
        out.push_str(l);
    }
    for g in &groups {
        let text: String = g.concat();
        let is_generated = {
            let parsed: Vec<Assertion> = serde_yml::from_str(&text)
                .map_err(|e| format!("could not read an existing assertion entry: {e}"))?;
            parsed.iter().any(|a| a.that.contains_key(GENERATED_MARKER))
        };
        if !is_generated {
            out.push_str(&text);
            if !text.ends_with('\n') {
                out.push_str(if crlf { "\r\n" } else { "\n" });
            }
        }
    }
    out.push_str(&new_block(item_indent)?);
    for l in &lines[end..] {
        out.push_str(l);
    }
    Ok(out)
}

/// One generated assertion: the whole projected board at that step.
/// `with_phase: false` leaves `phase` out -- the differ did not compare it.
fn assertion_for(p: &StateProjection, with_phase: bool) -> Result<Assertion, String> {
    let mut that = BTreeMap::new();
    that.insert(GENERATED_MARKER.to_string(), Value::Bool(true));
    that.insert("turn".to_string(), to_value(p.turn)?);
    if with_phase {
        that.insert("phase".to_string(), to_value(&p.phase)?);
    }
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
    use crate::exam::projection::{pair_by_wire_rows, StepPairing};

    /// One DCGO row per step: the 1:1 pairing a plain line produces.
    fn one_to_one(n: usize) -> StepPairing {
        pair_by_wire_rows(&vec![1; n], n)
    }

    fn with_phase(mut p: StateProjection, phase: &str) -> StateProjection {
        p.phase = phase.to_string();
        p
    }

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

    /// The oracle-observed decision points of the 2-step line: the position
    /// before each step (steps 0 and 1).
    fn compared_rows() -> Vec<StateProjection> {
        vec![row(0, 0), row(1, -3)]
    }

    /// What `exam_one` actually holds: the compared rows plus the trailing
    /// post-final-step state (step 2) that DCGO never observes.
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
    fn backfill_writes_assertions_for_every_compared_step() {
        let out = backfill(LINE, &compared_rows()).expect("a complete clean run should backfill");
        let s = Scenario::from_yaml(&out).expect("the result must still parse");

        let gen = generated(&s);
        let ats: Vec<u32> = gen.iter().map(|a| a.at).collect();
        assert_eq!(ats, vec![0, 1], "one assertion per compared step");

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
    fn a_trailing_row_beyond_the_compared_steps_is_not_written() {
        // `exam_one`'s projections carry a post-final-step row (step 2) that
        // DCGO's pre-decision trace never observes.
        let ours = full_run();
        let observed = compared_rows();
        let report = diff(&observed, &observed);
        assert!(report.is_clean());
        assert_eq!(report.compared_steps, 2);

        let out = backfill_from_diff(LINE, &ours, &observed, &one_to_one(2), &report)
            .expect("clean run backfills");
        let s = Scenario::from_yaml(&out).unwrap();
        let ats: Vec<u32> = generated(&s).iter().map(|a| a.at).collect();
        assert_eq!(
            ats,
            vec![0, 1],
            "the unobserved trailing step must not be asserted"
        );

        // And the low-level entry point refuses the trailing row outright.
        let err = backfill(LINE, &ours).unwrap_err();
        assert!(err.contains("step 2"), "got: {err}");
    }

    #[test]
    fn backfill_refuses_when_the_diff_was_not_clean() {
        // Backfilling a diverged run would bake DCGO's disagreement in as our
        // expected value and make the scenario pass forever.
        let ours = compared_rows();
        let mut dcgo = compared_rows();
        dcgo[1] = row(1, 99);
        let report = diff(&ours, &dcgo);
        assert!(!report.is_clean(), "fixture must actually diverge");

        let err = backfill_from_diff(LINE, &ours, &dcgo, &one_to_one(2), &report).unwrap_err();
        assert!(err.contains("DIVERGED"), "got: {err}");
        assert!(err.contains("step 1"), "got: {err}");

        // A truncated run is refused too: it has no divergences either, and
        // "we got through 1 of 2 steps" must not read as "all agreed".
        let truncated = diff(&ours, &ours[..1]);
        assert!(truncated.divergences.is_empty());
        let err = backfill_from_diff(LINE, &ours, &ours[..1], &one_to_one(2), &truncated).unwrap_err();
        assert!(err.contains("TRUNCATED"), "got: {err}");

        // And the report-free entry point still refuses the truncation shape
        // it CAN see on its own: a confirmed set that misses a step.
        let err = backfill(LINE, &ours[..1]).unwrap_err();
        assert!(err.contains('1'), "the missing step must be named: {err}");

        // Nothing at all is not a pass either.
        assert!(backfill(LINE, &[]).is_err());

        // ...and a clean report does go through, so the gate is a gate and not
        // a blanket refusal.
        let clean = diff(&ours, &ours);
        assert!(clean.is_clean());
        assert!(backfill_from_diff(LINE, &ours, &ours, &one_to_one(2), &clean).is_ok());
    }

    #[test]
    fn only_the_rows_dcgo_observed_are_written_when_some_of_ours_had_no_partner() {
        // Step 0 is a sim-only row (zero wire rows): DCGO never observed it, so
        // it gets no assertion -- but the paired step 1 still does. Refusing the
        // whole file here left most multi-gate scenarios with no `assert:`
        // block at all (Plan 1 final review).
        let ours = compared_rows();
        let dcgo = vec![row(0, -3)];
        let pairing = StepPairing { pairs: vec![(1, 0)], ours_unpairable: 1, dcgo_unpairable: 0 };
        let report = crate::exam::differ::diff_paired(&ours, &dcgo, &pairing);
        assert!(report.is_clean(), "fixture must be a clean report: {report:?}");

        let out = backfill_from_diff(LINE, &ours, &dcgo, &pairing, &report)
            .expect("the paired row is confirmed and must be written");
        let s = Scenario::from_yaml(&out).unwrap();
        let ats: Vec<u32> = generated(&s).iter().map(|a| a.at).collect();
        assert_eq!(ats, vec![1], "only the step DCGO observed is asserted");
    }

    #[test]
    fn phase_is_left_out_wherever_the_differ_did_not_compare_it() {
        // Our engine parks a selection in its own phase (SelectTarget, ...)
        // and names the end-of-turn window EndOfTurnAction; DCGO stays on
        // Main for both, and the differ skips `phase` there. Writing our
        // phase would assert a value the oracle never confirmed.
        let ours = vec![with_phase(row(0, 0), "SelectTarget"), with_phase(row(1, -3), "EndOfTurnAction")];
        let dcgo = vec![row(0, 0), row(1, -3)];
        let report = crate::exam::differ::diff_paired(&ours, &dcgo, &one_to_one(2));
        assert!(report.is_clean(), "{report:?}");

        let out = backfill_from_diff(LINE, &ours, &dcgo, &one_to_one(2), &report).unwrap();
        let s = Scenario::from_yaml(&out).unwrap();
        for a in generated(&s) {
            assert!(!a.that.contains_key("phase"), "step {} asserts an uncompared phase", a.at);
            assert!(a.that.contains_key("memory"), "the compared fields are still written");
        }

        // A phase the differ DID compare is still asserted.
        let main = compared_rows();
        let report = diff(&main, &main);
        let out = backfill_from_diff(LINE, &main, &main, &one_to_one(2), &report).unwrap();
        let s = Scenario::from_yaml(&out).unwrap();
        assert!(generated(&s).iter().all(|a| a.that.contains_key("phase")));
    }

    #[test]
    fn backfill_refuses_an_inline_assert_value_with_a_readable_message() {
        let inline = format!("{LINE}assert: []\n");
        let err = backfill(&inline, &compared_rows()).unwrap_err();
        assert!(err.contains("inline value"), "got: {err}");
        assert!(!err.contains("  "), "no runs of spaces in the message: {err}");
    }

    #[test]
    fn backfill_refuses_when_the_rows_do_not_match_the_compared_count() {
        // The report compared 2 steps but the caller handed over only 1 row.
        let observed = compared_rows();
        let report = diff(&observed, &observed);
        assert_eq!(report.compared_steps, 2);
        let err = backfill_from_diff(LINE, &observed[..1], &observed, &one_to_one(2), &report)
            .unwrap_err();
        assert!(err.contains("do not match"), "got: {err}");

        // Rows that skip a decision point are refused by the coverage check.
        let gap = vec![row(0, 0), row(5, 1)];
        assert!(backfill(LINE, &gap).is_err());
    }

    #[test]
    fn backfill_preserves_hand_authored_assertions_it_did_not_generate() {
        let authored = format!("{LINE}assert:\n  - at: 1\n    that: {{ p0.memory: 3 }}\n");
        let out = backfill(&authored, &compared_rows()).unwrap();
        let s = Scenario::from_yaml(&out).unwrap();

        let hand: Vec<&Assertion> = s
            .assertions
            .iter()
            .filter(|a| !a.that.contains_key(GENERATED_MARKER))
            .collect();
        assert_eq!(hand.len(), 1, "the hand-authored assertion must survive");
        assert_eq!(hand[0].at, 1);
        assert_eq!(
            hand[0].that.get("p0.memory").unwrap(),
            &Value::Number(3.into()),
            "and must survive UNTOUCHED -- backfill owns only what it marked"
        );
        assert_eq!(generated(&s).len(), 2);
        // The hand-authored entry is still the original text, verbatim.
        assert!(
            out.contains("  - at: 1\n    that: { p0.memory: 3 }\n"),
            "got:\n{out}"
        );
    }

    #[test]
    fn backfill_keeps_the_scenario_text_and_only_adds_the_assert_block() {
        let commented = format!("# header comment\n# second line\n{LINE}");
        let out = backfill(&commented, &compared_rows()).unwrap();
        assert!(
            out.starts_with(&commented),
            "everything before the new block must be byte-for-byte the original"
        );
        assert!(out[commented.len()..].starts_with("assert:\n"));

        // With an existing assert section followed by another key, only the
        // generated entries change; header, steps, decks, the hand entry and
        // the trailing key are untouched.
        let with_tail = format!(
            "# top\n{LINE}assert:\n  # why\n  - at: 1\n    that: {{ p0.memory: 3 }}\nnotes: keep me\n"
        );
        let out = backfill(&with_tail, &compared_rows()).unwrap();
        let head = with_tail.split("assert:").next().unwrap();
        assert!(out.starts_with(head));
        assert!(out.contains("  # why\n  - at: 1\n    that: { p0.memory: 3 }\n"));
        assert!(out.ends_with("notes: keep me\n"));
        let s = Scenario::from_yaml(&out.replace("notes: keep me\n", "")).unwrap();
        assert_eq!(generated(&s).len(), 2);
    }

    #[test]
    fn backfill_is_idempotent() {
        // Running it twice must not accumulate duplicate `at:` entries.
        let once = backfill(LINE, &compared_rows()).unwrap();
        let twice = backfill(&once, &compared_rows()).unwrap();
        assert_eq!(once, twice, "a second backfill must be a no-op");

        let s = Scenario::from_yaml(&twice).unwrap();
        assert_eq!(generated(&s).len(), 2, "no duplicated generated entries");

        // Idempotent even with a hand-authored assertion in the file, and even
        // when the observed state CHANGED: the second run replaces the
        // generated block rather than appending a second copy of it.
        let authored = format!("{LINE}assert:\n  - at: 1\n    that: {{ p0.memory: 3 }}\n");
        let a1 = backfill(&authored, &compared_rows()).unwrap();
        assert_eq!(a1, backfill(&a1, &compared_rows()).unwrap());
        let mut changed = compared_rows();
        changed[1] = row(1, 7);
        let a2 = backfill(&a1, &changed).unwrap();
        let s = Scenario::from_yaml(&a2).unwrap();
        assert_eq!(generated(&s).len(), 2);
        assert_eq!(
            generated(&s)
                .iter()
                .find(|a| a.at == 1)
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
        let out = backfill(LINE, &compared_rows()).unwrap();
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
