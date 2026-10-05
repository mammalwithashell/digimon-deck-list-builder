//! One oracle round-trip for one job: submit, wait for THIS job's result,
//! locate its sidecar. Never "newest recording" -- several agents share a queue.
//!
//! What the result MEANS is the caller's call: DCGO files a failed exam job
//! (a prompt mismatch, say) with its recording and a partial sidecar, so only
//! the cases where no result exists at all -- a timeout, or a job the sweep
//! quarantined into `failed/` -- are errors here.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::exam::differ::DiffReport;
use crate::exam::scenario::Scenario;
use crate::exam::verdict::{
    record_interaction_verdict, record_scenario_verdict, sha256_hex, ClauseTextBook,
    InteractionBook, Verdict, VerdictStore,
};
use crate::job::{JobOutcome, JobResult, DIR_DONE, DIR_FAILED, DIR_JOBS};

/// What DCGO filed for one submitted job.
#[derive(Debug, Clone)]
pub struct OracleRun {
    pub job_id: String,
    pub outcome: JobOutcome,
    /// DCGO's failure detail; empty on success.
    pub message: String,
    /// `None` when DCGO filed no recording (it failed before the game began).
    pub recording_path: Option<PathBuf>,
    /// The `.state.jsonl` beside the recording, when there is a recording.
    pub sidecar_path: Option<PathBuf>,
}

