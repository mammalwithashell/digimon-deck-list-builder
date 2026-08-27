//! Emits a **draft** `cards_behavioral` test from what the oracle observed.
//!
//! # This module produces evidence, never truth
//!
//! DCGO is source-priority #2, below `general_rule.pdf`. What a draft records
//! is *what DCGO did*, which is strong evidence about a card and is not the
//! same claim as *what the rules say*. A generated test that asserted a
//! behavior nobody read would launder a DCGO quirk into a permanent guard, and
//! under the no-approximations policy a permanent guard around the wrong
//! behavior is worse than no guard at all. So the header says whose observation
//! this is, names the build and job it came from, and stops there — it makes no
//! claim about whether the behavior is right.
//!
//! # Returning a `String` is the safety property
//!
//! [`draft_test`] returns text. It never writes a file and never runs `git`.
//! Writing the draft into `code/digimon-engine/tests/cards_behavioral/<set>/`
//! is the CLI's job, behind an explicit `--write-draft` flag, with a human in
//! the loop. Keeping the drafter pure makes "the drafter never auto-commits" a
//! structural fact rather than a rule someone has to remember.
//!
//! # The body is left unwritten on purpose
//!
//! The emitted test is `#[ignore]`d and its body is `unimplemented!`. A
//! generated body that compiled and passed while asserting nothing would be a
//! false green — the exact failure mode the whole exam exists to remove.

use crate::exam::projection::StateProjection;
use crate::exam::scenario::Scenario;

/// Where a draft's observation came from. Every field lands in the header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    /// DCGO build hash the observation was made against.
    pub dcgo_build: String,
    /// Harness job that produced it.
    pub job_id: String,
    /// Repo-relative path of the scenario that was run.
    pub scenario_path: String,
}

/// Render a draft behavioral test for `scenario`, annotated with what the
/// oracle observed.
///
/// Returns the text. See the module docs: it never touches the filesystem.
pub fn draft_test(
    scenario: &Scenario,
    confirmed: &[StateProjection],
    provenance: &Provenance,
) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "// {}\n",
        "-".repeat(72)
    ));
    out.push_str("// DRAFT — machine-generated from an exam run. Read it before you keep it.\n");
    out.push_str("//\n");
    // The provenance line the header contract is written against:
    //   "DCGO build <hash>, job <id>, scenario <path> observed:"
    out.push_str(&format!(
        "// DCGO build {}, job {}, scenario {} observed:\n",
        provenance.dcgo_build, provenance.job_id, provenance.scenario_path
    ));
    out.push_str("//   the state recorded below is what DCGO DID. It is evidence about this\n");
    out.push_str("//   card, not a ruling on it: DCGO is source-priority #2, below\n");
    out.push_str("//   `general_rule.pdf`. Check the printed card text and the rules manual\n");
    out.push_str("//   before promoting any of this into a permanent guard.\n");
    out.push_str("//\n");
    out.push_str(&format!("// Card:   {}\n", scenario.card));
    out.push_str(&format!("// Clause: {}\n", scenario.clause));
    out.push_str(&format!("// Seed:   {}\n", scenario.seed));
    out.push_str("//\n");
    out.push_str("// Line of play, as authored:\n");
    for (i, step) in scenario.steps.iter().enumerate() {
        out.push_str(&format!(
            "//   {i}. seat {} -> {:?}\n",
            step.actor, step.act
        ));
    }
    out.push_str("//\n");
    if confirmed.is_empty() {
        out.push_str("// Observed state: NONE RECORDED. Nothing below is grounded in a run.\n");
    } else {
        out.push_str("// Observed state after each step:\n");
        let mut rows: Vec<&StateProjection> = confirmed.iter().collect();
        rows.sort_by_key(|p| p.step);
        for p in rows {
            out.push_str(&format!(
                "//   step {}: turn={} phase={} memory={}\n",
                p.step, p.turn, p.phase, p.memory
            ));
            out.push_str(&format!(
                "//     p0: security={} hand={:?} trash={:?} field={}\n",
                p.p0.security,
                p.p0.hand,
                p.p0.trash,
                render_field_of(&p.p0)
            ));
            out.push_str(&format!(
                "//     p1: security={} hand={:?} trash={:?} field={}\n",
                p.p1.security,
                p.p1.hand,
                p.p1.trash,
                render_field_of(&p.p1)
            ));
        }
    }
    out.push_str(&format!("// {}\n\n", "-".repeat(72)));

    let name = test_fn_name(scenario);
    out.push_str("#[test]\n");
    out.push_str(
        "#[ignore = \"draft: an unreviewed DCGO observation -- finish it and enable by hand\"]\n",
    );
    out.push_str(&format!("fn {name}() {{\n"));
    out.push_str("    // Express the line above as a DebugRunner test and assert the observed\n");
    out.push_str("    // state. Deliberately left unwritten: a generated body that passes\n");
    out.push_str("    // while asserting nothing is a false green, which is the failure mode\n");
    out.push_str("    // this whole exam exists to remove.\n");
    out.push_str(
        "    unimplemented!(\"draft: write the assertions from the observation block above\");\n",
    );
    out.push_str("}\n");

    out
}

