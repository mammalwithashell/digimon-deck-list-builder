//! The oracle half of an exam run: read DCGO's recording and state sidecar,
//! align the trace to the scenario origin, pair it with our lowered steps, and
//! diff. Moved out of the `dcgo-harness` binary so `exam --sidecar`,
//! `exam --oracle` and the MCP share one pipeline (oracle-readiness Plan 3,
//! Task 2).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use digimon_engine::CardData;

use crate::exam::differ::{diff_paired, DiffReport};
use crate::exam::projection::{
    align_to_scenario_origin, pair_by_wire_rows_with_ownership, parse_sidecar, StateProjection,
    StepPairing,
};
use crate::exam::run::lower_and_run;
use crate::exam::scenario::Scenario;

/// What diffing one lowered line against one DCGO sidecar established.
pub struct OracleDiff {
    pub report: DiffReport,
    /// Our projections incl. the trailing state (what `assert:` talks about).
    pub projections: Vec<StateProjection>,
    /// DCGO's aligned projections.
    pub dcgo: Vec<StateProjection>,
    /// Which of our rows was compared against which DCGO row -- what backfill
    /// needs to write only oracle-observed state.
    pub pairing: StepPairing,
}

/// The recording `.jsonl` beside a `.state.jsonl` sidecar, read whole.
pub fn recording_path_beside(sidecar: &Path) -> PathBuf {
    let s = sidecar.to_string_lossy();
    PathBuf::from(
        s.strip_suffix(".state.jsonl")
            .map(|stem| format!("{stem}.jsonl"))
            .unwrap_or_else(|| s.to_string()),
    )
}

/// Read the recording beside `sidecar`. It is required both to align the
/// oracle trace and to take DCGO's post-shuffle deck order.
pub fn recording_beside(sidecar: &Path) -> Result<String, String> {
    let rp = recording_path_beside(sidecar);
    std::fs::read_to_string(&rp).map_err(|e| {
        format!(
            "reading the recording beside the sidecar ({}): {e}. It is required both                  to align the oracle trace to the scenario's origin and to take DCGO's                  post-shuffle deck order.",
            rp.display()
        )
    })
}

/// Both seats' decks in DCGO's own post-shuffle order, taken from a recording's
/// `game_start` row.
///
/// The row is written from one seat's perspective (`my_player_id`), so the two
/// lists have to be assigned by that id rather than positionally. Egg cards are
/// appended: `Game::new_inner` splits them out by card kind, and the scenario's
/// deck argument is one flat list per seat.
pub fn decks_from_recording(text: &str) -> Result<(Vec<String>, Vec<String>), String> {
    let first = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .ok_or_else(|| "recording is empty".to_string())?;
    let row: serde_json::Value =
        serde_json::from_str(first).map_err(|e| format!("malformed game_start row: {e}"))?;
    if row.get("type").and_then(|v| v.as_str()) != Some("game_start") {
        return Err("first recording row is not game_start".to_string());
    }

    let arr = |k: &str| -> Result<Vec<String>, String> {
        row.get(k)
            .and_then(|v| v.as_array())
            .ok_or_else(|| format!("game_start has no array `{k}`"))
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
    };

    let my_id = row
        .get("my_player_id")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "game_start has no my_player_id".to_string())?;

    // REVERSED: the two engines number a deck from opposite ends. DCGO records
    // top-first (`my_deck_post_shuffle[..5]` is exactly its `initial_hand`),
    // while our `Player::draw` pops from the BACK of the vector. Feeding the
    // recorded order straight through dealt our seats the deck's LAST five
    // cards -- verified precisely: our p0 hand equalled
    // `my_deck_post_shuffle[45..]`, card for card, and p1 the same on its own
    // list. Reversing makes the top of the deck the back of the vector, so both
    // engines deal the same opening hand from the same recorded shuffle.
    let mut mine: Vec<String> = arr("my_deck_post_shuffle")?.into_iter().rev().collect();
    mine.extend(arr("my_egg_deck").unwrap_or_default());
    let mut theirs: Vec<String> = arr("opp_deck_post_shuffle")?.into_iter().rev().collect();
    theirs.extend(arr("opp_egg_deck").unwrap_or_default());

    Ok(if my_id == 0 {
        (mine, theirs)
    } else {
        (theirs, mine)
    })
}