/// Write `job_json` to `<root>/jobs/<job_id>.json` (temp file + rename), after
/// removing any stale `done/<job_id>.json` / `done/<job_id>.result.json` /
/// `failed/<job_id>.json`; poll for this job's own result until `timeout`.
///
/// On a timeout the job is withdrawn from `jobs/` if no player claimed it, so
/// a player started later does not run a job nobody is waiting for.
pub fn submit_and_wait(
    root: &Path,
    job_id: &str,
    job_json: &str,
    timeout: Duration,
    poll: Duration,
) -> Result<OracleRun, String> {
    let done = root.join(DIR_DONE);
    let failed = root.join(DIR_FAILED).join(format!("{job_id}.json"));
    let result_path = done.join(format!("{job_id}.result.json"));
    for stale in [done.join(format!("{job_id}.json")), result_path.clone(), failed.clone()] {
        if stale.exists() {
            std::fs::remove_file(&stale)
                .map_err(|e| format!("removing stale {}: {e}", stale.display()))?;
        }
    }
    let jobs = root.join(DIR_JOBS);
    std::fs::create_dir_all(&jobs).map_err(|e| format!("creating {}: {e}", jobs.display()))?;
    let tmp = jobs.join(format!(".{job_id}.json.tmp"));
    let dest = jobs.join(format!("{job_id}.json"));
    std::fs::write(&tmp, job_json).map_err(|e| format!("writing {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, &dest).map_err(|e| format!("submitting {}: {e}", dest.display()))?;

    let started = Instant::now();
    loop {
        if result_path.exists() {
            let text = std::fs::read_to_string(&result_path)
                .map_err(|e| format!("reading {}: {e}", result_path.display()))?;
            let result = JobResult::from_json(&text)
                .map_err(|e| format!("parsing {}: {e}", result_path.display()))?;
            let recording = result.recording_path.trim();
            let (recording_path, sidecar_path) = if recording.is_empty() {
                (None, None)
            } else {
                let stem = recording.strip_suffix(".jsonl").unwrap_or(recording);
                (Some(PathBuf::from(recording)), Some(PathBuf::from(format!("{stem}.state.jsonl"))))
            };
            return Ok(OracleRun {
                job_id: job_id.to_string(),
                outcome: result.outcome,
                message: result.message,
                recording_path,
                sidecar_path,
            });
        }
        if failed.exists() {
            return Err(format!(
                "oracle job {job_id} was quarantined into failed/ (it timed out on the \
                 player repeatedly); no result was filed"
            ));
        }
        if started.elapsed() >= timeout {
            let withdrawn = dest.exists() && std::fs::remove_file(&dest).is_ok();
            return Err(format!(
                "oracle job {job_id} timed out after {}s{} (is the player running? `node status`)",
                timeout.as_secs_f64().round() as u64,
                if withdrawn { "; no player claimed it, so it was withdrawn from jobs/" } else { "" }
            ));
        }
        std::thread::sleep(poll);
    }
}

// ---------------------------------------------------------------------------
// Verdicts: what one oracle run established, and filing it in the store.
// Shared by `exam --sidecar --verdicts` and `exam --oracle`, so the two routes
// can never disagree about what a run confirms or where it is filed.
// ---------------------------------------------------------------------------

/// What one oracle-mode scenario run established about its clauses (or its
/// interaction), before the verdict store gets involved.
#[derive(Debug, Clone)]
pub struct VerdictEvent {
    /// Every clause the line covers (`covers:`, default `[clause]`). A clause
    /// scenario's verdict is recorded for each.
    pub clause_ids: Vec<String>,
    /// Set on an interaction exam: the verdict is filed under this id INSTEAD
    /// of any clause -- a passing negative probe proves a clause did NOT fire,
    /// which must never read as that clause confirmed.
    pub interaction_id: Option<String>,
    pub card: String,
    pub verdict: Verdict,
    pub reason: Option<String>,
    pub scenario_path: String,
}

impl VerdictEvent {
    /// Classify one diffed run. A CLEAN oracle diff confirms; anything else
    /// (divergence or truncation) is a finding to triage. A Q&A exam also
    /// needs our engine to agree with the ruling (`ruling_ok`, `None` when the
    /// scenario carries none): ours = DCGO but not the ruling is `ours_wrong`,
    /// never `confirmed` (card-loop design D7).
    pub fn from_diff(
        s: &Scenario,
        report: &DiffReport,
        ruling_ok: Option<bool>,
        scenario_path: String,
    ) -> VerdictEvent {
        let clean = report.is_clean() && ruling_ok != Some(false);
        let reason = if clean {
            None
        } else {
            let lead = format!("{report}").lines().next().unwrap_or_default().to_string();
            Some(match ruling_ok {
                Some(false) => format!(
                    "ours contradicts ruling qa:{} (ours vs DCGO: {}); {lead}",
                    s.expect_ruling.as_ref().map(|r| r.q_id.as_str()).unwrap_or_default(),
                    if report.is_clean() { "agree" } else { "diverge" }
                ),
                _ => lead,
            })
        };
        VerdictEvent {
            clause_ids: s.covered_clauses(),
            interaction_id: s.interaction.as_ref().map(|i| i.id.clone()),
            card: s.card.clone(),
            verdict: if clean { Verdict::Confirmed } else { Verdict::Diverged },
            reason,
            scenario_path,
        }
    }

    /// The ids this event is filed under: the interaction id instead of any
    /// clause, else every covered clause.
    pub fn ids(&self) -> Vec<String> {
        match &self.interaction_id {
            Some(id) => vec![id.clone()],
            None => self.clause_ids.clone(),
        }
    }
}

/// Files [`VerdictEvent`]s into a verdict store, refusing any id outside its
/// denominator (the orphan rule): clause ids against the clause-text book,
/// interaction ids against the interaction denominator (loaded on first use,
/// so a clause-only run never needs it).
pub struct VerdictRecorder {
    pub store: VerdictStore,
    clause_book: ClauseTextBook,
    interaction_denominator: PathBuf,
    interaction_book: Option<Result<InteractionBook, String>>,
}

impl VerdictRecorder {
    pub fn load(
        store_dir: &Path,
        clause_text_json: &Path,
        interaction_denominator: &Path,
    ) -> Result<VerdictRecorder, String> {
        let clause_book = ClauseTextBook::load(clause_text_json)?;
        let store = VerdictStore::load_dir(store_dir)?;
        Ok(VerdictRecorder {
            store,
            clause_book,
            interaction_denominator: interaction_denominator.to_path_buf(),
            interaction_book: None,
        })
    }

    /// Record one event: one entry per id it is filed under, `Ok(id)` when
    /// recorded and `Err(why)` when refused.
    pub fn record(&mut self, ev: &VerdictEvent, recorded_at: &str) -> Vec<Result<String, String>> {
        let mut out = Vec::new();
        for id in ev.ids() {
            let result = if ev.interaction_id.is_some() {
                let denominator = &self.interaction_denominator;
                match self
                    .interaction_book
                    .get_or_insert_with(|| InteractionBook::load(denominator))
                {
                    Ok(ib) => record_interaction_verdict(
                        &mut self.store,
                        ib,
                        &id,
                        std::slice::from_ref(&ev.card),
                        ev.verdict,
                        Some(ev.scenario_path.clone()),
                        ev.reason.clone(),
                        recorded_at.to_string(),
                    ),
                    Err(e) => Err(format!("refusing interaction {id}: {e}")),
                }
            } else {
                record_scenario_verdict(
                    &mut self.store,
                    &self.clause_book,
                    &id,
                    ev.verdict,
                    Some(ev.scenario_path.clone()),
                    ev.reason.clone(),
                    recorded_at.to_string(),
                )
            };
            out.push(result.map(|()| id));
        }
        out
    }

    /// Tell the store what every clause's text hashes to right now (so stale
    /// verdicts report as invalidated), write the touched card files, and
    /// return the store summary over the clause-text denominator.
    pub fn save(&mut self, dir: &Path) -> Result<String, String> {
        let all_ids = self.clause_book.clause_ids();
        for id in &all_ids {
            if let Some(ct) = self.clause_book.get(id) {
                self.store.set_current_text_sha(id, &sha256_hex(&ct.text));
            }
        }
        self.store.save_dir(dir)?;
        Ok(self.store.summary(&all_ids).describe())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn root() -> tempfile::TempDir {
        let t = tempfile::tempdir().unwrap();
        for d in ["jobs", "claimed", "done", "failed"] {
            std::fs::create_dir_all(t.path().join(d)).unwrap();
        }
        t
    }

    fn write_result(root: &std::path::Path, job: &str, outcome: &str, rec: &std::path::Path, message: &str) {
        let body = serde_json::json!({
            "job_id": job, "outcome": outcome,
            "recording_path": rec.display().to_string(),
            "steps": 4, "duration_seconds": 1.0, "message": message
        });
        std::fs::write(root.join("done").join(format!("{job}.result.json")), body.to_string()).unwrap();
    }

    #[test]
    fn completed_result_yields_the_sidecar_beside_the_recording() {
        let t = root();
        let rec = t.path().join("rec.jsonl");
        std::fs::write(&rec, "").unwrap();
        std::fs::write(t.path().join("rec.state.jsonl"), "").unwrap();
        let r = t.path().to_path_buf();
        let rec2 = rec.clone();
        let h = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            write_result(&r, "exam-X-effect0", "completed", &rec2, "");
        });
        let run = submit_and_wait(t.path(), "exam-X-effect0", "{}", Duration::from_secs(5),
                                  Duration::from_millis(20)).unwrap();
        h.join().unwrap();
        assert_eq!(run.outcome, JobOutcome::Completed);
        assert_eq!(run.sidecar_path, Some(t.path().join("rec.state.jsonl")));
        assert!(t.path().join("jobs/exam-X-effect0.json").exists());
    }

    #[test]
    fn a_stale_result_is_removed_before_submitting() {
        let t = root();
        let rec = t.path().join("old.jsonl");
        write_result(t.path(), "exam-X-effect0", "completed", &rec, "");
        // A leftover quarantine of the same id must not fail the new job either.
        std::fs::write(t.path().join("failed/exam-X-effect0.json"), "{}").unwrap();
        let err = submit_and_wait(t.path(), "exam-X-effect0", "{}", Duration::from_millis(200),
                                  Duration::from_millis(20)).unwrap_err();
        assert!(err.contains("timed out"), "stale result must not be returned; got {err}");
    }

    #[test]
    fn a_failed_outcome_is_returned_with_its_message() {
        // DCGO files a failed exam job (prompt mismatch) WITH its recording:
        // the caller decides what it measured, so it is not an Err here.
        let t = root();
        let r = t.path().to_path_buf();
        let h = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            write_result(&r, "exam-Y-effect1", "failed", std::path::Path::new("x.jsonl"),
                         "prompt mismatch: step 3 expected prompt 'main_phase' but DCGO asked 'OptionalSkill'");
        });
        let run = submit_and_wait(t.path(), "exam-Y-effect1", "{}", Duration::from_secs(5),
                                  Duration::from_millis(20)).unwrap();
        h.join().unwrap();
        assert_eq!(run.outcome, JobOutcome::Failed);
        assert!(run.message.contains("prompt mismatch"), "got {}", run.message);
        assert_eq!(run.sidecar_path, Some(std::path::PathBuf::from("x.state.jsonl")));
    }

    #[test]
    fn a_job_quarantined_into_failed_is_an_error_naming_it() {
        let t = root();
        let r = t.path().to_path_buf();
        let h = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            std::fs::write(r.join("failed/exam-Z-effect2.json"), "{}").unwrap();
        });
        let err = submit_and_wait(t.path(), "exam-Z-effect2", "{}", Duration::from_secs(5),
                                  Duration::from_millis(20)).unwrap_err();
        h.join().unwrap();
        assert!(err.contains("exam-Z-effect2") && err.contains("quarantined"), "got {err}");
    }

    #[test]
    fn a_timed_out_job_no_player_claimed_is_withdrawn() {
        let t = root();
        let err = submit_and_wait(t.path(), "exam-W-effect0", "{}", Duration::from_millis(100),
                                  Duration::from_millis(20)).unwrap_err();
        assert!(err.contains("exam-W-effect0") && err.contains("timed out"), "got {err}");
        assert!(!t.path().join("jobs/exam-W-effect0.json").exists(),
                "an unclaimed job must not linger for a later player to run");
    }

    #[test]
    fn an_empty_recording_path_yields_no_sidecar() {
        let t = root();
        let r = t.path().to_path_buf();
        let h = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            write_result(&r, "exam-V-effect0", "failed", std::path::Path::new(""), "bad deck");
        });
        let run = submit_and_wait(t.path(), "exam-V-effect0", "{}", Duration::from_secs(5),
                                  Duration::from_millis(20)).unwrap();
        h.join().unwrap();
        assert_eq!(run.recording_path, None);
        assert_eq!(run.sidecar_path, None);
    }
}

