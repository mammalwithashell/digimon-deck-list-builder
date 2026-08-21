//! Supervises the DCGO oracle across a batch: keep it running, notice when
//! it stops making progress, restart it within a budget.
//!
//! ## Three signals, because two is not enough
//!
//! 1. **Dead process / dead coroutine.** Caught by the heartbeat
//!    (`daemon::classify_heartbeat`): `JobWatcher` rewrites
//!    `<root>/harness.heartbeat` every poll, so a stale mtime means the poll
//!    loop itself stopped.
//!
//! 2. **Claim age past the job's own `limits.timeout_seconds`.** The
//!    heartbeat is written from the TOP of the poll loop, so a game stuck in
//!    an infinite selection loop keeps the poll loop -- and the heartbeat --
//!    running while nothing advances. Observed live: a malformed deck made
//!    DCGO spin on "Unexpected Player Selection" indefinitely with a
//!    perfectly healthy heartbeat. Heartbeat alone sails straight past this.
//!
//! 3. **Forward progress -- the tiebreaker signal (2) needs.** Also observed
//!    live: a claim 256s old against a 240s timeout, heartbeat 1s old, and
//!    `dcgo-player.log` actively emitting "Active Attack, Battle Step" one
//!    second ago. That game was genuinely playing, just longer than a
//!    too-short timeout -- reaping on claim age alone would have killed a
//!    healthy long game, which is worse than the hang it was meant to catch.
//!    An overdue claim is only a real hang when there is ALSO no recent
//!    forward progress. Preferred evidence: the mtime of the most recently
//!    modified file in the recordings corpus directory (the recorder appends
//!    a row per decision, so its mtime advances while the game is alive);
//!    the player log's own mtime is the fallback when no corpus directory is
//!    configured.
//!
//! Signal 2 is necessary but not sufficient; signal 3 is what makes it
//! sufficient. Watching only the heartbeat sails past mode 2 entirely (the
//! mode that actually happened first); watching claim age without progress
//! reaps healthy long games (the false positive that happened second). All
//! three checks run every poll.

use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::daemon::{self, Health};
use crate::job::{JobSpec, DIR_CLAIMED};
use crate::queue::{self, QueueStatus};

/// Fallback timeout used only when a claimed job's file cannot be read or
/// parsed (corrupt, truncated mid-write). Matches `submit`'s own CLI
/// default, so an unreadable job is judged by the same budget a normal job
/// would have gotten, not an arbitrarily stricter or looser one.
pub const DEFAULT_JOB_TIMEOUT_SECONDS: u64 = 180;

/// How old the forward-progress evidence (newest recording file, or the
/// player log as fallback) may be before an already-overdue claim is judged
/// truly stuck rather than just a long game. 60s: double
/// `daemon::DEFAULT_STALE_SECONDS` (30s), which already has to absorb normal
/// poll jitter for a signal written every poll tick; a decision-to-decision
/// gap in active play (combat math, an animation, a slow AI pick) is
/// ordinarily single-digit seconds, so 60s leaves generous headroom for a
/// legitimately slow step without leaving a genuine stall undetected for
/// long relative to jobs' own multi-minute timeouts.
pub const DEFAULT_PROGRESS_STALE_SECONDS: u64 = 60;

/// What the supervisor should do this poll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The process is alive and no claim is both overdue AND stalled.
    Healthy,
    /// Mode 1: heartbeat stale or absent.
    HungNoHeartbeat,
    /// Mode 2 (necessary) + mode 3 (sufficient): this claim has outlived its
    /// own `limits.timeout_seconds` AND there is no recent forward progress.
    HungStuckJob { job_id: String, age_seconds: u64 },
    /// Nothing left pending or claimed -- the batch is finished.
    Drained,
}

