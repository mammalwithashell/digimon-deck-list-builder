//! Aligns two per-step projections and reports the **first** divergence.
//!
//! Two properties carry this module's design; both exist because of how the
//! report gets read, not because of how it gets computed.
//!
//! # 1. Lead with the first divergence
//!
//! Once two engines part, they are playing different games. Every state
//! difference at every later step is a *consequence* of the one place they
//! actually disagreed, and a report that ranks fifty consequences alongside
//! the single cause is a report nobody finishes reading. So [`diff`] marks
//! exactly one divergence as the lead ([`StepDivergence::downstream`] =
//! `false`) and every later one as `downstream: true`, and
//! [`DiffReport::first`] hands back the lead directly.
//!
//! Downstream divergences are kept rather than discarded: sometimes the shape
//! of the fallout tells you which clause misfired. But they are labelled, so
//! nobody mistakes the wreckage for the collision.
//!
//! # 2. Always print the full denominator
//!
//! A comparison that only got through 2 of 5 steps must **never** read the
//! same as one that ran all 5 and found nothing. That is the difference
//! between "our engine matches DCGO" and "our engine matched DCGO right up
//! until one of us fell over". So the report always carries all three counts
//! — `ours_steps`, `dcgo_steps`, `compared_steps` — and
//! [`DiffReport::is_clean`] is true **only** when there are no divergences
//! *and* all three agree. A truncated run is not a pass.
//!
//! The `Display` impl prints the denominator on every line it emits,
//! including the clean one, for exactly the same reason.
//!
//! # Alignment is by step index, never by position
//!
//! The two traces may skip different rows — DCGO does not dump a projection
//! for every internal transition our engine names a step, and vice versa.
//! Zipping them positionally would offset one side by one and report the
//! whole rest of the game as divergent. Rows are therefore keyed by their
//! `step` field, and only steps present on *both* sides are compared; the
//! ones that are not show up in the denominator as a shortfall.
//!
//! # Comparison respects the projection's wildcards
//!
//! Permanents compare field by field, but the keyword drill-down only reports
//! a difference when **both** sides measured them. `None` means "nobody
//! looked", and [`PermanentProjection`]'s own `PartialEq` treats it as a
//! wildcard — if the differ reported it anyway, a keyword-flag-off run would
//! grow a divergence the equality check itself does not see.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::exam::projection::{PermanentProjection, SeatProjection, StateProjection};

/// One named field that differs, with both sides rendered for the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldDiff {
    /// Dotted path into the projection, e.g. `memory`, `p0.field[1].dp`.
    pub path: String,
    pub ours: String,
    pub dcgo: String,
}

/// Everything that differed at one step index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepDivergence {
    pub step: u32,
    pub diffs: Vec<FieldDiff>,
    /// `false` for the first divergence in the trace, `true` for every later
    /// one — later divergences are almost always fallout from the first.
    pub downstream: bool,
}

/// The result of aligning two traces. The three counts are the denominator
/// and are always populated, clean run or not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffReport {
    /// Steps present on both sides and actually compared.
    pub compared_steps: u32,
    /// Rows our engine produced.
    pub ours_steps: u32,
    /// Rows DCGO produced.
    pub dcgo_steps: u32,
    pub divergences: Vec<StepDivergence>,
}

impl DiffReport {
    /// The lead divergence — the one place the two engines actually parted.
    pub fn first(&self) -> Option<&StepDivergence> {
        self.divergences.first()
    }

    /// True only when nothing diverged **and** the whole trace was compared.
    ///
    /// The length check is not belt-and-braces: a run that died after two of
    /// five steps has no divergences either, and reporting that as clean is
    /// how a broken scenario gets recorded as a confirmed clause.
    pub fn is_clean(&self) -> bool {
        self.divergences.is_empty()
            && self.ours_steps == self.dcgo_steps
            && self.compared_steps == self.ours_steps
    }

    /// `true` when the traces did not line up end to end. Distinct from
    /// "diverged": a truncated run is not a pass, but it is also not evidence
    /// that the engines disagree about the rules.
    pub fn is_truncated(&self) -> bool {
        !(self.ours_steps == self.dcgo_steps && self.compared_steps == self.ours_steps)
    }

