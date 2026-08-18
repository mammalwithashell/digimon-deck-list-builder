//! Queue state: how many jobs are pending, running, done, or dead — and what
//! to do about a claim that never came back.

use std::path::Path;
use std::time::SystemTime;

use crate::job::{JobOutcome, JobResult, DIR_CLAIMED, DIR_DONE, DIR_FAILED, DIR_JOBS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimVerdict {
    /// Still within its budget; leave it alone.
    Running,
    /// Overdue — move back to jobs/ and let DCGO try again.
    Requeue,
    /// Overdue and already failed twice. Park it in failed/ permanently.
    Quarantine,
}

/// Decide the fate of a claimed job.
///
/// The quarantine rule is the important one: a job that kills Unity will kill
/// it again, and an unbounded retry loop turns one poisonous job into a
/// silently stalled batch.
pub fn classify_claimed(age_seconds: u64, timeout_seconds: u64, prior_failures: u32) -> ClaimVerdict {
    if age_seconds <= timeout_seconds {
        return ClaimVerdict::Running;
    }
    if prior_failures >= 2 {
        ClaimVerdict::Quarantine
    } else {
        ClaimVerdict::Requeue
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct QueueStatus {
    pub pending: usize,
    pub claimed: usize,
    pub completed: usize,
    pub partial: usize,
    pub failed: usize,
}

impl QueueStatus {
    /// One line naming every bucket. Printed by `status` AND by `triage`, so a
    /// batch where most jobs died can never be mistaken for a clean run.
    pub fn summary(&self) -> String {
        format!(
            "pending={} claimed={} completed={} partial={} failed={}",
            self.pending, self.claimed, self.completed, self.partial, self.failed
        )
    }

    /// A clean run needs actual completed games. Zero completions is never
    /// clean, however few failures there were.
    pub fn is_clean(&self) -> bool {
        self.completed > 0 && self.failed == 0
    }
}

fn count_files(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
                .count()
        })
        .unwrap_or(0)
}

/// Tally the queue by reading the four directories.
pub fn scan(root: &Path) -> Result<QueueStatus, String> {
    let mut status = QueueStatus {
        pending: count_files(&root.join(DIR_JOBS)),
        claimed: count_files(&root.join(DIR_CLAIMED)),
        failed: count_files(&root.join(DIR_FAILED)),
        ..Default::default()
    };

    let done = root.join(DIR_DONE);
    if let Ok(rd) = std::fs::read_dir(&done) {
        for entry in rd.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.to_string_lossy().ends_with(".result.json") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            match JobResult::from_json(&text) {
                Ok(r) => match r.outcome {
                    JobOutcome::Completed => status.completed += 1,
                    JobOutcome::Partial => status.partial += 1,
                    JobOutcome::Failed => status.failed += 1,
                },
                // An unparseable result is a failure, not a silent skip.
                Err(_) => status.failed += 1,
            }
        }
    }
    Ok(status)
}

/// Move overdue claims back to `jobs/` (or to `failed/` if quarantined).
/// Returns (requeued, quarantined).
pub fn sweep_timeouts(root: &Path, timeout_seconds: u64) -> Result<(usize, usize), String> {
    let claimed = root.join(DIR_CLAIMED);
    let mut requeued = 0usize;
    let mut quarantined = 0usize;

    let Ok(rd) = std::fs::read_dir(&claimed) else {
        return Ok((0, 0));
    };
    for entry in rd.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().map(|x| x != "json").unwrap_or(true) {
            continue;
        }
        let age = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| SystemTime::now().duration_since(t).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Prior failures are tracked by a sidecar counter file so the count
        // survives the job moving between directories.
        let counter = claimed.join(format!(
            "{}.failures",
            path.file_stem().unwrap_or_default().to_string_lossy()
        ));
        let prior: u32 = std::fs::read_to_string(&counter)
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);

        match classify_claimed(age, timeout_seconds, prior) {
            ClaimVerdict::Running => {}
            ClaimVerdict::Requeue => {
                let dest = root.join(DIR_JOBS).join(path.file_name().unwrap_or_default());
                std::fs::rename(&path, &dest)
                    .map_err(|e| format!("requeueing {}: {}", path.display(), e))?;
                let _ = std::fs::write(&counter, (prior + 1).to_string());
                requeued += 1;
            }
            ClaimVerdict::Quarantine => {
                let dest = root.join(DIR_FAILED).join(path.file_name().unwrap_or_default());
                std::fs::rename(&path, &dest)
                    .map_err(|e| format!("quarantining {}: {}", path.display(), e))?;
                quarantined += 1;
            }
        }
    }
    Ok((requeued, quarantined))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_claim_is_still_running() {
        assert_eq!(classify_claimed(30, 180, 0), ClaimVerdict::Running);
    }

    #[test]
    fn an_overdue_claim_is_requeued_once() {
        assert_eq!(classify_claimed(200, 180, 0), ClaimVerdict::Requeue);
    }

    #[test]
    fn a_job_that_has_already_failed_twice_is_quarantined() {
        // Never retry indefinitely: one poisonous job must not silently stall
        // the whole batch.
        assert_eq!(classify_claimed(200, 180, 2), ClaimVerdict::Quarantine);
    }

    #[test]
    fn status_totals_are_reported_even_when_everything_failed() {
        let s = QueueStatus { pending: 0, claimed: 0, completed: 0, partial: 0, failed: 180 };
        let line = s.summary();
        assert!(line.contains("failed=180"), "denominator must be visible: {}", line);
        assert!(!s.is_clean(), "a batch of 180 failures is not a clean run");
    }

    #[test]
    fn a_run_with_no_completions_is_never_clean() {
        let s = QueueStatus { pending: 0, claimed: 0, completed: 0, partial: 0, failed: 0 };
        assert!(!s.is_clean(), "zero completed games cannot be a clean verdict");
    }
}