/// Pure decision core. No I/O -- callers own reading the queue, heartbeat,
/// and progress evidence, and pass in already-resolved values.
///
/// `claimed` is `(job_id, claim_age_seconds, that_job's_own_timeout_seconds)`
/// for every file currently sitting in `claimed/`. `progress_age_seconds` is
/// how long ago forward progress was last observed (`None` when no evidence
/// could be read at all -- absent corpus dir and absent/unreadable log both
/// collapse to this).
///
/// Order matters:
/// - `Drained` is checked first and wins regardless of every other signal:
///   once nothing is pending or claimed, a heartbeat or progress trail that
///   has since gone idle is not a hang -- there is nothing left to hang on.
/// - Heartbeat health is judged next (mode 1), before any per-job timeout or
///   progress check: a dead poll loop makes every claim's age and every
///   progress reading meaningless to attribute to a specific job.
/// - Only then does a per-job timeout matter, and only as a *candidate* --
///   it is confirmed a hang only when progress is not recent. A claim past
///   its own timeout with recent progress is a genuinely long game, not a
///   hang: reaping it would destroy real work over a too-short timeout, the
///   exact false positive observed live. `None` progress evidence fails
///   closed (treated as not-recent, i.e. it does NOT exonerate), matching
///   this module's existing rule that unreadable state is never read as
///   healthy (see `read_claimed_ages`'s `u64::MAX` fallback for an
///   unreadable claim mtime).
/// - Within the overdue-and-stalled check, the first such claim found is
///   enough to act on; a supervisor does not need to enumerate every one to
///   know a restart is warranted.
pub fn classify(
    hb: Health,
    claimed: &[(String, u64, u64)],
    pending: usize,
    progress_age_seconds: Option<u64>,
    progress_stale_seconds: u64,
) -> Verdict {
    if pending == 0 && claimed.is_empty() {
        return Verdict::Drained;
    }
    match hb {
        Health::Stale { .. } | Health::Missing => return Verdict::HungNoHeartbeat,
        Health::Healthy => {}
    }

    let progress_is_recent =
        matches!(progress_age_seconds, Some(age) if age <= progress_stale_seconds);

    for (job_id, age_seconds, timeout_seconds) in claimed {
        // Inclusive threshold mirrors `classify_heartbeat` and
        // `classify_claimed`: a claim landing exactly on its own timeout is
        // poll jitter, not a hang.
        if age_seconds > timeout_seconds && !progress_is_recent {
            return Verdict::HungStuckJob {
                job_id: job_id.clone(),
                age_seconds: *age_seconds,
            };
        }
    }
    Verdict::Healthy
}

/// Read every job currently sitting in `claimed/` as
/// `(job_id, claim_age_seconds, that_job's_own_timeout_seconds)` -- the exact
/// shape `classify` needs for its necessary (but not sufficient) check. The
/// timeout is read from the job's own `limits.timeout_seconds` (the contract
/// the job spec itself makes), not a single CLI-wide value;
/// `default_timeout_seconds` is used only when the file cannot be read or
/// parsed.
///
/// Age computation mirrors `queue::sweep_timeouts`: an unreadable mtime
/// fails closed to `u64::MAX` (maximally overdue) rather than `0`
/// (never overdue) -- a claim we cannot inspect must not read as healthy.
fn read_claimed_ages(root: &Path, default_timeout_seconds: u64) -> Vec<(String, u64, u64)> {
    let claimed_dir = root.join(DIR_CLAIMED);
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(&claimed_dir) else {
        return out;
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
            .unwrap_or(u64::MAX);
        let job_id = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let timeout_seconds = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| JobSpec::from_json(&text).ok())
            .map(|spec| spec.limits.timeout_seconds)
            .unwrap_or(default_timeout_seconds);
        out.push((job_id, age, timeout_seconds));
    }
    out
}

/// Seconds since `path` was last modified. `None` if it cannot be read
/// (does not exist, permissions, etc.) -- distinct from "recent", never
/// conflated with it.
fn file_age(path: &Path) -> Option<u64> {
    let meta = std::fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?;
    Some(modified.elapsed().map(|d| d.as_secs()).unwrap_or(0))
}

/// Age, in seconds, of the most recently modified regular file directly
/// inside `dir`. `None` if the directory cannot be read or has no entries
/// with a readable mtime.
fn newest_entry_age(dir: &Path) -> Option<u64> {
    let rd = std::fs::read_dir(dir).ok()?;
    let mut newest: Option<u64> = None;
    for entry in rd.filter_map(|e| e.ok()) {
        let age = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.elapsed().ok())
            .map(|d| d.as_secs());
        if let Some(age) = age {
            newest = Some(match newest {
                Some(n) => n.min(age),
                None => age,
            });
        }
    }
    newest
}