/// Diff an already-lowered line (its projections and pairing inputs) against
/// the DCGO sidecar at `sidecar_path`, whose recording text is given.
pub fn diff_lowered(
    scenario: &Scenario,
    projections: Vec<StateProjection>,
    steps: usize,
    wire_rows_per_step: &[usize],
    ours_present_per_step: &[bool],
    sidecar_path: &Path,
    recording_text: &str,
) -> Result<OracleDiff, String> {
    let sidecar_text = std::fs::read_to_string(&sidecar_path)
        .map_err(|e| format!("reading sidecar {}: {e}", sidecar_path.display()))?;
    let dcgo = parse_sidecar(&sidecar_text)?;

    // Align the oracle trace to the scenario's origin: DCGO records the
    // mulligan as action rows and dumps state at them, while our line begins
    // after it. Left alone the traces sit two steps apart and every scenario
    // reports a spurious divergence at step 0.
    let mut dcgo = align_to_scenario_origin(dcgo, &recording_text)?;
    // Compare "state at each decision point" on both sides. Ours holds one
    // projection BEFORE each step plus a trailing one AFTER the last, while
    // StateDumper writes one before each decision and stops. Dropping our
    // trailing entry aligns the two ends; keeping it made the differ report
    // TRUNCATED (correctly -- it refuses to call an unequal comparison clean)
    // on a run that had no divergence at all.
    //
    // `projections` itself keeps the trailing state, because an `assert:` block
    // legitimately wants to talk about the position AFTER the final step.
    let mut ours_for_diff: Vec<_> = projections.iter().take(steps).cloned().collect();
    // ...and pair the two by LOWERED STEP rather than 1:1, because a step can
    // consume one, two, or ZERO DCGO decision rows (an OptionalSkill+pick fold
    // writes two; a sim-only phase exit writes none). A positional pairing
    // slides apart from the first multi-row step onward and manufactures
    // divergences out of the offset -- measured on EX12-011#effect#0, where our
    // post-battle row was compared against DCGO's pre-battle mid-fold row.
    let pairing =
        pair_by_wire_rows_with_ownership(wire_rows_per_step, ours_present_per_step, dcgo.len());
    prepare_requested_measurements(scenario, &mut ours_for_diff, &mut dcgo, &pairing)?;
    let report = diff_paired(&ours_for_diff, &dcgo, &pairing);
    Ok(OracleDiff {
        report,
        projections,
        dcgo,
        pairing,
    })
}

