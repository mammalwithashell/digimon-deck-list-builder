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
    /// Our rows the pairing deliberately excluded, because the step put
    /// NOTHING on the DCGO wire (`LoweredStep::SimOnlyAction`). Counted, never
    /// erased: a dropped row that vanished from the denominator would turn
    /// "we could not compare that row" into "that row agreed".
    #[serde(default)]
    pub ours_unpairable: u32,
    /// DCGO rows the pairing deliberately excluded: the 2nd..Nth row of a step
    /// that consumes several. Both traces are PRE-decision snapshots, so a
    /// fold's second row is a mid-resolution state our trace never
    /// materializes.
    #[serde(default)]
    pub dcgo_unpairable: u32,
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
        self.divergences.is_empty() && self.every_row_is_accounted_for()
    }

    /// Every row on both sides was either compared or deliberately excluded
    /// by the pairing. Reduces to `compared == ours == dcgo` when nothing was
    /// excluded, which is the plain [`diff`] case.
    fn every_row_is_accounted_for(&self) -> bool {
        self.compared_steps + self.ours_unpairable == self.ours_steps
            && self.compared_steps + self.dcgo_unpairable == self.dcgo_steps
    }

    /// `true` when the traces did not line up end to end. Distinct from
    /// "diverged": a truncated run is not a pass, but it is also not evidence
    /// that the engines disagree about the rules.
    pub fn is_truncated(&self) -> bool {
        !self.every_row_is_accounted_for()
    }

    /// Every divergence, field by field, instead of the lead plus a count.
    ///
    /// [`Display`](fmt::Display) deliberately leads with the one place the two
    /// engines parted and reduces the fallout to `(+N downstream ...)`, because
    /// a report ranking fifty consequences beside one cause is a report nobody
    /// finishes. That is the right default -- but it makes triaging a
    /// multi-gate line an exercise in ARITHMETIC: "4 gates + 1 sim-only row =
    /// 5 = lead + 4" is a derivation about which rows diverged, not a reading
    /// of them. This rendering turns the derivation back into a reading, for
    /// `exam --all-diffs`.
    ///
    /// The lead stays labelled so nobody mistakes the wreckage for the
    /// collision, and the denominator rides every line, exactly as in
    /// `Display`.
    pub fn render_verbose(&self) -> String {
        use fmt::Write as _;
        if self.is_clean() {
            return format!("CLEAN ({})", self.denominator());
        }
        let mut out = String::new();
        match self.first() {
            None => {
                let _ = write!(
                    out,
                    "TRUNCATED, no divergence found ({})",
                    self.denominator()
                );
                return out;
            }
            Some(first) => {
                let _ = writeln!(
                    out,
                    "DIVERGED at step {} ({}{})",
                    first.step,
                    self.denominator(),
                    if self.is_truncated() {
                        "; trace truncated"
                    } else {
                        ""
                    }
                );
            }
        }
        for d in &self.divergences {
            let _ = writeln!(
                out,
                "  {} step {}:",
                if d.downstream { "downstream" } else { "LEAD" },
                d.step
            );
            for f in &d.diffs {
                let _ = writeln!(out, "    {}: ours={} dcgo={}", f.path, f.ours, f.dcgo);
            }
        }
        out
    }

    /// The denominator, rendered. Every report line carries it.
    ///
    /// Deliberately-excluded rows are NAMED rather than netted out of the
    /// counts: "compared 3 of 3 ours / 4 dcgo steps (1 DCGO intermediate row
    /// not comparable)" says both that everything comparable was compared and
    /// that one row was not.
    pub fn denominator(&self) -> String {
        let mut s = format!(
            "compared {} of {} ours / {} dcgo steps",
            self.compared_steps, self.ours_steps, self.dcgo_steps
        );
        let mut excluded: Vec<String> = Vec::new();
        if self.ours_unpairable > 0 {
            excluded.push(format!(
                "{} sim-only row(s) with no DCGO prompt",
                self.ours_unpairable
            ));
        }
        if self.dcgo_unpairable > 0 {
            excluded.push(format!(
                "{} DCGO intermediate row(s) not comparable",
                self.dcgo_unpairable
            ));
        }
        if !excluded.is_empty() {
            s.push_str(&format!(" ({})", excluded.join(" + ")));
        }
        s
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
    // Combat-interrupt windows are the same representation class: DCGO's
    // TurnPhase stays on the interrupted phase while its block/counter/
    // alliance UI is open. Observed live on EX12-065#effect#4: the only diff
    // line was `phase: ours=BlockTiming dcgo=Main` with every state field
    // equal -- the blocker prompt round-tripped correctly and the projection
    // still compares suspension, DP and zones.
    phase.starts_with("Select")
        || phase == "EffectChoice"
        || phase == "BlockTiming"
        || phase == "CounterTiming"
        || phase == "AllianceTiming"
}