    /// The denominator, rendered. Every report line carries it.
    pub fn denominator(&self) -> String {
        format!(
            "compared {} of {} ours / {} dcgo steps",
            self.compared_steps, self.ours_steps, self.dcgo_steps
        )
    }
}

impl fmt::Display for DiffReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_clean() {
            return write!(f, "CLEAN ({})", self.denominator());
        }
        match self.first() {
            None => write!(f, "TRUNCATED, no divergence found ({})", self.denominator()),
            Some(first) => {
                writeln!(
                    f,
                    "DIVERGED at step {} ({}{})",
                    first.step,
                    self.denominator(),
                    if self.is_truncated() {
                        "; trace truncated"
                    } else {
                        ""
                    }
                )?;
                for d in &first.diffs {
                    writeln!(f, "  {}: ours={} dcgo={}", d.path, d.ours, d.dcgo)?;
                }
                let downstream = self.divergences.len().saturating_sub(1);
                if downstream > 0 {
                    write!(
                        f,
                        "  (+{downstream} downstream divergence(s) at later steps)"
                    )?;
                }
                Ok(())
            }
        }
    }
}

/// Align two traces by step index and report their differences.
///
/// See the module docs: leads with the first divergence, keeps the rest as
/// `downstream`, and always reports the full denominator.
/// True for our engine's selection-interlude phases -- phases DCGO does not
/// represent at all (its TurnPhase stays on the interrupted phase while a
/// selection UI is open).
fn is_selection_phase(phase: &str) -> bool {
    phase.starts_with("Select") || phase == "EffectChoice"
}

pub fn diff(ours: &[StateProjection], dcgo: &[StateProjection]) -> DiffReport {
    // Keyed by `step`, not by position — the two sides may skip different
    // rows and a positional zip would offset one of them by one.
    let ours_by_step: BTreeMap<u32, &StateProjection> = ours.iter().map(|p| (p.step, p)).collect();
    let dcgo_by_step: BTreeMap<u32, &StateProjection> = dcgo.iter().map(|p| (p.step, p)).collect();

    let mut compared_steps = 0u32;
    let mut divergences: Vec<StepDivergence> = Vec::new();

    // BTreeMap iteration is ascending, so divergences come out in step order
    // and the first one really is the earliest.
    for (step, ours_row) in &ours_by_step {
        let Some(dcgo_row) = dcgo_by_step.get(step) else {
            continue;
        };
        compared_steps += 1;
        let diffs = diff_projection(ours_row, dcgo_row);
        if !diffs.is_empty() {
            divergences.push(StepDivergence {
                step: *step,
                diffs,
                // Provisional; fixed up below so the first is the lead.
                downstream: true,
            });
        }
    }

    if let Some(first) = divergences.first_mut() {
        first.downstream = false;
    }

    DiffReport {
        compared_steps,
        ours_steps: ours.len() as u32,
        dcgo_steps: dcgo.len() as u32,
        divergences,
    }
}

fn diff_projection(ours: &StateProjection, dcgo: &StateProjection) -> Vec<FieldDiff> {
    let mut diffs = Vec::new();
    push_ne(&mut diffs, "turn", &ours.turn, &dcgo.turn);
    // Phase compares through a normalization: our engine models a pending
    // selection as its own GamePhase (SelectTarget / SelectBudgeted /
    // SelectHand / EffectChoice / ...), while DCGO's TurnPhase stays on the
    // phase the selection interrupted ("Main", "Attack"...). Which engine
    // parks a phase marker is REPRESENTATION; whether the selection resolved
    // to the same board state is the semantics, and every other field still
    // compares. So a Select*/EffectChoice phase on our side never diffs
    // against DCGO's phase -- observed live on the ST1-15 gate: step 12 read
    // `phase: ours=SelectBudgeted dcgo=Main` with every state field equal,
    // turning a CLEAN selection round-trip into a false divergence.
    if !is_selection_phase(&ours.phase) {
        push_ne(&mut diffs, "phase", ours.phase.as_str(), dcgo.phase.as_str());
    }
    push_ne(&mut diffs, "memory", &ours.memory, &dcgo.memory);
    diff_seat(&mut diffs, "p0", &ours.p0, &dcgo.p0);
    diff_seat(&mut diffs, "p1", &ours.p1, &dcgo.p1);
    diffs
}