/// An explicit request must have observations on both paired sides. A missing
/// value (including a non-Main row) is an instrumentation error, never agreement.
pub(super) fn prepare_requested_measurements(
    scenario: &Scenario,
    ours: &mut [StateProjection],
    dcgo: &mut [StateProjection],
    pairing: &StepPairing,
) -> Result<(), String> {
    // Validate the complete mapping before either request validation or field
    // stripping indexes a row. Even an unrequested observation uses this map.
    for &(oi, di) in &pairing.pairs {
        if oi >= ours.len() || di >= dcgo.len() {
            return Err(format!(
                "projection rows do not match pairing: our row {oi} and DCGO row {di} requested, but only {} and {} rows supplied",
                ours.len(), dcgo.len()
            ));
        }
    }
    // Validate local physical binding continuity across at least two paired
    // requested anchors. Native IDs and native wire indices are NOT compared
    // between engines; each side must preserve its own prior public baseline.
    let mut movement_claims = std::collections::BTreeMap::<(usize, String), Vec<u32>>::new();
    for at in 0..=scenario.steps.len() as u32 {
        for (seat, ids) in scenario
            .movement_watch_ids_requested(at)
            .into_iter()
            .enumerate()
        {
            for id in ids {
                movement_claims.entry((seat, id)).or_default().push(at);
            }
        }
    }
    for ((seat, id), anchors) in movement_claims {
        if anchors.len() < 2 {
            return Err(format!(
                "p{seat}.{id}: movement history has no earlier paired public baseline"
            ));
        }
        for (label, rows, use_dcgo) in [("sim", &*ours, false), ("DCGO", &*dcgo, true)] {
            let mut old = None;
            let mut prior_step = None;
            for at in &anchors {
                let (oi, di) = pairing
                    .pairs
                    .iter()
                    .copied()
                    .find(|(oi, _)| ours[*oi].step == *at)
                    .ok_or_else(|| format!("step {at}: movement baseline unpaired"))?;
                let row = &rows[if use_dcgo { di } else { oi }];
                let projected = if seat == 0 { &row.p0 } else { &row.p1 };
                let value = crate::exam::assertions::movement_binding(projected, &id, seat as u8);
                match value {Some(v) if old.map_or(true,|b|b==v) && projected.movement_observation_at.is_some_and(|at|prior_step.map_or(true,|s|s<at))=>{old=Some(v);prior_step=projected.movement_observation_at;},_=>return Err(format!("step {at}: {label} p{seat}.{id} public movement binding missing/reset/rebound"))}
            }
        }
    }
    for step in 0..=scenario.steps.len() {
        let option_requested = scenario.legal_option_plays_requested(step as u32);
        let selection_requested = scenario.selection_can_decline_requested(step as u32);
        let candidates_requested = scenario.selection_candidates_requested(step as u32);
        let movement = scenario.movement_watch_ids_requested(step as u32);
        if !option_requested
            .iter()
            .chain(selection_requested.iter())
            .chain(candidates_requested.iter())
            .any(|b| *b)
            && movement.iter().all(|x| x.is_empty())
        {
            continue;
        }
        let (oi, di) = pairing
            .pairs
            .iter()
            .copied()
            .find(|(oi, _)| ours.get(*oi).map(|p| p.step) == Some(step as u32))
            .ok_or_else(|| {
                format!("step {step}: measurement requested but no paired oracle observation")
            })?;
        for seat in 0..2 {
            let a = if seat == 0 {
                &ours[oi].p0
            } else {
                &ours[oi].p1
            };
            let b = if seat == 0 {
                &dcgo[di].p0
            } else {
                &dcgo[di].p1
            };
            for id in &movement[seat] {
                if !a
                    .movement_observations
                    .as_ref()
                    .is_some_and(|m| m.contains_key(id))
                    || !b
                        .movement_observations
                        .as_ref()
                        .is_some_and(|m| m.contains_key(id))
                {
                    return Err(format!(
                        "step {step}: p{seat}.movement_observations watch {id} unmeasured"
                    ));
                }
            }
            for (field, requested, measured_a, measured_b) in [
                (
                    "legal_option_plays",
                    option_requested[seat],
                    a.legal_option_plays.is_some(),
                    b.legal_option_plays.is_some(),
                ),
                (
                    "selection_candidates",
                    candidates_requested[seat],
                    a.selection_candidates.is_some(),
                    b.selection_candidates.is_some(),
                ),
                (
                    "selection_can_decline",
                    selection_requested[seat],
                    a.selection_can_decline.is_some(),
                    b.selection_can_decline.is_some(),
                ),
            ] {
                if requested && (!measured_a || !measured_b) {
                    return Err(format!("step {step}: p{seat}.{field} requested but unmeasured (ours={measured_a}, DCGO={measured_b}); requires the corresponding live decision for that seat"));
                }
            }
        }
    }
    // New dumpers may emit observations on every supported decision. Only
    // explicitly requested fields participate in comparison and backfill.
    for &(oi, di) in &pairing.pairs {
        let watches = scenario.movement_watch_ids_requested(ours[oi].step);
        for projection in [&mut ours[oi], &mut dcgo[di]] {
            for (seat, row) in [(0, &mut projection.p0), (1, &mut projection.p1)] {
                if watches[seat].is_empty() {
                    row.movement_observations = None;
                } else if let Some(m) = &mut row.movement_observations {
                    m.retain(|id, _| watches[seat].contains(id));
                }
            }
        }
        let options = scenario.legal_option_plays_requested(ours[oi].step);
        let selections = scenario.selection_can_decline_requested(ours[oi].step);
        let candidates = scenario.selection_candidates_requested(ours[oi].step);
        if !candidates[0] {
            ours[oi].p0.selection_candidates = None;
            dcgo[di].p0.selection_candidates = None;
        }
        if !candidates[1] {
            ours[oi].p1.selection_candidates = None;
            dcgo[di].p1.selection_candidates = None;
        }
        if !options[0] {
            ours[oi].p0.legal_option_plays = None;
            dcgo[di].p0.legal_option_plays = None;
        }
        if !options[1] {
            ours[oi].p1.legal_option_plays = None;
            dcgo[di].p1.legal_option_plays = None;
        }
        if !selections[0] {
            ours[oi].p0.selection_can_decline = None;
            dcgo[di].p0.selection_can_decline = None;
        }
        if !selections[1] {
            ours[oi].p1.selection_can_decline = None;
            dcgo[di].p1.selection_can_decline = None;
        }
    }
    Ok(())
}

