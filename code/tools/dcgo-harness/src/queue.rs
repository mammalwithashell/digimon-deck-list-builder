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

/// Count `*.json` files directly in `dir`.
///
/// `scan` returns `Result<QueueStatus, String>` specifically so directory
/// errors are visible rather than silently read as "zero jobs" — a real
/// permissions/IO problem must not be indistinguishable from an empty queue.
/// A missing directory is the one case that legitimately means zero: a fresh
/// root before its first `submit` has no `jobs/`/`claimed/`/`failed/` yet.
fn count_files(dir: &Path) -> Result<usize, String> {
    match std::fs::read_dir(dir) {
        Ok(rd) => Ok(rd
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
            .count()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(e) => Err(format!("reading {}: {}", dir.display(), e)),
    }
}

/// Tally the queue by reading the four directories.
pub fn scan(root: &Path) -> Result<QueueStatus, String> {
    let mut status = QueueStatus {
        pending: count_files(&root.join(DIR_JOBS))?,
        claimed: count_files(&root.join(DIR_CLAIMED))?,
        failed: count_files(&root.join(DIR_FAILED))?,
        ..Default::default()
    };

    let done = root.join(DIR_DONE);
    match std::fs::read_dir(&done) {
        Ok(rd) => {
            for entry in rd.filter_map(|e| e.ok()) {
                let path = entry.path();
                if !path.to_string_lossy().ends_with(".result.json") {
                    continue;
                }
                // A result file that cannot be read (permissions, half-written,
                // or a stray directory sharing the name) is exactly as bad as
                // one that cannot be parsed below — it must still land in
                // `failed`, never fall out of every counter unnoticed.
                let text = match std::fs::read_to_string(&path) {
                    Ok(t) => t,
                    Err(_) => {
                        status.failed += 1;
                        continue;
                    }
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
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("reading {}: {}", done.display(), e)),
    }
    Ok(status)
}

/// Move overdue claims back to `jobs/` (or to `failed/` if quarantined).
/// Returns (requeued, quarantined).
pub fn sweep_timeouts(root: &Path, timeout_seconds: u64) -> Result<(usize, usize), String> {
    let claimed = root.join(DIR_CLAIMED);
    let jobs = root.join(DIR_JOBS);
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
            // Unreadable mtime means we cannot confirm this claim is healthy.
            // Failing open (age=0 -> always "Running") is how a stuck job
            // with unreadable metadata would sit unswept forever; fail
            // closed instead so it's treated as maximally overdue.
            .unwrap_or(u64::MAX);

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
                let dest = jobs.join(path.file_name().unwrap_or_default());
                std::fs::rename(&path, &dest)
                    .map_err(|e| format!("requeueing {}: {}", path.display(), e))?;
                // Must propagate: a swallowed write here leaves the counter
                // stuck at its old value, so the job keeps reading back as
                // "0 prior failures" forever and never crosses the
                // quarantine threshold — precisely the unbounded retry loop
                // rule 2 exists to prevent. The adjacent rename() already
                // propagates the same way; the counter write must match it.
                std::fs::write(&counter, (prior + 1).to_string()).map_err(|e| {
                    format!("updating failure counter {}: {}", counter.display(), e)
                })?;
                requeued += 1;
            }
            ClaimVerdict::Quarantine => {
                let dest = root.join(DIR_FAILED).join(path.file_name().unwrap_or_default());
                std::fs::rename(&path, &dest)
                    .map_err(|e| format!("quarantining {}: {}", path.display(), e))?;
                // The job is terminal; its counter has no further use. Best
                // effort — if this fails, the orphan-reap pass below will
                // still catch it once the ID is no longer live anywhere.
                let _ = std::fs::remove_file(&counter);
                quarantined += 1;
            }
        }
    }

    // Reap orphaned `.failures` counters: `pool::build_jobs` emits
    // deterministic IDs (vol-00000, vol-00001, ...) starting from 0 on
    // EVERY submit, so a later batch can reuse an ID from an earlier one.
    // A counter is only meaningful while its job is still live (pending in
    // `jobs/` or claimed in `claimed/`); once the job is gone from both —
    // because it finished successfully into `done/`, or was already
    // quarantined into `failed/` — the leftover counter is a landmine: the
    // next batch's brand-new job with that same ID would inherit a stale
    // failure count and could be quarantined on its very first timeout.
    if let Ok(rd) = std::fs::read_dir(&claimed) {
        for entry in rd.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().map(|x| x != "failures").unwrap_or(true) {
                continue;
            }
            let job_name = format!("{}.json", path.file_stem().unwrap_or_default().to_string_lossy());
            let still_live = jobs.join(&job_name).exists() || claimed.join(&job_name).exists();
            if !still_live {
                let _ = std::fs::remove_file(&path);
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

    // --- Filesystem-backed tests for `scan` and `sweep_timeouts` ---
    //
    // These use real temp directories under `std::env::temp_dir()` rather
    // than mocks, because the bugs they cover (F1-F5) all live in the actual
    // rename/write/read-dir calls, not in any pure logic already covered by
    // `classify_claimed`'s tests above. No `tempfile` dependency: a unique
    // path is built by hand and removed at the end of each test.

    use std::sync::atomic::{AtomicU64, Ordering};

    /// A process-unique, human-legible temp root. Combines a monotonic
    /// counter with wall-clock nanos so parallel `cargo test` threads (this
    /// crate's tests run concurrently by default) never collide.
    fn temp_root(tag: &str) -> std::path::PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("dcgo_harness_queue_test_{tag}_{nanos}_{n}"));
        std::fs::create_dir_all(&dir).expect("create temp root");
        dir
    }

    fn make_queue_dirs(root: &Path) {
        for d in [DIR_JOBS, DIR_CLAIMED, DIR_DONE, DIR_FAILED] {
            std::fs::create_dir_all(root.join(d)).expect("create queue subdir");
        }
    }

    /// Set a file's mtime into the past so `sweep_timeouts`'s age
    /// calculation sees it as overdue without an actual sleep.
    fn backdate(path: &Path, seconds_ago: u64) {
        let past = SystemTime::now()
            .checked_sub(std::time::Duration::from_secs(seconds_ago))
            .expect("valid past timestamp");
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .expect("open file to backdate");
        let times = std::fs::FileTimes::new().set_modified(past);
        file.set_times(times).expect("set backdated mtime");
    }

    fn cleanup(root: &Path) {
        let _ = std::fs::remove_dir_all(root);
    }

    /// F6: the full requeue -> requeue -> quarantine lifecycle, driven by
    /// real sweeps against real files, ending with the job permanently
    /// parked in `failed/` and never touched again.
    #[test]
    fn overdue_claim_requeues_twice_then_quarantines_and_never_retries_again() {
        let root = temp_root("lifecycle");
        make_queue_dirs(&root);
        let claimed_dir = root.join(DIR_CLAIMED);
        let jobs_dir = root.join(DIR_JOBS);
        let failed_dir = root.join(DIR_FAILED);
        let claimed_job = claimed_dir.join("vol-lc.json");
        let jobs_job = jobs_dir.join("vol-lc.json");

        std::fs::write(&claimed_job, "{}").expect("seed claimed job");
        backdate(&claimed_job, 999);

        // First timeout: no prior failures -> requeue, counter becomes 1.
        let (requeued, quarantined) = sweep_timeouts(&root, 180).expect("sweep 1");
        assert_eq!((requeued, quarantined), (1, 0));
        assert!(jobs_job.exists(), "requeued job must land back in jobs/");
        assert!(!claimed_job.exists());
        assert_eq!(
            std::fs::read_to_string(claimed_dir.join("vol-lc.failures")).unwrap().trim(),
            "1"
        );

        // DCGO re-claims it; it times out again.
        std::fs::rename(&jobs_job, &claimed_job).expect("re-claim 1");
        backdate(&claimed_job, 999);

        // Second timeout: prior_failures=1 (< 2) -> still requeue, not quarantine.
        let (requeued, quarantined) = sweep_timeouts(&root, 180).expect("sweep 2");
        assert_eq!((requeued, quarantined), (1, 0));
        assert!(jobs_job.exists());
        assert_eq!(
            std::fs::read_to_string(claimed_dir.join("vol-lc.failures")).unwrap().trim(),
            "2"
        );

        // DCGO re-claims it a third time; it times out a third time.
        std::fs::rename(&jobs_job, &claimed_job).expect("re-claim 2");
        backdate(&claimed_job, 999);

        // Third timeout: prior_failures=2 -> quarantine, not another requeue.
        let (requeued, quarantined) = sweep_timeouts(&root, 180).expect("sweep 3");
        assert_eq!((requeued, quarantined), (0, 1));
        assert!(failed_dir.join("vol-lc.json").exists(), "must land in failed/");
        assert!(!jobs_job.exists());
        assert!(!claimed_job.exists());
        // The counter is terminal-cleaned once the job is quarantined (F2).
        assert!(!claimed_dir.join("vol-lc.failures").exists());

        // A further sweep must find nothing left in claimed/ to touch: the
        // job must never be requeued or quarantined a second time.
        let (requeued, quarantined) = sweep_timeouts(&root, 180).expect("sweep 4");
        assert_eq!((requeued, quarantined), (0, 0));
        assert!(failed_dir.join("vol-lc.json").exists(), "stays put in failed/");

        cleanup(&root);
    }

    /// F1: a counter write that cannot succeed must fail the whole sweep
    /// loudly, not get silently dropped (which would leave the job
    /// requeuing forever without ever crossing the quarantine threshold).
    #[test]
    fn a_failed_counter_write_propagates_as_an_error() {
        let root = temp_root("counter_write_fails");
        make_queue_dirs(&root);
        let claimed_dir = root.join(DIR_CLAIMED);
        let claimed_job = claimed_dir.join("vol-badcounter.json");
        std::fs::write(&claimed_job, "{}").expect("seed claimed job");
        backdate(&claimed_job, 999);

        // Force std::fs::write(&counter, ..) to fail: the counter path is
        // already occupied by a directory, which write() cannot overwrite.
        std::fs::create_dir_all(claimed_dir.join("vol-badcounter.failures"))
            .expect("occupy counter path with a directory");

        let result = sweep_timeouts(&root, 180);
        assert!(
            result.is_err(),
            "a swallowed counter-write error would let this job retry forever undetected"
        );

        cleanup(&root);
    }

    /// F2: a `.failures` counter left behind by a job that is no longer in
    /// `jobs/` or `claimed/` (e.g. it already finished into `done/`) must be
    /// reaped, so a later batch reusing the same deterministic ID does not
    /// inherit someone else's failure history.
    #[test]
    fn orphaned_failures_counter_is_reaped() {
        let root = temp_root("orphan");
        make_queue_dirs(&root);
        let claimed_dir = root.join(DIR_CLAIMED);
        let orphan = claimed_dir.join("vol-orphan.failures");
        std::fs::write(&orphan, "2").expect("seed orphaned counter");

        let (requeued, quarantined) = sweep_timeouts(&root, 180).expect("sweep");
        assert_eq!((requeued, quarantined), (0, 0), "no live job to act on");
        assert!(!orphan.exists(), "orphaned counter must be reaped");

        cleanup(&root);
    }

    /// Companion to the orphan-reap test: a counter must survive while its
    /// job is still live, whether pending (back in `jobs/`) or claimed.
    #[test]
    fn failures_counter_survives_while_its_job_is_still_live() {
        let root = temp_root("keep_live");
        make_queue_dirs(&root);
        let claimed_dir = root.join(DIR_CLAIMED);
        let counter = claimed_dir.join("vol-keep.failures");
        std::fs::write(&counter, "1").expect("seed counter");
        // The job itself sits in jobs/ (e.g. requeued, not yet re-claimed).
        std::fs::write(root.join(DIR_JOBS).join("vol-keep.json"), "{}").expect("seed live job");

        sweep_timeouts(&root, 180).expect("sweep");
        assert!(counter.exists(), "counter for a still-live job must not be reaped");

        cleanup(&root);
    }

    /// F4: metadata that can't be read must not fail open as "healthy"
    /// (age=0 -> Running forever). This drives the scenario through the real
    /// classification: an entry whose age comes back as u64::MAX must sweep
    /// exactly like a very old file, which the lifecycle test above already
    /// confirms end-to-end. This test pins the unit-level contract
    /// `classify_claimed` relies on: MAX age is always overdue regardless of
    /// timeout.
    #[test]
    fn max_age_is_always_overdue() {
        assert_eq!(classify_claimed(u64::MAX, 180, 0), ClaimVerdict::Requeue);
        // Even an absurdly generous timeout is still exceeded by MAX age --
        // the only way age ties/undercuts the timeout is if the timeout is
        // itself u64::MAX, which no real `--timeout-seconds` value is.
        assert_eq!(classify_claimed(u64::MAX, 1_000_000_000, 0), ClaimVerdict::Requeue);
    }

    /// F3: a result file that exists but cannot be *read* (as opposed to
    /// one that reads fine but fails to *parse*, already covered by the
    /// pre-existing "unparseable result is a failure" behavior) must still
    /// count as failed, not silently vanish from every counter.
    #[test]
    fn an_unreadable_result_file_counts_as_failed() {
        let root = temp_root("unreadable_result");
        make_queue_dirs(&root);
        let done_dir = root.join(DIR_DONE);
        // A directory named like a result file cannot be read as text via
        // std::fs::read_to_string -- this exercises the read failure path,
        // distinct from the JSON-parse failure path already covered above.
        std::fs::create_dir_all(done_dir.join("vol-unreadable.result.json"))
            .expect("seed a directory shaped like a result file");

        let status = scan(&root).expect("scan");
        assert_eq!(
            status.failed, 1,
            "an unreadable result file must count as failed, not disappear: {}",
            status.summary()
        );

        cleanup(&root);
    }

    /// F3 (JSON-parse variant, exercised end-to-end via `scan` rather than
    /// only through `JobResult::from_json` directly): garbage content in a
    /// readable result file must also count as failed.
    #[test]
    fn a_garbage_result_file_counts_as_failed() {
        let root = temp_root("garbage_result");
        make_queue_dirs(&root);
        let done_dir = root.join(DIR_DONE);
        std::fs::write(done_dir.join("vol-garbage.result.json"), "not json at all")
            .expect("seed garbage result file");

        let status = scan(&root).expect("scan");
        assert_eq!(status.failed, 1, "unparseable result must count as failed: {}", status.summary());

        cleanup(&root);
    }

    /// F5: `scan` on a completely fresh root (no jobs/claimed/done/failed
    /// directories created yet) must report all zeros, not error out. A
    /// missing directory here means "nothing submitted yet," not a problem.
    #[test]
    fn scan_on_a_fresh_root_with_no_directories_returns_all_zeros() {
        let root = temp_root("fresh_root");
        // Deliberately skip make_queue_dirs: this root has never had
        // `submit` run against it.

        let status = scan(&root).expect("scan of a fresh root must not error");
        assert_eq!(status.pending, 0);
        assert_eq!(status.claimed, 0);
        assert_eq!(status.completed, 0);
        assert_eq!(status.partial, 0);
        assert_eq!(status.failed, 0);

        cleanup(&root);
    }
}