fn diff_seat(out: &mut Vec<FieldDiff>, seat: &str, ours: &SeatProjection, dcgo: &SeatProjection) {
    push_ne(
        out,
        &format!("{seat}.security"),
        &ours.security,
        &dcgo.security,
    );
    push_list_ne(out, &format!("{seat}.hand"), &ours.hand, &dcgo.hand);
    push_list_ne(out, &format!("{seat}.trash"), &ours.trash, &dcgo.trash);

    if ours.field.len() != dcgo.field.len() {
        // Different-length fields cannot be aligned index-wise without
        // inventing a pairing the other side never made, so report the shape
        // once rather than manufacturing N per-slot diffs.
        out.push(FieldDiff {
            path: format!("{seat}.field"),
            ours: render_field(&ours.field),
            dcgo: render_field(&dcgo.field),
        });
        return;
    }
    for (i, (a, b)) in ours.field.iter().zip(dcgo.field.iter()).enumerate() {
        diff_permanent(out, &format!("{seat}.field[{i}]"), a, b);
    }
}

fn diff_permanent(
    out: &mut Vec<FieldDiff>,
    path: &str,
    ours: &PermanentProjection,
    dcgo: &PermanentProjection,
) {
    push_ne(
        out,
        &format!("{path}.card_id"),
        ours.card_id.as_str(),
        dcgo.card_id.as_str(),
    );
    push_ne(out, &format!("{path}.dp"), &ours.dp, &dcgo.dp);
    push_ne(
        out,
        &format!("{path}.suspended"),
        &ours.suspended,
        &dcgo.suspended,
    );
    push_list_ne(
        out,
        &format!("{path}.sources"),
        &ours.sources,
        &dcgo.sources,
    );
    // Only when BOTH sides measured — see the module docs.
    if let (Some(a), Some(b)) = (&ours.keywords, &dcgo.keywords) {
        push_list_ne(out, &format!("{path}.keywords"), a, b);
    }
}

fn push_ne<T: PartialEq + fmt::Display + ?Sized>(
    out: &mut Vec<FieldDiff>,
    path: &str,
    ours: &T,
    dcgo: &T,
) {
    if ours != dcgo {
        out.push(FieldDiff {
            path: path.to_string(),
            ours: ours.to_string(),
            dcgo: dcgo.to_string(),
        });
    }
}

fn push_list_ne(out: &mut Vec<FieldDiff>, path: &str, ours: &[String], dcgo: &[String]) {
    if ours != dcgo {
        out.push(FieldDiff {
            path: path.to_string(),
            ours: render_list(ours),
            dcgo: render_list(dcgo),
        });
    }
}

fn render_list(items: &[String]) -> String {
    format!("[{}]", items.join(", "))
}

