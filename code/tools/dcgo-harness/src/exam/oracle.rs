//! One oracle round-trip for one job: submit, wait for THIS job's result,
//! locate its sidecar. Never "newest recording" -- several agents share a queue.
//!
//! What the result MEANS is the caller's call: DCGO files a failed exam job
//! (a prompt mismatch, say) with its recording and a partial sidecar, so only
//! the cases where no result exists at all -- a timeout, or a job the sweep
//! quarantined into `failed/` -- are errors here.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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