/// True for the ONE cross-engine phase PAIR that names a single window under
/// two spellings: our `EndOfTurnAction` park and DCGO's `Main`.
///
/// `general_rule.pdf` p.13 §6-6-1 — "the turn will end **with the current
/// phase**" — and §6-6-2 — "the current phase will continue until all
/// processing has been resolved". The end-of-turn window is therefore still
/// the Main phase as far as the rules are concerned, and DCGO implements that
/// literally: `AutoProcessing.cs::EndTurnProcess` runs the whole OnEndTurn
/// stack (the `<Execute>` / `<Engage>` OptionalSkill gate and any attack it
/// takes) while `TurnPhase` is still `Main`, only reaching `phase.End`
/// afterwards. Our engine gives that same window its own `GamePhase`
/// name. Two spellings, one window.
///
/// Deliberately a PAIR and not a blanket suppression like
/// [`is_selection_phase`]: `EndOfTurnAction` is a phase our engine can linger
/// in across a turn boundary, so suppressing it against `Breeding` / `Draw` /
/// `Active` / `End` would hide a genuine desync. `turn` is left comparing
/// independently for the same reason -- it matched at every observed gate row,
/// so a rotation only one side took must still surface.
fn is_equivalent_phase_pair(ours: &str, dcgo: &str) -> bool {
    ours == "EndOfTurnAction" && dcgo == "Main"
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
        ours_unpairable: 0,
        dcgo_unpairable: 0,
        divergences,
    }
}