fn render_field_of(seat: &crate::exam::projection::SeatProjection) -> String {
    let parts: Vec<String> = seat
        .field
        .iter()
        .map(|perm| {
            format!(
                "{}(dp={}{}{})",
                perm.card_id,
                perm.dp,
                if perm.suspended { ", suspended" } else { "" },
                if perm.sources.is_empty() {
                    String::new()
                } else {
                    format!(", under={:?}", perm.sources)
                }
            )
        })
        .collect();
    format!("[{}]", parts.join(", "))
}

/// `EX12-035#effect#0` -> `dcgo_exam_ex12_035_effect_0`.
fn test_fn_name(scenario: &Scenario) -> String {
    let mut name = String::from("dcgo_exam_");
    let mut last_was_sep = true;
    for ch in scenario.clause.chars() {
        if ch.is_ascii_alphanumeric() {
            name.push(ch.to_ascii_lowercase());
            last_was_sep = false;
        } else if !last_was_sep {
            name.push('_');
            last_was_sep = true;
        }
    }
    while name.ends_with('_') {
        name.pop();
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

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
  - actor: 0
    do: { play: { card: EX12-035, from: hand } }
"#;

    fn scenario() -> Scenario {
        Scenario::from_yaml(LINE).expect("fixture should parse")
    }

    fn prov() -> Provenance {
        Provenance {
            dcgo_build: "8c4f98cb6".to_string(),
            job_id: "vol-00042".to_string(),
            scenario_path: "qa/dcgo-exams/EX12/EX12-035.yaml".to_string(),
        }
    }

    #[test]
    fn the_header_records_provenance_and_does_not_claim_correctness() {
        let out = draft_test(&scenario(), &[], &prov());
        assert!(out.contains("DCGO build"));
        assert!(out.contains("job "));
        assert!(out.contains("observed:"));
        // The exact provenance line, not just its pieces scattered around.
        assert!(
            out.contains(
                "DCGO build 8c4f98cb6, job vol-00042, \
                 scenario qa/dcgo-exams/EX12/EX12-035.yaml observed:"
            ),
            "got:\n{out}"
        );
        // DCGO is source-priority #2, below general_rule.pdf. A drafted test
        // encodes strong evidence, not truth -- and a generated test asserting
        // a behavior nobody read would launder a DCGO quirk into a permanent
        // guard, which under the no-approximations policy is worse than no
        // test.
        assert!(
            !out.to_lowercase().contains("correct"),
            "the header must not assert correctness:\n{out}"
        );
        assert!(
            out.contains("general_rule.pdf"),
            "the header must point the reader at the higher-priority source"
        );
    }

    #[test]
    fn output_is_a_compilable_test_shape() {
        let out = draft_test(&scenario(), &[], &prov());
        assert!(out.contains("#[test]"));
        assert!(out.contains("fn "));
        assert!(
            out.contains("fn dcgo_exam_ex12_035_effect_0()"),
            "the fn name must be derived from the clause id:\n{out}"
        );
        // The body must not silently pass: an empty generated test would be a
        // false green.
        assert!(out.contains("unimplemented!"));
        assert!(out.contains("#[ignore"));
    }

    #[test]
    fn the_draft_is_returned_never_written_to_disk() {
        // The drafter must never auto-commit; returning a String is what makes
        // that structural rather than a rule someone has to remember. There is
        // no path argument it *could* write to.
        let out = draft_test(&scenario(), &[], &prov());
        assert!(!out.is_empty());
    }

    #[test]
    fn observed_state_is_rendered_and_its_absence_is_said_out_loud() {
        // A draft with no run behind it must say so, or it reads like an
        // observation that happened to be empty.
        let none = draft_test(&scenario(), &[], &prov());
        assert!(none.contains("NONE RECORDED"), "got:\n{none}");

        let row = StateProjection::from_sidecar_line(
            r#"{"step":2,"turn":1,"phase":"Main","memory":-3,
                "p0":{"security":5,"hand":["ST1-02"],"trash":[],
                      "field":[{"card_id":"EX12-035","dp":4000,
                                "suspended":false,"sources":[]}]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#,
        )
        .unwrap();
        let some = draft_test(&scenario(), &[row], &prov());
        assert!(some.contains("step 2: turn=1 phase=Main memory=-3"), "got:\n{some}");
        assert!(some.contains("EX12-035(dp=4000)"), "got:\n{some}");
        assert!(!some.contains("NONE RECORDED"));
        // Still no correctness claim once there IS an observation.
        assert!(!some.to_lowercase().contains("correct"));
    }
}