/// Resolve the forward-progress signal `classify` uses to break the tie on
/// an overdue claim. Prefers the newest file mtime in `corpus_dir` (the
/// recorder appends a row per decision, so its mtime advances while a game
/// is alive); falls back to the player log's own mtime under `root` when no
/// corpus directory is configured, or it has no readable entries.
fn progress_age(root: &Path, corpus_dir: Option<&Path>) -> Option<u64> {
    if let Some(dir) = corpus_dir {
        if let Some(age) = newest_entry_age(dir) {
            return Some(age);
        }
    }
    file_age(&root.join(daemon::LOG_FILE))
}

/// Result of a `watch` run: everything `main` needs to print the full
/// denominator regardless of whether the batch drained cleanly or the
/// restart budget ran out.
#[derive(Debug, Clone)]
pub struct WatchOutcome {
    /// True only if the queue drained (`pending == 0 && claimed == 0`).
    /// False means the restart budget was spent with work still remaining.
    pub drained: bool,
    pub restarts_used: u32,
    pub max_restarts: u32,
    /// One entry per hang detected, in order, naming which mode fired and
    /// what was done about it -- so a batch where most jobs died can never
    /// be mistaken for a quiet, clean run.
    pub events: Vec<String>,
    pub final_status: QueueStatus,
}