/// Lower `scenario_text` against the decks recorded beside `sidecar`, align the
/// DCGO trace to the scenario origin, pair by lowered step, and diff -- the
/// whole oracle comparison for one scenario, in one call.
///
/// A line that does not run to completion in our engine is an error: there is
/// nothing comparable to diff.
pub fn diff_against_sidecar(
    scenario_text: &str,
    sidecar: &Path,
    card_data: &HashMap<String, CardData>,
) -> Result<OracleDiff, String> {
    let s = Scenario::from_yaml(scenario_text)?;
    let recording_text = recording_beside(sidecar)?;
    let (p0, p1) = decks_from_recording(&recording_text)?;
    let run = lower_and_run(&s, p0, p1, card_data)?;
    if !run.complete {
        return Err(format!(
            "the line did not run to completion in our engine: {} of {} steps{}",
            run.steps_run,
            run.steps_total,
            if run.stall_reasons.is_empty() {
                String::new()
            } else {
                format!(" ({})", run.stall_reasons.join("; "))
            }
        ));
    }
    diff_lowered(
        &s,
        run.projections,
        s.steps.len(),
        &run.wire_rows_per_step,
        &run.ours_present_per_step,
        sidecar,
        &recording_text,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/oracle")
            .join(name)
    }

    fn scenario_text(stem: &str) -> String {
        std::fs::read_to_string(fixture(&format!("{stem}.yaml"))).unwrap()
    }

    #[test]
    fn a_clean_recorded_run_diffs_clean() {
        let od = diff_against_sidecar(
            &scenario_text("ST23-06-effect0"),
            &fixture("ST23-06-effect0.state.jsonl"),
            &crate::exam::test_support::load_card_data(),
        )
        .unwrap();
        assert!(od.report.is_clean(), "{}", od.report);
        assert_eq!(od.report.compared_steps, 4);
        assert_eq!(od.pairing.pairs.len(), 4);
        // Ours keeps the trailing post-final-step row; DCGO's trace does not.
        assert_eq!(od.projections.len(), 5);
        assert_eq!(od.dcgo.len(), 4);
    }

    #[test]
    fn a_diverged_recorded_run_reports_its_first_divergence() {
        // Preserve the historical evidence on disk. The negative control is
        // an explicitly synthetic memory difference in a temporary copy.
        let work = tempfile::tempdir().unwrap();
        let sidecar = work.path().join("ST23-04-effect0.state.jsonl");
        std::fs::copy(fixture("ST23-04-effect0.jsonl"), work.path().join("ST23-04-effect0.jsonl")).unwrap();
        let original = std::fs::read_to_string(fixture("ST23-04-effect0.state.jsonl")).unwrap();
        std::fs::write(&sidecar, crate::exam::test_support::with_synthetic_st23_memory_divergence(&original)).unwrap();
        let od = diff_against_sidecar(
            &scenario_text("ST23-04-effect0"),
            &sidecar,
            &crate::exam::test_support::load_card_data(),
        )
        .unwrap();
        assert!(!od.report.is_clean());
        let first = od.report.first().expect("synthetic memory divergence");
        assert_eq!(first.step, 12);
        assert_eq!(first.diffs[0].path, "memory");
        assert_eq!(first.diffs[0].ours, "-3");
        assert_eq!(first.diffs[0].dcgo, "-2");
    }

    #[test]
    fn a_sidecar_with_no_recording_beside_it_is_named() {
        let err = diff_against_sidecar(
            &scenario_text("ST23-06-effect0"),
            &fixture("no-such-run.state.jsonl"),
            &crate::exam::test_support::load_card_data(),
        )
        .err()
        .expect("no recording, no diff");
        assert!(err.contains("no-such-run.jsonl"), "got: {err}");
    }

    #[test]
    fn decks_come_from_the_recordings_own_shuffle_reversed_with_eggs_last() {
        let text = std::fs::read_to_string(fixture("ST23-06-effect0.jsonl")).unwrap();
        let (p0, _p1) = decks_from_recording(&text).unwrap();
        let start: serde_json::Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
        let top_first: Vec<&str> = start["my_deck_post_shuffle"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        let eggs = start["my_egg_deck"]
            .as_array()
            .map(|a| a.len())
            .unwrap_or(0);
        // DCGO records top-first; our engine draws from the back, eggs last.
        assert_eq!(p0.len(), top_first.len() + eggs);
        assert_eq!(p0[top_first.len() - 1], top_first[0]);
        assert_eq!(p0[0], *top_first.last().unwrap());
    }
}

#[cfg(test)]
mod legal_option_measurement_oracle_tests {
    use super::*;
    use crate::exam::projection::pair_by_wire_rows;
    fn scenario(requested: bool) -> Scenario {
        Scenario::from_yaml(&format!("card: ST1-16\nclause: ST1-16#effect#0\nseed: 1\ndecks:\n  p0: {{rest: x}}\n  p1: {{rest: x}}\nsteps:\n  - actor: 0\n    do: {{pass: {{}}}}\n{}", if requested {"expect_ruling:\n  q_id: Q1\n  assert:\n    - at: 0\n      that: {p0.legal_option_plays: []}\ninteraction: {id: 'qa:Q1', source: qa, kind: positive}\n"} else {""})).unwrap()
    }
    fn row(measured: bool) -> StateProjection {
        StateProjection::from_sidecar_line(&format!(r#"{{"step":0,"turn":1,"phase":"Main","memory":3,"p0":{{"security":5,"hand":[],"trash":[],"field":[]{}}},"p1":{{"security":5,"hand":[],"trash":[],"field":[]}}}}"#, if measured {r#", "legal_option_plays": []"#} else {""})).unwrap()
    }
    #[test]
    fn legal_option_measurement_requested_missing_either_side_fails_closed() {
        for (a, b) in [(false, false), (true, false), (false, true)] {
            let error = prepare_requested_measurements(
                &scenario(true),
                &mut [row(a)],
                &mut [row(b)],
                &pair_by_wire_rows(&[1], 1),
            )
            .unwrap_err();
            assert!(error.contains("unmeasured"), "{error}");
        }
        assert!(prepare_requested_measurements(
            &scenario(true),
            &mut [row(true)],
            &mut [row(true)],
            &pair_by_wire_rows(&[1], 1)
        )
        .is_ok());
    }
    #[test]
    fn legal_option_measurement_unrequested_new_and_old_sidecars_stay_compatible() {
        let mut ours = [row(false)];
        let mut dcgo = [row(true)];
        prepare_requested_measurements(
            &scenario(false),
            &mut ours,
            &mut dcgo,
            &pair_by_wire_rows(&[1], 1),
        )
        .unwrap();
        assert!(dcgo[0].p0.legal_option_plays.is_none());
        assert!(crate::exam::differ::diff(&ours, &dcgo).is_clean());
    }
    #[test]
    fn legal_option_measurement_requested_unpaired_row_is_not_confirmable() {
        let error = prepare_requested_measurements(
            &scenario(true),
            &mut [row(true)],
            &mut [],
            &pair_by_wire_rows(&[0], 0),
        )
        .unwrap_err();
        assert!(error.contains("no paired oracle observation"));
    }
}

#[cfg(test)]
mod selection_decline_observation_oracle_tests {
    use super::*;
    use crate::exam::projection::pair_by_wire_rows;

    fn scenario(request: &str) -> Scenario {
        Scenario::from_yaml(&format!("card: EX3-037\nclause: EX3-037#effect#1\nseed: 1\ndecks:\n  p0: {{rest: x}}\n  p1: {{rest: x}}\nsteps:\n  - actor: 0\n    do: {{pass: {{}}}}\n{request}")).unwrap()
    }
    const ASSERT_REQUEST: &str =
        "assert:\n  - at: 0\n    that: {p0.selection_can_decline: false}\n";
    const RULING_REQUEST: &str = "interaction: {id: 'qa:Q3414', source: qa, kind: positive}\nexpect_ruling:\n  q_id: Q3414\n  assert:\n    - at: 0\n      that: {p0.selection_can_decline: false}\n";
    fn row(measured: bool) -> StateProjection {
        StateProjection::from_sidecar_line(&format!(r#"{{"step":0,"turn":1,"phase":"Main","memory":3,"p0":{{"security":5,"hand":[],"trash":[],"field":[]{}}},"p1":{{"security":5,"hand":[],"trash":[],"field":[]}}}}"#, if measured {r#", "selection_can_decline": false"#} else {""})).unwrap()
    }
    #[test]
    fn selection_decline_observation_requested_missing_either_side_fails_closed() {
        for request in [ASSERT_REQUEST, RULING_REQUEST] {
            for (a, b) in [(false, false), (false, true), (true, false)] {
                let result = prepare_requested_measurements(
                    &scenario(request),
                    &mut [row(a)],
                    &mut [row(b)],
                    &pair_by_wire_rows(&[1], 1),
                );
                assert!(
                    result.is_err(),
                    "request={request}, measured ours={a} DCGO={b}"
                );
            }
        }
    }
    #[test]
    fn requested_measurements_reject_malformed_pairings_before_mutating_rows() {
        for request in ["", ASSERT_REQUEST, RULING_REQUEST] {
            for bad_pair in [(1, 0), (0, 1), (1, 1)] {
                let mut ours = [row(true)];
                let mut dcgo = [row(true)];
                let original_ours = ours.clone();
                let original_dcgo = dcgo.clone();
                let mut pairing = pair_by_wire_rows(&[1], 1);
                // A valid row precedes the malformed row: validation must be
                // atomic, so it cannot strip observations before finding it.
                pairing.pairs.push(bad_pair);
                let error = prepare_requested_measurements(
                    &scenario(request),
                    &mut ours,
                    &mut dcgo,
                    &pairing,
                )
                .unwrap_err();
                assert!(error.contains("do not match"), "{error}");
                assert_eq!(ours, original_ours);
                assert_eq!(dcgo, original_dcgo);
            }
        }
    }
    #[test]
    fn selection_decline_observation_requested_unpaired_row_fails_closed() {
        let result = prepare_requested_measurements(
            &scenario(ASSERT_REQUEST),
            &mut [row(true)],
            &mut [],
            &pair_by_wire_rows(&[0], 0),
        );
        assert!(result.is_err());
    }
    #[test]
    fn selection_decline_observation_requested_measurements_survive_pairing() {
        let mut ours = [row(true)];
        let mut dcgo = [row(true)];
        prepare_requested_measurements(
            &scenario(RULING_REQUEST),
            &mut ours,
            &mut dcgo,
            &pair_by_wire_rows(&[1], 1),
        )
        .unwrap();
        for projection in [&ours[0], &dcgo[0]] {
            let json = serde_json::to_value(projection).unwrap();
            assert_eq!(
                json["p0"]["selection_can_decline"],
                serde_json::json!(false)
            );
        }
    }
    #[test]
    fn selection_decline_observation_unrequested_old_sidecar_stays_compatible() {
        let mut ours = [row(false)];
        let mut dcgo = [row(true)];
        prepare_requested_measurements(
            &scenario(""),
            &mut ours,
            &mut dcgo,
            &pair_by_wire_rows(&[1], 1),
        )
        .unwrap();
        assert!(crate::exam::differ::diff(&ours, &dcgo).is_clean());
        assert!(serde_json::to_value(&dcgo[0]).unwrap()["p0"]
            .get("selection_can_decline")
            .is_none());
    }
}

#[cfg(test)]
mod selection_candidate_oracle_tests {
    use super::*;
    use crate::exam::projection::pair_by_wire_rows;
    fn scenario(ruling: bool, requested: bool) -> Scenario {
        let mut text="card: LM-027\nclause: LM-027#effect#0\nseed: 1\ndecks:\n  p0: {rest: x}\n  p1: {rest: x}\nsteps:\n  - actor: 0\n    do: {pass: {}}\n".to_string();
        if requested {
            if ruling {
                text.push_str("interaction: {id: 'qa:Q4033', source: qa, kind: positive}\nexpect_ruling:\n  q_id: Q4033\n  assert:\n  - at: 0\n    that: {p0.selection_candidates: [ST1-05]}\n");
            } else {
                text.push_str(
                    "assert:\n  - at: 0\n    that: {p0.selection_candidates: [ST1-05]}\n",
                );
            }
        }
        Scenario::from_yaml(&text).unwrap()
    }
    fn row(measured: bool) -> StateProjection {
        StateProjection::from_sidecar_line(&format!(r#"{{"step":0,"turn":1,"phase":"SelectHand","memory":1,"p0":{{"security":5,"hand":[],"trash":[],"field":[]{}}},"p1":{{"security":5,"hand":[],"trash":[],"field":[]}}}}"#,if measured {r#", "selection_candidates": ["ST1-05"]"#}else{""})).unwrap()
    }
    #[test]
    fn selection_candidates_requests_from_assert_or_ruling_require_both_measurements() {
        for ruling in [false, true] {
            let s = scenario(ruling, true);
            assert_eq!(s.selection_candidates_requested(0), [true, false]);
            for (a, b) in [(false, false), (true, false), (false, true)] {
                assert!(prepare_requested_measurements(
                    &s,
                    &mut [row(a)],
                    &mut [row(b)],
                    &pair_by_wire_rows(&[1], 1)
                )
                .is_err());
            }
            let mut a = [row(true)];
            let mut b = [row(true)];
            prepare_requested_measurements(&s, &mut a, &mut b, &pair_by_wire_rows(&[1], 1))
                .unwrap();
            assert_eq!(a[0].p0.selection_candidates, Some(vec!["ST1-05".into()]));
        }
    }
    #[test]
    fn selection_candidates_unrequested_fields_are_stripped_and_unpaired_requests_fail() {
        let mut a = [row(true)];
        let mut b = [row(true)];
        prepare_requested_measurements(
            &scenario(false, false),
            &mut a,
            &mut b,
            &pair_by_wire_rows(&[1], 1),
        )
        .unwrap();
        assert!(a[0].p0.selection_candidates.is_none() && b[0].p0.selection_candidates.is_none());
        assert!(prepare_requested_measurements(
            &scenario(true, true),
            &mut [row(true)],
            &mut [],
            &pair_by_wire_rows(&[0], 0)
        )
        .is_err());
    }
}

#[cfg(test)]
mod movement_native_anchor_tests {
    use super::*;
    fn row(step: u32, bind: u32, native: u64) -> StateProjection {
        serde_json::from_value(serde_json::json!({"step":step,"turn":1,"phase":"main","memory":3,
          "p0":{"security":5,"hand":[],"trash":[],"field":[],"movement_observation_at":step,
            "movement_observations":{"X":{"version":1,"deletion_dispatches":0,"rule_trashes":[]}},
            "movement_audit":[{"owner":0,"card_id":"X","native_top":native,"native_lower":[],"native_linked":[],"kind":"public_watch_bound","binding_start":bind}]},
          "p1":{"security":5,"hand":[],"trash":[],"field":[]}})).unwrap()
    }
    #[test]
    fn two_mulligans_and_folded_wire_rows_preserve_native_interval() {
        let s=Scenario::from_yaml("card: BT9-109\nclause: BT9-109#inherited#0\nseed: 1\ndecks:\n  p0: {rest: x}\n  p1: {rest: x}\nsteps:\n  - {actor: 0, do: {pass: {}}}\n  - {actor: 1, do: {pass: {}}}\n  - {actor: 0, do: {pass: {}}}\nassert:\n  - at: 0\n    that: {p0.movement_observations: {X: {version: 1, deletion_dispatches: 0, rule_trashes: []}}}\n  - at: 2\n    that: {p0.movement_observations: {X: {version: 1, deletion_dispatches: 0, rule_trashes: []}}}\n").unwrap();
        let recording="{\"type\":\"action\",\"source\":\"mulligan\",\"step\":12}\n{\"type\":\"action\",\"source\":\"mulligan\",\"step\":13}\n";
        let mut dcgo =
            align_to_scenario_origin((12..18).map(|s| row(s, 14, 91)).collect(), recording)
                .unwrap();
        assert_eq!(dcgo[0].step, 0);
        assert_eq!(dcgo[0].p0.movement_observation_at, Some(14));
        let mut ours: Vec<_> = (0..3).map(|s| row(s, 0, 11)).collect();
        let pairing = StepPairing {
            pairs: vec![(0, 0), (1, 2), (2, 3)],
            ours_unpairable: 0,
            dcgo_unpairable: 1,
        };
        assert!(prepare_requested_measurements(&s, &mut ours, &mut dcgo, &pairing).is_ok());
        dcgo[3].p0.movement_observation_at = None;
        assert!(prepare_requested_measurements(&s, &mut ours, &mut dcgo, &pairing).is_err());
    }
}
