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

    Ok(if my_id == 0 { (mine, theirs) } else { (theirs, mine) })
}

/// Diff an already-lowered line (its projections and pairing inputs) against
/// the DCGO sidecar at `sidecar_path`, whose recording text is given.
pub fn diff_lowered(
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
    let dcgo = align_to_scenario_origin(dcgo, &recording_text)?;
    // Compare "state at each decision point" on both sides. Ours holds one
    // projection BEFORE each step plus a trailing one AFTER the last, while
    // StateDumper writes one before each decision and stops. Dropping our
    // trailing entry aligns the two ends; keeping it made the differ report
    // TRUNCATED (correctly -- it refuses to call an unequal comparison clean)
    // on a run that had no divergence at all.
    //
    // `projections` itself keeps the trailing state, because an `assert:` block
    // legitimately wants to talk about the position AFTER the final step.
    let ours_for_diff: Vec<_> = projections
        .iter()
        .take(steps)
        .cloned()
        .collect();
    // ...and pair the two by LOWERED STEP rather than 1:1, because a step can
    // consume one, two, or ZERO DCGO decision rows (an OptionalSkill+pick fold
    // writes two; a sim-only phase exit writes none). A positional pairing
    // slides apart from the first multi-row step onward and manufactures
    // divergences out of the offset -- measured on EX12-011#effect#0, where our
    // post-battle row was compared against DCGO's pre-battle mid-fold row.
    let pairing =
        pair_by_wire_rows_with_ownership(wire_rows_per_step, ours_present_per_step, dcgo.len());
    let report = diff_paired(&ours_for_diff, &dcgo, &pairing);
    Ok(OracleDiff { report, projections, dcgo, pairing })
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
        let od = diff_against_sidecar(
            &scenario_text("ST23-04-effect0"),
            &fixture("ST23-04-effect0.state.jsonl"),
            &crate::exam::test_support::load_card_data(),
        )
        .unwrap();
        assert!(!od.report.is_clean());
        assert_eq!(od.report.first().map(|d| d.step), Some(14));
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
        let start: serde_json::Value =
            serde_json::from_str(text.lines().next().unwrap()).unwrap();
        let top_first: Vec<&str> = start["my_deck_post_shuffle"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        let eggs = start["my_egg_deck"].as_array().map(|a| a.len()).unwrap_or(0);
        // DCGO records top-first; our engine draws from the back, eggs last.
        assert_eq!(p0.len(), top_first.len() + eggs);
        assert_eq!(p0[top_first.len() - 1], top_first[0]);
        assert_eq!(p0[0], *top_first.last().unwrap());
    }
}