/// Diff two traces paired by LOWERED SCENARIO STEP rather than by row index.
///
/// [`diff`] keys rows by their `step` field, which is right when both sides
/// emit one row per decision. They do not: a scenario step can consume one,
/// two, or zero DCGO decision rows depending on how the emitter lowered it
/// (an `OptionalSkill`+pick fold writes two; a sim-only phase exit writes
/// none). See [`pair_by_wire_rows`](crate::exam::projection::pair_by_wire_rows)
/// for the table and the measured witness.
///
/// The pairing decides WHICH rows are partners. It changes nothing about what
/// counts as a difference between partners, and the rows it excludes stay in
/// the denominator.
///
/// Divergences are reported at OUR row's `step`, so a finding points at the
/// scenario step its author wrote rather than at a DCGO row index.
pub fn diff_paired(
    ours: &[StateProjection],
    dcgo: &[StateProjection],
    pairing: &crate::exam::projection::StepPairing,
) -> DiffReport {
    let mut compared_steps = 0u32;
    let mut divergences: Vec<StepDivergence> = Vec::new();

    for (oi, di) in &pairing.pairs {
        let (Some(ours_row), Some(dcgo_row)) = (ours.get(*oi), dcgo.get(*di)) else {
            continue;
        };
        compared_steps += 1;
        let diffs = diff_projection(ours_row, dcgo_row);
        if !diffs.is_empty() {
            divergences.push(StepDivergence {
                step: ours_row.step,
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
        ours_unpairable: pairing.ours_unpairable,
        dcgo_unpairable: pairing.dcgo_unpairable,
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
    if !is_selection_phase(&ours.phase) && !is_equivalent_phase_pair(&ours.phase, &dcgo.phase) {
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
    fn a_combat_interrupt_phase_on_our_side_is_representation() {
        // EX12-065#effect#4 live case: BlockTiming vs Main, all state equal.
        let ours = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"BlockTiming","memory":0,
                "p0":{"security":5,"hand":[],"trash":[],"field":[]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#).unwrap();
        let dcgo = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"Main","memory":0,
                "p0":{"security":5,"hand":[],"trash":[],"field":[]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#).unwrap();
        assert!(diff(&[ours], &[dcgo]).is_clean());
    }

    // ── the EndOfTurnAction <-> Main phase PAIR ─────────────────────────
    //
    // general_rule.pdf p.13 §6-6-1 / §6-6-2: the turn ends *with the current
    // phase*, and the current phase *continues until all processing has been
    // resolved*. DCGO implements that literally -- `AutoProcessing.cs::
    // EndTurnProcess` runs the OnEndTurn stack (including <Execute>'s
    // OptionalSkill gate and any attack it takes) while `TurnPhase` is still
    // `Main`, and only moves to `phase.End` afterwards. Our engine gives the
    // same window its own name, `GamePhase::EndOfTurnAction`. Two names, one
    // window -- representation, not semantics.

    /// Build a projection pair differing ONLY in `phase`.
    fn phase_pair(ours_phase: &str, dcgo_phase: &str) -> (StateProjection, StateProjection) {
        let row = |phase: &str| {
            StateProjection::from_sidecar_line(&format!(
                r#"{{"step":0,"turn":4,"phase":"{phase}","memory":3,
                    "p0":{{"security":5,"hand":["A"],"trash":[],"field":[]}},
                    "p1":{{"security":4,"hand":[],"trash":["B"],"field":[]}}}}"#
            ))
            .unwrap()
        };
        (row(ours_phase), row(dcgo_phase))
    }

    #[test]
    fn our_end_of_turn_park_against_dcgos_main_is_representation() {
        let (ours, dcgo) = phase_pair("EndOfTurnAction", "Main");
        let r = diff(&[ours], &[dcgo]);
        assert!(
            r.is_clean(),
            "EndOfTurnAction/Main name the same §6-6-2 window: {:?}",
            r.divergences
        );
    }

    #[test]
    fn the_end_of_turn_pair_is_a_pair_not_a_blanket_suppression() {
        // `EndOfTurnAction` is a phase our engine can linger in across a turn
        // boundary, so suppressing it against ANY dcgo phase would hide a
        // genuine desync. Only the Main pairing is the §6-6-2 window.
        for dcgo_phase in ["Breeding", "Draw", "Active", "End"] {
            let (ours, dcgo) = phase_pair("EndOfTurnAction", dcgo_phase);
            let r = diff(&[ours], &[dcgo]);
            assert!(
                !r.is_clean(),
                "EndOfTurnAction vs {dcgo_phase} is a real desync and must diff"
            );
            assert_eq!(r.first().unwrap().diffs[0].path, "phase");
        }
    }

    #[test]
    fn the_end_of_turn_pair_does_not_suppress_the_other_direction() {
        // ours=Main / dcgo=EndOfTurnAction is not a shape either engine can
        // produce, but if it ever appeared it would be a finding, not noise.
        let (ours, dcgo) = phase_pair("Main", "EndOfTurnAction");
        assert!(!diff(&[ours], &[dcgo]).is_clean());
    }

    #[test]
    fn turn_still_compares_independently_at_the_end_of_turn_gate() {
        // The phase pair suppresses ONE field. `turn` matched at every gate
        // row of the observed corpus, so it stays a live comparison -- a
        // rotation our engine took and DCGO did not must still surface.
        let ours = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":4,"phase":"EndOfTurnAction","memory":3,
                "p0":{"security":5,"hand":[],"trash":[],"field":[]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#,
        )
        .unwrap();
        let dcgo = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":5,"phase":"Main","memory":3,
                "p0":{"security":5,"hand":[],"trash":[],"field":[]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#,
        )
        .unwrap();
        let r = diff(&[ours], &[dcgo]);
        assert!(!r.is_clean(), "a turn mismatch at the gate is semantics");
        assert_eq!(r.first().unwrap().diffs[0].path, "turn");
    }

    #[test]
    fn board_state_still_diffs_underneath_the_end_of_turn_pair() {
        // The normalization suppresses the phase LABEL only; everything the
        // gate actually did must still compare.
        let ours = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":4,"phase":"EndOfTurnAction","memory":3,
                "p0":{"security":5,"hand":[],"trash":[],"field":[]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#,
        )
        .unwrap();
        let dcgo = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":4,"phase":"Main","memory":3,
                "p0":{"security":5,"hand":[],"trash":[],"field":[]},
                "p1":{"security":3,"hand":[],"trash":[],"field":[]}}"#,
        )
        .unwrap();
        let r = diff(&[ours], &[dcgo]);
        assert!(!r.is_clean());
        assert_eq!(r.first().unwrap().diffs[0].path, "p1.security");
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

    // ── pairing by lowered step ─────────────────────────────────────────

    use crate::exam::projection::pair_by_wire_rows;

    #[test]
    fn diff_paired_compares_our_row_against_the_first_dcgo_row_of_its_step() {
        // The EX12-011#effect#0 shape: a 3-step line whose middle step is an
        // OptionalSkill+pick fold, so DCGO writes 4 rows for 3 steps. The
        // fold's second row is a mid-resolution state; positionally it would
        // sit opposite our LAST step and manufacture a divergence.
        let ours = vec![row(0, 0), row(1, 1), row(2, 2)];
        let dcgo = vec![row(0, 0), row(1, 1), row(9, 99), row(2, 2)];
        let pairing = pair_by_wire_rows(&[1, 2, 1], dcgo.len());

        // Positional alignment sees the manufactured divergence...
        assert!(!diff(&ours, &dcgo).is_clean());
        // ...the step-derived pairing does not.
        let r = diff_paired(&ours, &dcgo, &pairing);
        assert!(r.is_clean(), "{:?}", r.divergences);
        assert_eq!(r.compared_steps, 3);
    }

    #[test]
    fn a_dropped_intermediate_row_stays_in_the_denominator() {
        // "We could not compare that row" must never render as "that row
        // agreed".
        let ours = vec![row(0, 0), row(1, 1), row(2, 2)];
        let dcgo = vec![row(0, 0), row(1, 1), row(9, 99), row(2, 2)];
        let r = diff_paired(&ours, &dcgo, &pair_by_wire_rows(&[1, 2, 1], dcgo.len()));
        let d = r.denominator();
        assert!(d.contains("compared 3 of 3 ours / 4 dcgo"), "{d}");
        assert!(d.contains("1 DCGO intermediate"), "the drop must be named: {d}");
        assert!(r.to_string().contains("1 DCGO intermediate"), "{}", r);
    }

    #[test]
    fn a_sim_only_row_is_named_in_the_denominator_too() {
        let ours = vec![row(0, 0), row(1, 1), row(2, 2)];
        let dcgo = vec![row(0, 0), row(2, 2)];
        let r = diff_paired(&ours, &dcgo, &pair_by_wire_rows(&[1, 0, 1], dcgo.len()));
        assert!(r.is_clean(), "{:?}", r.divergences);
        let d = r.denominator();
        assert!(d.contains("1 sim-only"), "{d}");
    }

    #[test]
    fn a_dcgo_trace_longer_than_the_line_predicts_is_never_clean() {
        // Rows beyond what the lowered line predicts are UNACCOUNTED on
        // purpose: DCGO answered a prompt the scenario never wrote, and that
        // is a finding, not a pass.
        let ours = vec![row(0, 0), row(1, 1)];
        let dcgo = vec![row(0, 0), row(1, 1), row(2, 2)];
        let r = diff_paired(&ours, &dcgo, &pair_by_wire_rows(&[1, 1], dcgo.len()));
        assert!(r.divergences.is_empty(), "the paired rows agree");
        assert!(!r.is_clean(), "an unaccounted DCGO row is not a clean run");
        assert!(r.is_truncated());
    }

    #[test]
    fn a_dcgo_trace_shorter_than_the_line_predicts_is_never_clean() {
        let ours = vec![row(0, 0), row(1, 1), row(2, 2)];
        let dcgo = vec![row(0, 0)];
        let r = diff_paired(&ours, &dcgo, &pair_by_wire_rows(&[1, 1, 1], dcgo.len()));
        assert!(!r.is_clean());
        assert_eq!(r.compared_steps, 1);
        assert_eq!(r.ours_steps, 3);
    }

    #[test]
    fn the_pairing_never_hides_a_real_divergence() {
        // The normalization is about WHICH rows are partners, not about what
        // counts as a difference between partners.
        let ours = vec![row(0, 0), row(1, 1), row(2, 2)];
        let dcgo = vec![row(0, 0), row(1, 1), row(9, 99), row(2, 7)];
        let r = diff_paired(&ours, &dcgo, &pair_by_wire_rows(&[1, 2, 1], dcgo.len()));
        assert!(!r.is_clean());
        let first = r.first().unwrap();
        assert_eq!(first.step, 2, "the divergence is reported at OUR step index");
        assert_eq!(first.diffs[0].path, "memory");
        assert_eq!(first.diffs[0].dcgo, "7");
    }

    #[test]
    fn an_all_one_row_pairing_reproduces_plain_diff_exactly() {
        // The pairing must be a strict generalization: with one wire row per
        // step it has to agree with the positional path it replaces.
        let ours = vec![row(0, 0), row(1, 1), row(2, 2)];
        let dcgo = vec![row(0, 0), row(1, 9), row(2, 2)];
        let plain = diff(&ours, &dcgo);
        let paired = diff_paired(&ours, &dcgo, &pair_by_wire_rows(&[1, 1, 1], dcgo.len()));
        assert_eq!(plain, paired);
    }

    // ── verbose rendering ───────────────────────────────────────────────

    #[test]
    fn render_verbose_prints_every_divergence_not_just_the_lead() {
        // `Display` leads with the cause and counts the fallout, which is the
        // right default. But triaging a multi-gate line then requires DERIVING
        // which rows diverged from that count -- arithmetic, not reading. The
        // verbose rendering turns the derivation back into a reading.
        let ours = vec![row(0, 0), row(1, 1), row(2, 2), row(3, 3)];
        let dcgo = vec![row(0, 0), row(1, 9), row(2, 8), row(3, 7)];
        let r = diff(&ours, &dcgo);

        let lead_only = r.to_string();
        assert!(lead_only.contains("(+2 downstream"), "got: {lead_only}");
        assert!(!lead_only.contains("step 2"), "Display must stay lead-only: {lead_only}");

        let all = r.render_verbose();
        for step in ["step 1", "step 2", "step 3"] {
            assert!(all.contains(step), "{step} missing from:
{all}");
        }
        // Every row still shows its field-level values, so nothing has to be
        // inferred from a count.
        assert!(all.contains("memory: ours=1 dcgo=9"), "{all}");
        assert!(all.contains("memory: ours=2 dcgo=8"), "{all}");
        assert!(all.contains("memory: ours=3 dcgo=7"), "{all}");
    }

    #[test]
    fn render_verbose_marks_the_lead_and_labels_the_downstream_rows() {
        let ours = vec![row(0, 0), row(1, 1), row(2, 2)];
        let dcgo = vec![row(0, 0), row(1, 9), row(2, 9)];
        let all = diff(&ours, &dcgo).render_verbose();
        assert!(all.contains("LEAD"), "the cause must stay distinguishable: {all}");
        assert!(all.contains("downstream"), "{all}");
        // The denominator rides every rendering, verbose included.
        assert!(all.contains("compared 3 of 3 ours / 3 dcgo"), "{all}");
    }

    #[test]
    fn render_verbose_of_a_clean_report_says_clean() {
        let r = diff(&[row(0, 0)], &[row(0, 0)]);
        assert!(r.render_verbose().contains("CLEAN"));
        assert!(r.render_verbose().contains("compared 1 of 1 ours / 1 dcgo"));
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