#[cfg(test)]
mod verdict_event_tests {
    use super::*;
    use crate::exam::differ::{DiffReport, FieldDiff, StepDivergence};
    use crate::exam::scenario::Scenario;
    use crate::exam::verdict::Verdict;

    const BASE: &str = "card: BT7-056\nclause: BT7-056#effect#0\nseed: 7\ndecks:\n  p0: { stack: [BT7-056], rest: vb-standard }\n  p1: { stack: [], rest: vb-standard }\nsteps:\n  - actor: 0\n    do: { hatch: {} }\n  - actor: 0\n    do: { pass: {} }\n";

    fn scenario(extra: &str) -> Scenario {
        Scenario::from_yaml(&format!("{BASE}{extra}")).unwrap()
    }

    fn clean() -> DiffReport {
        DiffReport { compared_steps: 2, ours_steps: 2, dcgo_steps: 2, ours_unpairable: 0, dcgo_unpairable: 0, divergences: vec![] }
    }

    fn diverged() -> DiffReport {
        let mut r = clean();
        r.divergences.push(StepDivergence {
            step: 1,
            diffs: vec![FieldDiff { path: "memory".into(), ours: "3".into(), dcgo: "2".into() }],
            downstream: false,
        });
        r
    }

    #[test]
    fn a_clean_diff_confirms_every_covered_clause() {
        let s = scenario("covers: [BT7-056#effect#0, BT7-056#inherited#0]\n");
        let ev = VerdictEvent::from_diff(&s, &clean(), None, "x.yaml".into());
        assert_eq!(ev.verdict, Verdict::Confirmed);
        assert_eq!(ev.reason, None);
        assert_eq!(ev.ids(), vec!["BT7-056#effect#0".to_string(), "BT7-056#inherited#0".to_string()]);
    }