/// Loop until the queue drains or the restart budget is spent.
///
/// Ensures the player is running, then polls every `poll` until `classify`
/// reports a hang or `Drained`. On a hang: stop the process, sweep every
/// currently-claimed job back to `jobs/` (or into quarantine on repeated
/// failure) via the existing `queue::sweep_timeouts`, relaunch, and count it
/// against `max_restarts`. A timeout of `0` is passed to `sweep_timeouts`
/// deliberately: the process that was babysitting every current claim is
/// about to be killed, so none of them can make further progress under it,
/// whether or not each one had individually exceeded its own timeout yet.
///
/// `corpus_dir` is the recordings directory used for the forward-progress
/// signal (mode 3); pass `None` to fall back to the player log's own mtime
/// only. `progress_stale_seconds` is the staleness threshold for that
/// signal -- see `DEFAULT_PROGRESS_STALE_SECONDS` for the default and why.
///
/// Not unit tested directly -- it spawns real processes via `daemon::up`.
/// The decision logic it calls (`classify`) is unit tested exhaustively;
/// this function is a thin, mostly-untested wrapper around it plus known,
/// already-tested I/O (`daemon`, `queue`).
#[allow(clippy::too_many_arguments)]
pub fn run(
    root: &Path,
    build_dir: &Path,
    poll: Duration,
    stale_seconds: u64,
    max_restarts: u32,
    corpus_dir: Option<&Path>,
    progress_stale_seconds: u64,
) -> Result<WatchOutcome, String> {
    let mut restarts_used = 0u32;
    let mut events: Vec<String> = Vec::new();
    let mut last_verdict: Option<Verdict> = None;

    loop {
        let up_msg = daemon::up(root, build_dir)?;
        println!("watch: {}", up_msg);

        loop {
            std::thread::sleep(poll);

            let status = queue::scan(root)?;
            let hb = daemon::classify_heartbeat(daemon::heartbeat_age(root), stale_seconds);
            let claimed = read_claimed_ages(root, DEFAULT_JOB_TIMEOUT_SECONDS);
            let progress = progress_age(root, corpus_dir);
            let verdict = classify(hb, &claimed, status.pending, progress, progress_stale_seconds);

            if last_verdict.as_ref() != Some(&verdict) {
                println!("watch: {:?} ({})", verdict, status.summary());
                last_verdict = Some(verdict.clone());
            }

            match verdict {
                Verdict::Drained => {
                    return Ok(WatchOutcome {
                        drained: true,
                        restarts_used,
                        max_restarts,
                        events,
                        final_status: status,
                    });
                }
                Verdict::Healthy => {
                    // Stay in the inner loop; keep polling this process.
                }
                Verdict::HungNoHeartbeat | Verdict::HungStuckJob { .. } => {
                    let mode_desc = match &verdict {
                        Verdict::HungNoHeartbeat => "mode 1 (no heartbeat)".to_string(),
                        Verdict::HungStuckJob {
                            job_id,
                            age_seconds,
                        } => format!(
                            "mode 2 (stuck job {}, claimed {}s ago, no recent progress)",
                            job_id, age_seconds
                        ),
                        _ => unreachable!(),
                    };

                    if restarts_used >= max_restarts {
                        // Budget already spent -- leave the hung process in
                        // place for postmortem instead of tearing it down
                        // for no further benefit, and report rather than
                        // act.
                        let final_status = queue::scan(root)?;
                        events.push(format!(
                            "{} -- restart budget exhausted ({}/{}), left running for inspection",
                            mode_desc, restarts_used, max_restarts
                        ));
                        return Ok(WatchOutcome {
                            drained: false,
                            restarts_used,
                            max_restarts,
                            events,
                            final_status,
                        });
                    }

                    println!(
                        "watch: hang detected -- {}; restarting (budget {}/{})",
                        mode_desc,
                        restarts_used + 1,
                        max_restarts
                    );
                    daemon::down(root)?;
                    let (requeued, quarantined) = queue::sweep_timeouts(root, 0)?;
                    println!(
                        "watch: swept claimed/ -> requeued={} quarantined={}",
                        requeued, quarantined
                    );
                    events.push(format!(
                        "{} -- restarted ({}/{}), swept requeued={} quarantined={}",
                        mode_desc,
                        restarts_used + 1,
                        max_restarts,
                        requeued,
                        quarantined
                    ));
                    restarts_used += 1;
                    last_verdict = None; // force a fresh print once it settles
                    break; // back to the outer loop: `up` relaunches
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- classify: the pure decision core ---

    #[test]
    fn healthy_heartbeat_with_a_claim_well_within_its_timeout_is_healthy() {
        let claimed = vec![("vol-1".to_string(), 10u64, 180u64)];
        assert_eq!(
            classify(Health::Healthy, &claimed, 5, Some(2), 60),
            Verdict::Healthy
        );
    }

    #[test]
    fn healthy_heartbeat_with_a_claim_past_its_own_timeout_and_stale_progress_is_hung_stuck_job() {
        // The regression that matters most: DCGO spun on "Unexpected Player
        // Selection" forever while the poll loop (and thus the heartbeat)
        // stayed perfectly healthy, and nothing was writing to the recording
        // or the log either. Progress evidence is stale (or absent), so this
        // must be reported as a genuine hang.
        let claimed = vec![("vol-42".to_string(), 200u64, 180u64)];
        assert_eq!(
            classify(Health::Healthy, &claimed, 0, Some(500), 60),
            Verdict::HungStuckJob {
                job_id: "vol-42".to_string(),
                age_seconds: 200,
            }
        );
    }

    #[test]
    fn an_overdue_claim_with_recent_progress_is_healthy_not_hung() {
        // Live false positive this test pins: claim 256s old against a 240s
        // timeout, heartbeat 1s old, dcgo-player.log actively emitting
        // "Active Attack, Battle Step" 1s ago. That game was genuinely
        // playing, just past a too-short timeout. Reaping on claim age alone
        // would destroy a healthy long game -- worse than the hang it
        // prevents. This must fail before the progress signal exists.
        let claimed = vec![("vb-003".to_string(), 256u64, 240u64)];
        assert_eq!(
            classify(Health::Healthy, &claimed, 0, Some(1), 60),
            Verdict::Healthy
        );
    }

    #[test]
    fn an_overdue_claim_with_no_progress_evidence_at_all_is_still_hung() {
        // `None` (corpus dir unreadable/absent AND log unreadable/absent)
        // must fail closed -- it does not exonerate an overdue claim the way
        // genuinely recent progress does.
        let claimed = vec![("vol-9".to_string(), 300u64, 180u64)];
        assert_eq!(
            classify(Health::Healthy, &claimed, 0, None, 60),
            Verdict::HungStuckJob {
                job_id: "vol-9".to_string(),
                age_seconds: 300,
            }
        );
    }

    #[test]
    fn a_claim_within_its_timeout_with_stale_progress_is_not_yet_hung() {
        // A long think between decisions is normal. Progress staleness is
        // only ever a tiebreaker for an ALREADY-overdue claim, never a
        // standalone trigger.
        let claimed = vec![("vol-3".to_string(), 30u64, 180u64)];
        assert_eq!(
            classify(Health::Healthy, &claimed, 0, Some(600), 60),
            Verdict::Healthy
        );
    }

    #[test]
    fn stale_heartbeat_is_hung_no_heartbeat_regardless_of_progress() {
        let claimed = vec![("vol-1".to_string(), 10u64, 180u64)];
        assert_eq!(
            classify(Health::Stale { age_seconds: 90 }, &claimed, 0, Some(0), 60),
            Verdict::HungNoHeartbeat
        );
    }

    #[test]
    fn missing_heartbeat_with_a_recorded_pid_is_hung_not_healthy() {
        // `Health::Missing` is what `classify_heartbeat` reports when the
        // heartbeat file does not exist at all -- including "a pid is
        // recorded (the process was launched) but it never reached its
        // first poll-loop iteration". That must read as hung, never as a
        // quiet kind of healthy.
        let claimed: Vec<(String, u64, u64)> = vec![];
        assert_eq!(
            classify(Health::Missing, &claimed, 3, None, 60),
            Verdict::HungNoHeartbeat
        );
    }

    #[test]
    fn nothing_pending_and_nothing_claimed_is_drained() {
        let claimed: Vec<(String, u64, u64)> = vec![];
        assert_eq!(
            classify(Health::Healthy, &claimed, 0, None, 60),
            Verdict::Drained
        );
    }

    #[test]
    fn drained_wins_even_over_a_stale_heartbeat() {
        // Once the batch is actually finished, DCGO going quiet is not a
        // hang -- there is nothing left for it to be hung on.
        let claimed: Vec<(String, u64, u64)> = vec![];
        assert_eq!(
            classify(Health::Stale { age_seconds: 500 }, &claimed, 0, None, 60),
            Verdict::Drained
        );
    }

    #[test]
    fn a_claim_exactly_at_its_timeout_is_not_yet_hung() {
        // Boundary: poll jitter landing a claim's age exactly on its own
        // timeout must not reap a healthy game, independent of progress.
        // Mirrors `classify_heartbeat`'s and `classify_claimed`'s own
        // inclusive boundaries.
        let claimed = vec![("vol-7".to_string(), 180u64, 180u64)];
        assert_eq!(
            classify(Health::Healthy, &claimed, 0, None, 60),
            Verdict::Healthy
        );
    }

    #[test]
    fn mode_1_is_checked_before_mode_2_when_both_signals_fire() {
        // A dead poll loop makes every claim's age and every progress
        // reading meaningless to attribute to a specific job -- there is no
        // live process to have gotten stuck on any one job. Mode 1 must win
        // regardless of what the progress signal says.
        let claimed = vec![("vol-9".to_string(), 999u64, 180u64)];
        assert_eq!(
            classify(Health::Missing, &claimed, 0, Some(0), 60),
            Verdict::HungNoHeartbeat
        );
    }

    #[test]
    fn the_first_overdue_and_stalled_claim_found_is_named_when_several_are_claimed() {
        let claimed = vec![
            ("vol-a".to_string(), 10u64, 180u64),  // within budget
            ("vol-b".to_string(), 250u64, 180u64), // overdue + stale progress
        ];
        assert_eq!(
            classify(Health::Healthy, &claimed, 0, Some(600), 60),
            Verdict::HungStuckJob {
                job_id: "vol-b".to_string(),
                age_seconds: 250,
            }
        );
    }

    // --- read_claimed_ages: filesystem-backed, no process spawning ---

    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp_root(tag: &str) -> std::path::PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("dcgo_harness_watch_test_{tag}_{nanos}_{n}"));
        std::fs::create_dir_all(&dir).expect("create temp root");
        dir
    }

    fn cleanup(root: &Path) {
        let _ = std::fs::remove_dir_all(root);
    }

    fn sample_job_json(job_id: &str, timeout_seconds: u64) -> String {
        format!(
            r#"{{
                "job_id": "{job_id}",
                "policy": "ai",
                "decks": {{"p0": ["EX12-035"], "p1": ["BT16-082"]}},
                "first_player": 0,
                "seed": 1,
                "limits": {{"max_turns": 40, "timeout_seconds": {timeout_seconds}}}
            }}"#
        )
    }

    #[test]
    fn read_claimed_ages_uses_each_jobs_own_timeout() {
        let root = temp_root("own_timeout");
        let claimed_dir = root.join(DIR_CLAIMED);
        std::fs::create_dir_all(&claimed_dir).unwrap();
        std::fs::write(
            claimed_dir.join("vol-1.json"),
            sample_job_json("vol-1", 240),
        )
        .unwrap();

        let ages = read_claimed_ages(&root, DEFAULT_JOB_TIMEOUT_SECONDS);
        assert_eq!(ages.len(), 1);
        assert_eq!(ages[0].0, "vol-1");
        assert_eq!(ages[0].2, 240, "must read the job's own configured timeout");

        cleanup(&root);
    }

    #[test]
    fn read_claimed_ages_falls_back_to_default_on_unparseable_job() {
        let root = temp_root("fallback_timeout");
        let claimed_dir = root.join(DIR_CLAIMED);
        std::fs::create_dir_all(&claimed_dir).unwrap();
        std::fs::write(claimed_dir.join("vol-bad.json"), "not json").unwrap();

        let ages = read_claimed_ages(&root, DEFAULT_JOB_TIMEOUT_SECONDS);
        assert_eq!(ages.len(), 1);
        assert_eq!(ages[0].0, "vol-bad");
        assert_eq!(
            ages[0].2, DEFAULT_JOB_TIMEOUT_SECONDS,
            "an unparseable job must fall back to the default, not disappear or panic"
        );

        cleanup(&root);
    }

    #[test]
    fn read_claimed_ages_on_a_fresh_root_is_empty() {
        let root = temp_root("empty_claimed");
        // Deliberately do not create claimed/ at all.
        let ages = read_claimed_ages(&root, DEFAULT_JOB_TIMEOUT_SECONDS);
        assert!(ages.is_empty());
        cleanup(&root);
    }

    #[test]
    fn read_claimed_ages_ignores_non_json_siblings() {
        let root = temp_root("ignore_siblings");
        let claimed_dir = root.join(DIR_CLAIMED);
        std::fs::create_dir_all(&claimed_dir).unwrap();
        std::fs::write(
            claimed_dir.join("vol-1.json"),
            sample_job_json("vol-1", 180),
        )
        .unwrap();
        // A failure-counter sidecar written by `sweep_timeouts` lives right
        // alongside claimed job files; it must never be misread as a job.
        std::fs::write(claimed_dir.join("vol-1.failures"), "1").unwrap();

        let ages = read_claimed_ages(&root, DEFAULT_JOB_TIMEOUT_SECONDS);
        assert_eq!(
            ages.len(),
            1,
            "the .failures sidecar must not be counted as a claim"
        );

        cleanup(&root);
    }

    // --- progress_age: filesystem-backed, no process spawning ---

    #[test]
    fn progress_age_prefers_the_newest_file_in_the_corpus_dir() {
        let root = temp_root("progress_corpus");
        let corpus = root.join("recordings");
        std::fs::create_dir_all(&corpus).unwrap();
        std::fs::write(corpus.join("game-1.jsonl"), "{}").unwrap();
        // A log file under root exists too, but the corpus dir must win.
        std::fs::write(root.join(daemon::LOG_FILE), "log line").unwrap();

        let age = progress_age(&root, Some(&corpus));
        assert_eq!(age, Some(0), "a freshly written recording is recent progress");

        cleanup(&root);
    }

    #[test]
    fn progress_age_falls_back_to_the_player_log_when_no_corpus_dir_given() {
        let root = temp_root("progress_log_fallback");
        std::fs::write(root.join(daemon::LOG_FILE), "log line").unwrap();

        let age = progress_age(&root, None);
        assert_eq!(age, Some(0));

        cleanup(&root);
    }

    #[test]
    fn progress_age_falls_back_to_the_player_log_when_corpus_dir_is_empty() {
        let root = temp_root("progress_empty_corpus");
        let corpus = root.join("recordings");
        std::fs::create_dir_all(&corpus).unwrap();
        // corpus dir exists but has nothing in it.
        std::fs::write(root.join(daemon::LOG_FILE), "log line").unwrap();

        let age = progress_age(&root, Some(&corpus));
        assert_eq!(age, Some(0), "must fall back to the log, not report no evidence");

        cleanup(&root);
    }

    #[test]
    fn progress_age_is_none_when_nothing_is_readable() {
        let root = temp_root("progress_nothing");
        // Neither a corpus dir nor a log file exists.
        let age = progress_age(&root, Some(&root.join("nonexistent")));
        assert_eq!(age, None);
        cleanup(&root);
    }
}