fn render_field(field: &[PermanentProjection]) -> String {
    let rendered: Vec<String> = field
        .iter()
        .map(|p| {
            format!(
                "{}(dp={}{}{})",
                p.card_id,
                p.dp,
                if p.suspended { ", suspended" } else { "" },
                if p.sources.is_empty() {
                    String::new()
                } else {
                    format!(", under={}", render_list(&p.sources))
                }
            )
        })
        .collect();
    format!("[{}]", rendered.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exam::projection::StateProjection;

    fn row(step: u32, memory: i64) -> StateProjection {
        StateProjection::from_sidecar_line(&format!(
            r#"{{"step":{step},"turn":1,"phase":"Main","memory":{memory},
               "p0":{{"security":5,"hand":[],"trash":[],"field":[]}},
               "p1":{{"security":5,"hand":[],"trash":[],"field":[]}}}}"#
        ))
        .unwrap()
    }


    #[test]
    fn a_selection_phase_on_our_side_is_representation_not_a_divergence() {
        // Live case from the ST1-15 gate: our engine parks in a Select* phase
        // while DCGO's TurnPhase stays "Main" during the selection UI. All
        // state fields equal -> must be CLEAN.
        let ours = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"SelectBudgeted","memory":0,
                "p0":{"security":5,"hand":[],"trash":[],"field":[]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#).unwrap();
        let dcgo = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"Main","memory":0,
                "p0":{"security":5,"hand":[],"trash":[],"field":[]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#).unwrap();
        let r = diff(&[ours], &[dcgo]);
        assert!(r.is_clean(), "{:?}", r.divergences);
    }

    #[test]
    fn a_real_phase_mismatch_still_diffs() {
        // The normalization must not swallow a genuine phase disagreement.
        let ours = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"Breeding","memory":0,
                "p0":{"security":5,"hand":[],"trash":[],"field":[]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#).unwrap();
        let dcgo = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"Main","memory":0,
                "p0":{"security":5,"hand":[],"trash":[],"field":[]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#).unwrap();
        let r = diff(&[ours], &[dcgo]);
        assert!(!r.is_clean(), "Breeding-vs-Main is semantics and must diff");
    }

    #[test]
    fn identical_traces_are_clean() {
        let a = vec![row(0, 0), row(1, 1)];
        let r = diff(&a, &a);
        assert!(r.is_clean());
        assert_eq!(r.compared_steps, 2);
    }

    #[test]
    fn the_first_divergence_leads_and_the_rest_are_downstream() {
        // Once two engines part they are playing different games. A report
        // ranking fifty consequences beside one cause is a report nobody
        // finishes.
        let ours = vec![row(0, 0), row(1, 1), row(2, 2)];
        let dcgo = vec![row(0, 0), row(1, 9), row(2, 9)];
        let r = diff(&ours, &dcgo);
        assert!(!r.is_clean());
        let first = r.first().unwrap();
        assert_eq!(first.step, 1);
        assert!(!first.downstream);
        assert!(
            r.divergences.iter().skip(1).all(|d| d.downstream),
            "everything after the first divergence must be marked downstream"
        );
    }

    #[test]
    fn a_diff_names_the_field_path_and_both_values() {
        let ours = vec![row(0, 3)];
        let dcgo = vec![row(0, 7)];
        let r = diff(&ours, &dcgo);
        let d = &r.first().unwrap().diffs[0];
        assert_eq!(d.path, "memory");
        assert_eq!(d.ours, "3");
        assert_eq!(d.dcgo, "7");
    }

    #[test]
    fn unequal_lengths_report_the_full_denominator() {
        // "Ran 2 of 5 steps" must never read the same as "ran all 5 clean".
        let ours = vec![row(0, 0), row(1, 0), row(2, 0)];
        let dcgo = vec![row(0, 0)];
        let r = diff(&ours, &dcgo);
        assert_eq!(r.ours_steps, 3);
        assert_eq!(r.dcgo_steps, 1);
        assert_eq!(r.compared_steps, 1);
        assert!(!r.is_clean(), "a truncated comparison is not a clean pass");
    }

    #[test]
    fn empty_dcgo_trace_is_not_a_pass() {
        let ours = vec![row(0, 0)];
        let r = diff(&ours, &[]);
        assert!(!r.is_clean());
    }

    // --- supporting tests beyond the plan's five ---------------------------

    #[test]
    fn alignment_is_by_step_index_not_by_position() {
        // The two traces skip different rows. A positional zip would pair
        // ours[1] (step 1) with dcgo[1] (step 2) and report the rest of the
        // game as divergent -- a phantom finding from an offset, not a bug.
        let ours = vec![row(0, 0), row(1, 1), row(2, 2)];
        let dcgo = vec![row(0, 0), row(2, 2)];
        let r = diff(&ours, &dcgo);
        assert!(
            r.divergences.is_empty(),
            "steps present on both sides agree: {:?}",
            r.divergences
        );
        // ...but the shortfall still shows in the denominator, so this is not
        // reported as a pass.
        assert_eq!(r.compared_steps, 2);
        assert_eq!(r.ours_steps, 3);
        assert_eq!(r.dcgo_steps, 2);
        assert!(!r.is_clean());
        assert!(r.is_truncated());
    }

    #[test]
    fn a_truncated_run_never_renders_like_a_clean_one() {
        let clean = diff(&[row(0, 0)], &[row(0, 0)]);
        let truncated = diff(&[row(0, 0), row(1, 0)], &[row(0, 0)]);
        assert!(clean.to_string().contains("CLEAN"));
        assert!(!truncated.to_string().contains("CLEAN"));
        // Every rendering carries the denominator, clean or not.
        assert!(clean.to_string().contains("compared 1 of 1 ours / 1 dcgo"));
        assert!(truncated
            .to_string()
            .contains("compared 1 of 2 ours / 1 dcgo"));
    }

    #[test]
    fn board_state_diffs_name_their_path() {
        let ours = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"Main","memory":0,
                "p0":{"security":5,"hand":[],"trash":[],
                      "field":[{"card_id":"EX12-035","dp":4000,"suspended":false,"sources":[]}]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#,
        )
        .unwrap();
        let dcgo = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"Main","memory":0,
                "p0":{"security":4,"hand":[],"trash":[],
                      "field":[{"card_id":"EX12-035","dp":7000,"suspended":true,"sources":[]}]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#,
        )
        .unwrap();
        let r = diff(&[ours], &[dcgo]);
        let paths: Vec<&str> = r
            .first()
            .unwrap()
            .diffs
            .iter()
            .map(|d| d.path.as_str())
            .collect();
        assert_eq!(
            paths,
            vec!["p0.security", "p0.field[0].dp", "p0.field[0].suspended"]
        );
    }

    #[test]
    fn an_unmeasured_keyword_list_is_never_a_divergence() {
        // DCGO's keyword dump defaults OFF; an absent list means "nobody
        // looked". Reporting it would put a keyword divergence on every
        // permanent of every flag-off run.
        let with = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"Main","memory":0,
                "p0":{"security":5,"hand":[],"trash":[],
                      "field":[{"card_id":"X","dp":1000,"suspended":false,
                                "sources":[],"keywords":["Blocker"]}]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#,
        )
        .unwrap();
        let without = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"Main","memory":0,
                "p0":{"security":5,"hand":[],"trash":[],
                      "field":[{"card_id":"X","dp":1000,"suspended":false,"sources":[]}]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#,
        )
        .unwrap();
        assert!(diff(&[with.clone()], &[without.clone()]).is_clean());
        assert!(diff(&[without], &[with.clone()]).is_clean());

        // Measured-vs-measured still diffs, and names the path.
        let other = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"Main","memory":0,
                "p0":{"security":5,"hand":[],"trash":[],
                      "field":[{"card_id":"X","dp":1000,"suspended":false,
                                "sources":[],"keywords":["Rush"]}]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#,
        )
        .unwrap();
        let r = diff(&[with], &[other]);
        assert_eq!(r.first().unwrap().diffs[0].path, "p0.field[0].keywords");
    }

    #[test]
    fn a_field_of_a_different_size_reports_the_shape_once() {
        let ours = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"Main","memory":0,
                "p0":{"security":5,"hand":[],"trash":[],
                      "field":[{"card_id":"A","dp":1000,"suspended":false,"sources":[]},
                               {"card_id":"B","dp":2000,"suspended":false,"sources":[]}]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#,
        )
        .unwrap();
        let dcgo = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"Main","memory":0,
                "p0":{"security":5,"hand":[],"trash":[],
                      "field":[{"card_id":"A","dp":1000,"suspended":false,"sources":[]}]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#,
        )
        .unwrap();
        let r = diff(&[ours], &[dcgo]);
        let diffs = &r.first().unwrap().diffs;
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "p0.field");
        assert!(diffs[0].ours.contains("B(dp=2000)"));
        assert!(!diffs[0].dcgo.contains('B'));
    }

    #[test]
    fn duplicate_step_ids_can_never_read_as_clean() {
        // Two rows sharing a step index collapse to one comparison, so the
        // row count and the compared count part -- and the denominator says
        // so instead of quietly reporting a pass.
        let ours = vec![row(0, 0), row(0, 0)];
        let dcgo = vec![row(0, 0)];
        let r = diff(&ours, &dcgo);
        assert_eq!(r.ours_steps, 2);
        assert_eq!(r.compared_steps, 1);
        assert!(!r.is_clean());
    }
}