    #[test]
    fn a_divergence_is_diverged_with_the_reports_lead_line() {
        let ev = VerdictEvent::from_diff(&scenario(""), &diverged(), None, "x.yaml".into());
        assert_eq!(ev.verdict, Verdict::Diverged);
        let lead = format!("{}", diverged()).lines().next().unwrap().to_string();
        assert_eq!(ev.reason.as_deref(), Some(lead.as_str()));
    }

    #[test]
    fn contradicting_the_ruling_is_diverged_even_when_dcgo_agrees() {
        let s = scenario("interaction: { id: \"qa:Q1601\", source: qa, kind: positive }\nexpect_ruling: { q_id: Q1601, assert: [ { at: 1, that: { memory: 0 } } ] }\n");
        let ev = VerdictEvent::from_diff(&s, &clean(), Some(false), "x.yaml".into());
        assert_eq!(ev.verdict, Verdict::Diverged);
        let reason = ev.reason.unwrap();
        assert!(reason.starts_with("ours contradicts ruling qa:Q1601 (ours vs DCGO: agree)"), "{reason}");
    }

    #[test]
    fn an_interaction_is_filed_under_its_id_instead_of_any_clause() {
        let s = scenario("interaction: { id: \"probe:BT7-056#effect#0:scope:neg\", source: probe, kind: negative }\n");
        let ev = VerdictEvent::from_diff(&s, &clean(), None, "x.yaml".into());
        assert_eq!(ev.ids(), vec!["probe:BT7-056#effect#0:scope:neg".to_string()]);
    }

    fn recorder(dir: &std::path::Path) -> VerdictRecorder {
        let book = dir.join("clauses.json");
        std::fs::write(&book, r#"{"clauses":[{"id":"BT7-056#effect#0","label":"[On Play]","text":"t"}]}"#).unwrap();
        VerdictRecorder::load(&dir.join("store"), &book, &dir.join("no-denominator.json")).unwrap()
    }

    #[test]
    fn the_recorder_files_known_clauses_and_refuses_orphans() {
        let dir = tempfile::tempdir().unwrap();
        let mut rec = recorder(dir.path());
        let s = scenario("covers: [BT7-056#effect#0, BT7-056#inherited#0]\n");
        let ev = VerdictEvent::from_diff(&s, &clean(), None, "x.yaml".into());
        let out = rec.record(&ev, "2026-10-05T00:00:00Z");
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].as_ref().unwrap(), "BT7-056#effect#0");
        assert!(out[1].as_ref().unwrap_err().contains("refusing"), "{:?}", out[1]);
        let summary = rec.save(&dir.path().join("store")).unwrap();
        assert!(summary.contains("1/1 confirmed"), "{summary}");
    }

    #[test]
    fn an_interaction_without_a_denominator_is_refused_by_name() {
        let dir = tempfile::tempdir().unwrap();
        let mut rec = recorder(dir.path());
        let s = scenario("interaction: { id: \"qa:Q1601\", source: qa, kind: positive }\n");
        let ev = VerdictEvent::from_diff(&s, &clean(), None, "x.yaml".into());
        let out = rec.record(&ev, "2026-10-05T00:00:00Z");
        assert!(out[0].as_ref().unwrap_err().starts_with("refusing interaction qa:Q1601:"), "{:?}", out[0]);
    }
}
