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

/// Grace past a job's own `limits.timeout_seconds` before a claim the heartbeat
/// still names counts as a stall. The player files a job that outlives its
/// limit as `failed` ("timeout after Ns") itself, and a scripted line wedged on
/// a prompt as a prompt mismatch within seconds (DCGO fork, 2026-10-09); this
/// is the backstop for a player too wedged to file anything, or a build older
/// than that. Short enough that limit + grace stays inside the default oracle
/// wait (240 + 30 < 300), so the backstop reports a stall rather than a
/// timeout.
pub const STALL_GRACE_SECS: u64 = 30;

/// `limits.timeout_seconds` when a job's JSON does not carry one.
const DEFAULT_LIMIT_SECS: u64 = 180;

/// A player wedged on one job: its claim outlived the job's own limit while the
/// heartbeat kept naming it. Two pilots sat on one such job for 15 minutes.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Stall {
    pub job_id: String,
    pub claimed_for_s: u64,
    pub limit_s: u64,
    /// `true`: the player is still on it -- restart the player. A claim this
    /// old that the heartbeat no longer names is an orphan, set aside instead.
    pub heartbeat_names_it: bool,
    /// The newest recording under the sibling `dcgo_recordings/` and its last
    /// row: where the stalled line stopped, as far as DCGO flushed it.
    pub newest_recording: Option<String>,
    pub last_row: Option<String>,
}

/// Why [`submit_and_wait`] returned no result. `stall` is set when the player
/// is wedged -- on this job, or on another job that kept ours unclaimed.
#[derive(Debug)]
pub struct WaitError {
    pub message: String,
    pub stall: Option<Stall>,
}

impl std::fmt::Display for WaitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl From<String> for WaitError {
    fn from(message: String) -> Self {
        WaitError { message, stall: None }
    }
}

fn job_limit_secs(job_json: &str) -> u64 {
    serde_json::from_str::<serde_json::Value>(job_json)
        .ok()
        .and_then(|v| v.get("limits")?.get("timeout_seconds")?.as_u64())
        .unwrap_or(DEFAULT_LIMIT_SECS)
}

/// What the player's heartbeat names: a job id, `idle`, or nothing readable.
fn heartbeat_job(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join(crate::daemon::HEARTBEAT_FILE)).ok()?;
    let s = text.trim();
    (!s.is_empty()).then(|| s.to_string())
}

fn claim_age_secs(claim: &Path) -> Option<u64> {
    let modified = std::fs::metadata(claim).ok()?.modified().ok()?;
    Some(modified.elapsed().map(|d| d.as_secs()).unwrap_or(0))
}

/// The newest `*.jsonl` recording (not a `.state.jsonl` sidecar) under the
/// sibling `dcgo_recordings/`, with its last non-empty row, truncated.
fn newest_recording(root: &Path) -> (Option<String>, Option<String>) {
    let Some(parent) = root.parent() else { return (None, None) };
    let Ok(rd) = std::fs::read_dir(parent.join("dcgo_recordings")) else { return (None, None) };
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for e in rd.filter_map(|e| e.ok()) {
        let p = e.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !name.ends_with(".jsonl") || name.ends_with(".state.jsonl") {
            continue;
        }
        let Ok(m) = e.metadata().and_then(|m| m.modified()) else { continue };
        if newest.as_ref().map(|(t, _)| m > *t).unwrap_or(true) {
            newest = Some((m, p));
        }
    }
    let Some((_, path)) = newest else { return (None, None) };
    let last = std::fs::read_to_string(&path).ok().and_then(|t| {
        t.lines().rev().map(str::trim).find(|l| !l.is_empty()).map(|l| {
            if l.chars().count() > 400 {
                format!("{}...", l.chars().take(400).collect::<String>())
            } else {
                l.to_string()
            }
        })
    });
    (Some(path.display().to_string()), last)
}

/// A claim that outlived its job's limit plus grace: a stall when the heartbeat
/// still names it, an orphan (nobody will file it) when the player moved on.
fn overdue_claim(root: &Path, claim: &Path, job_id: &str) -> Option<Stall> {
    let age = claim_age_secs(claim)?;
    let limit = std::fs::read_to_string(claim)
        .map(|t| job_limit_secs(&t))
        .unwrap_or(DEFAULT_LIMIT_SECS);
    if age <= limit + STALL_GRACE_SECS {
        return None;
    }
    let names_it = heartbeat_job(root).as_deref() == Some(job_id);
    let (newest_recording, last_row) = if names_it { newest_recording(root) } else { (None, None) };
    Some(Stall {
        job_id: job_id.to_string(),
        claimed_for_s: age,
        limit_s: limit,
        heartbeat_names_it: names_it,
        newest_recording,
        last_row,
    })
}

fn stall_error(s: Stall) -> WaitError {
    let mut message = format!(
        "player stalled on {}: claimed {}s ago (its own limit is {}s) and the heartbeat still \
         names it; the scripted line did not complete DCGO's prompt after the last recorded row \
         -- restart the player (`node down`, then `node up --build <dir>`)",
        s.job_id, s.claimed_for_s, s.limit_s
    );
    if let Some(r) = &s.newest_recording {
        message.push_str(&format!("; newest recording {r}"));
    }
    if let Some(l) = &s.last_row {
        message.push_str(&format!(", last row {l}"));
    }
    WaitError { message, stall: Some(s) }
}

/// Any claim in `claimed/` the player is wedged on, for a timeout report.
fn any_stalled_claim(root: &Path) -> Option<Stall> {
    let rd = std::fs::read_dir(root.join(crate::job::DIR_CLAIMED)).ok()?;
    for e in rd.filter_map(|e| e.ok()) {
        let p = e.path();
        if p.extension().map(|x| x != "json").unwrap_or(true) {
            continue;
        }
        let Some(stem) = p.file_stem().and_then(|s| s.to_str()) else { continue };
        if let Some(s) = overdue_claim(root, &p, stem) {
            if s.heartbeat_names_it {
                return Some(s);
            }
        }
    }
    None
}

/// Move an orphaned claim to `aside/<job_id>.<unix secs>.json`: evidence of
/// the run that never filed, out of the way of the resubmission.
fn set_aside(root: &Path, claim: &Path, job_id: &str) -> Result<(), String> {
    let dir = root.join("aside");
    std::fs::create_dir_all(&dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let dest = dir.join(format!("{job_id}.{secs}.json"));
    std::fs::rename(claim, &dest)
        .map_err(|e| format!("setting aside {} as {}: {e}", claim.display(), dest.display()))
}

/// Write `job_json` to `<root>/jobs/<job_id>.json` (temp file + rename), after
/// removing any stale `done/<job_id>.json` / `done/<job_id>.result.json` /
/// `failed/<job_id>.json`; poll for this job's own result until `timeout`.
///
/// A `claimed/<job_id>.json` with the SAME spec is this run already on the
/// player: it is adopted (waited on), not resubmitted and not refused -- a
/// different spec is refused, since its result would be diffed against the
/// new line. A claim older than the job's own limit is a stall when the
/// heartbeat still names it (reported at once, with the evidence) and an
/// orphan otherwise (set aside, then the job is submitted afresh). The stall
/// check also runs while waiting, so a wedged player is reported before
/// `timeout` rather than read as "is the player running?".
///
/// On a timeout the job is withdrawn from `jobs/` if no player claimed it, so
/// a player started later does not run a job nobody is waiting for; when the
/// player is wedged on another job, the report names that job.
pub fn submit_and_wait(
    root: &Path,
    job_id: &str,
    job_json: &str,
    timeout: Duration,
    poll: Duration,
) -> Result<OracleRun, WaitError> {
    let claimed = root.join(crate::job::DIR_CLAIMED).join(format!("{job_id}.json"));
    let mut adopted = false;
    if claimed.exists() {
        let existing = std::fs::read_to_string(&claimed)
            .map_err(|e| format!("reading {}: {e}", claimed.display()))?;
        if existing != job_json {
            return Err(format!(
                "an earlier {job_id} is still running on a player ({} exists) with a different \
                 spec: submitting now would hand back THAT run's result. Wait for it to finish, \
                 or clear an orphaned claim with `dcgo-harness --root <root> status --sweep`.",
                claimed.display()
            )
            .into());
        }
        match overdue_claim(root, &claimed, job_id) {
            Some(s) if s.heartbeat_names_it => return Err(stall_error(s)),
            Some(_) => set_aside(root, &claimed, job_id)?,
            None => adopted = true,
        }
    }
    let done = root.join(DIR_DONE);
    let failed = root.join(DIR_FAILED).join(format!("{job_id}.json"));
    let result_path = done.join(format!("{job_id}.result.json"));
    let jobs = root.join(DIR_JOBS);
    let dest = jobs.join(format!("{job_id}.json"));
    if !adopted {
        for stale in [done.join(format!("{job_id}.json")), result_path.clone(), failed.clone()] {
            if stale.exists() {
                std::fs::remove_file(&stale)
                    .map_err(|e| format!("removing stale {}: {e}", stale.display()))?;
            }
        }
        std::fs::create_dir_all(&jobs).map_err(|e| format!("creating {}: {e}", jobs.display()))?;
        let tmp = jobs.join(format!(".{job_id}.json.tmp"));
        std::fs::write(&tmp, job_json).map_err(|e| format!("writing {}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, &dest).map_err(|e| format!("submitting {}: {e}", dest.display()))?;
    }

    let started = Instant::now();
    // DCGO writes the result with File.WriteAllText -- not atomic -- so a poll
    // can land between the truncate and the write. An unreadable result is
    // "not yet" until it has stayed unreadable this long.
    const UNREADABLE_FOR: Duration = Duration::from_secs(2);
    let mut unreadable_since: Option<Instant> = None;
    loop {
        if result_path.exists() {
            let parsed = std::fs::read_to_string(&result_path)
                .map_err(|e| format!("reading {}: {e}", result_path.display()))
                .and_then(|text| {
                    JobResult::from_json(&text)
                        .map_err(|e| format!("parsing {}: {e}", result_path.display()))
                });
            let result = match parsed {
                Ok(r) => r,
                Err(e) => {
                    let since = *unreadable_since.get_or_insert_with(Instant::now);
                    if since.elapsed() >= UNREADABLE_FOR {
                        return Err(e.into());
                    }
                    std::thread::sleep(poll);
                    continue;
                }
            };
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
            )
            .into());
        }
        if claimed.exists() {
            if let Some(s) = overdue_claim(root, &claimed, job_id) {
                if s.heartbeat_names_it {
                    return Err(stall_error(s));
                }
            }
        }
        if started.elapsed() >= timeout {
            let withdrawn = dest.exists() && std::fs::remove_file(&dest).is_ok();
            let other = if withdrawn { any_stalled_claim(root) } else { None };
            let mut message = format!(
                "oracle job {job_id} timed out after {}s{}",
                timeout.as_secs_f64().round() as u64,
                if withdrawn { "; no player claimed it, so it was withdrawn from jobs/" } else { "" }
            );
            match &other {
                Some(s) => message.push_str(&format!(
                    "; the player is stalled on {} (claimed {}s ago, its limit is {}s) -- restart it",
                    s.job_id, s.claimed_for_s, s.limit_s
                )),
                None => message.push_str(" (is the player running? `node status`)"),
            }
            return Err(WaitError { message, stall: other });
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

// ---------------------------------------------------------------------------
// The one-call loop: preflight, lower, submit, wait, diff, record, backfill.
// ---------------------------------------------------------------------------

/// What one scenario's one-call oracle run established. Serialized as one
/// JSON line per scenario by `exam --oracle`, and returned by the MCP.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OracleExamResult {
    pub scenario: String,
    /// The scenario's primary clause.
    pub clause: String,
    /// Every id the verdict is filed under: the `covers:` clauses, or the
    /// interaction id instead of any clause.
    pub ids: Vec<String>,
    /// `confirmed` | `diverged` | `unmeasured`. Unmeasured means the oracle
    /// did not measure the line (timeout, quarantine, DCGO stopped it before
    /// any divergence); nothing is written to the verdict store for it.
    pub verdict: String,
    /// The differ's lead line when the diff was not clean.
    pub first_divergence: Option<String>,
    pub divergence: Option<DivergenceAt>,
    /// Set when DCGO stopped the job on a prompt mismatch: which SCENARIO step
    /// the mismatched wire row belongs to, so a caller never has to guess the
    /// step from DCGO's row index.
    pub mismatch: Option<MismatchAt>,
    /// Why the verdict is not `confirmed`.
    pub reason: Option<String>,
    /// The differ's compared-row counts, when a diff ran.
    pub denominator: Option<String>,
    pub job_id: String,
    /// What DCGO filed: `completed` | `partial` | `failed`. Absent when no
    /// result was filed (timeout, quarantine).
    pub job_outcome: Option<String>,
    pub sidecar: Option<String>,
    /// Whether the scenario's `assert:` block was rewritten from this run.
    pub backfilled: bool,
    /// What backfill did, or why it did not run.
    pub backfill_note: Option<String>,
    /// Ids filed in the verdict store by this call.
    pub recorded: Vec<String>,
    /// The store's refusals (orphan ids), verbatim.
    pub refused: Vec<String>,
    /// The player is wedged (on this job, or on another that kept this one
    /// unclaimed): `reason` says so, this carries the evidence for triage.
    pub stall: Option<Stall>,
    /// The three-way legs of a measured run (card-loop design D7), as
    /// structured fields so a driver never parses them out of `reason`:
    /// `ours_vs_dcgo` = the differ was clean; `ours_vs_ruling` = our engine
    /// meets the scenario's `expect_ruling:` (absent without one, or when the
    /// block is vacuous); `ruling_q_id` = that block's Q-number. All absent
    /// when the line was not measured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ours_vs_dcgo: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ours_vs_ruling: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ruling_q_id: Option<String>,
}

pub struct OracleExamOptions<'a> {
    /// The harness root holding jobs/ claimed/ done/ failed/.
    pub root: &'a Path,
    /// The player build, for the preflight's action-space gate and the
    /// verdict's `dcgo_build` stamp.
    pub build: Option<&'a Path>,
    pub cards_json: &'a Path,
    pub decks: Option<&'a Path>,
    /// Record the verdict here (needs `clause_text_json`).
    pub verdicts_dir: Option<&'a Path>,
    pub clause_text_json: Option<&'a Path>,
    pub interaction_denominator: &'a Path,
    /// On `confirmed`, write the observed state into the scenario's `assert:`.
    pub backfill: bool,
    pub timeout: Duration,
    pub poll: Duration,
}

/// A DCGO prompt mismatch, placed on the scenario: DCGO's message names its
/// input ROW (`step N` is the job's N-th wire row); `step` is the scenario
/// step that put that row on the wire (`LoweredRun::wire_rows_per_step`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct MismatchAt {
    pub row: usize,
    pub step: u32,
    /// The prompt the row expected / DCGO asked, when the message names them
    /// (an actor mismatch names neither).
    pub expected: Option<String>,
    pub asked: Option<String>,
}

/// Parse DCGO's `prompt mismatch: step N expected prompt 'X' but DCGO asked 'Y'`
/// (or `... expected actor A but DCGO asked actor B`, or
/// `... step N selection did not complete 'X' (wanted [..], <why>)`) into
/// `(row, expected prompt, asked prompt)`.
///
/// "did not complete" is DCGO asking exactly the prompt the row expected and
/// the scripted answer not ending it -- too few picks for a prompt that takes
/// N or a cancel (the EX7-073 scope-neg wedge), or a prompt that sat open with
/// nothing moving -- so expected and asked are both that prompt.
pub fn parse_prompt_mismatch(message: &str) -> Option<(usize, Option<String>, Option<String>)> {
    let rest = message.split("prompt mismatch:").nth(1)?.trim();
    let rest = rest.strip_prefix("step ")?;
    let (row_s, tail) = rest.split_once(' ')?;
    let row: usize = row_s.parse().ok()?;
    let quoted = |s: &str| -> Option<String> {
        let a = s.find('\'')?;
        let b = s[a + 1..].find('\'')?;
        Some(s[a + 1..a + 1 + b].to_string())
    };
    let (expected, asked) = match tail.split_once(" but DCGO asked") {
        Some((e, a)) if e.trim_start().starts_with("expected prompt") => (quoted(e), quoted(a)),
        _ => match tail.strip_prefix("selection did not complete") {
            Some(rest) => {
                let prompt = quoted(rest);
                (prompt.clone(), prompt)
            }
            None => (None, None),
        },
    };
    Some((row, expected, asked))
}

/// The scenario step whose wire rows include `row`, from the rows each
/// scenario step put on the DCGO wire (a fold is two rows, a sim-only step
/// none). `None` when `row` is past the wire.
pub fn scenario_step_for_row(wire_rows_per_step: &[usize], row: usize) -> Option<usize> {
    let mut first = 0usize;
    for (step, rows) in wire_rows_per_step.iter().enumerate() {
        if row < first + rows {
            return Some(step);
        }
        first += rows;
    }
    None
}

/// The first divergence as fields (spec 4.2(6)): the scenario step it was
/// reported at and its first differing projection field.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DivergenceAt {
    pub step: u32,
    pub field: String,
    pub ours: String,
    pub dcgo: String,
}

impl OracleExamResult {
    /// `confirmed` AND filed: a verdict the store refused (an id outside the
    /// denominator -- the orphan rule) covers nothing, whatever the diff said.
    pub fn is_clean_confirmation(&self) -> bool {
        self.verdict == "confirmed" && self.refused.is_empty()
    }
}

/// Refuse before any submission when the node cannot answer: `node::health`
/// NO-GO, or no live player (no running `harness.pid` and no heartbeat
/// within `DEFAULT_STALE_SECONDS`) -- a job nobody can claim only times out.
pub fn preflight(root: &Path, build: Option<&Path>) -> Result<(), String> {
    let h = crate::node::health(root, build);
    if !h.go {
        return Err(format!("oracle NO-GO:\n{}", h.describe()));
    }
    let pid_live = crate::daemon::read_pid(root)
        .map(crate::daemon::pid_alive)
        .unwrap_or(false);
    let beat_fresh = crate::daemon::heartbeat_age(root)
        .map(|age| age <= crate::daemon::DEFAULT_STALE_SECONDS)
        .unwrap_or(false);
    if !pid_live && !beat_fresh {
        return Err(format!(
            "oracle refused: no live player on {} (no running {} and no {} in the last {}s), \
             so a submitted job could only time out. Start one with \
             `dcgo-harness --root <root> node up --build <dir>`.",
            root.display(),
            crate::daemon::PID_FILE,
            crate::daemon::HEARTBEAT_FILE,
            crate::daemon::DEFAULT_STALE_SECONDS
        ));
    }
    Ok(())
}

/// The DCGO commit a player build was made from, for the verdict stamp.
fn build_commit(build: Option<&Path>) -> Option<String> {
    crate::manifest::load(build?).ok().map(|m| m.dcgo_commit)
}

fn outcome_name(o: JobOutcome) -> &'static str {
    match o {
        JobOutcome::Completed => "completed",
        JobOutcome::Partial => "partial",
        JobOutcome::Failed => "failed",
    }
}

/// Does our engine agree with the scenario's Q&A ruling? `None` without one;
/// a ruling with zero checks is vacuous and never counts as agreement.
fn ruling_agrees(s: &Scenario, projections: &[crate::exam::projection::StateProjection]) -> Option<bool> {
    let (checked, failures) = crate::exam::assertions::check_ruling(s, projections)?;
    Some(failures.is_empty() && checked > 0)
}

/// The line a successful backfill reports, naming any value it left out
/// because the sim-only replay does not reproduce it.
pub fn backfill_message(scenario: &Path, dropped: &[String]) -> String {
    let mut m = format!("wrote confirmed state into {}", scenario.display());
    if !dropped.is_empty() {
        m.push_str(&format!(
            " (left out {} value(s) the sim-only replay does not reproduce -- stack the \
             cards they depend on to assert them: {})",
            dropped.len(),
            dropped.join(", ")
        ));
    }
    m
}

/// One call: preflight, then [`run_oracle_exam_loaded`] with the card data
/// and deck book loaded from `opts`.
pub fn run_oracle_exam(scenario: &Path, opts: &OracleExamOptions) -> Result<OracleExamResult, String> {
    preflight(opts.root, opts.build)?;
    let card_data = dcgo_replay::load_card_data_at(opts.cards_json)
        .map_err(|e| format!("loading {}: {e}", opts.cards_json.display()))?;
    let book = crate::exam::deckbook::DeckBook::load(opts.decks, opts.cards_json)?;
    run_oracle_exam_loaded(scenario, opts, &card_data, &book)
}

/// Lower the scenario (refusing a line our engine cannot finish before any
/// Unity time is spent), submit its job under `exam-<stem>`, wait for THAT
/// job's result, diff its sidecar, record the verdict and backfill.
///
/// `Err` only for problems on this side (unreadable or unlowerable scenario,
/// a line that does not complete, a store that cannot be written). Whatever
/// the oracle does -- answers, diverges, stops, times out -- is a result.
/// Does not preflight: callers running a batch call [`preflight`] themselves.
pub fn run_oracle_exam_loaded(
    scenario: &Path,
    opts: &OracleExamOptions,
    card_data: &std::collections::HashMap<String, digimon_engine::CardData>,
    book: &crate::exam::deckbook::DeckBook,
) -> Result<OracleExamResult, String> {
    use crate::exam::deckbook::ordered_deck;

    let text = std::fs::read_to_string(scenario)
        .map_err(|e| format!("reading {}: {e}", scenario.display()))?;
    let s = Scenario::from_yaml(&text)?;
    let lowered = crate::exam::run::lower_and_run(
        &s,
        ordered_deck(&s.decks.p0, book)?,
        ordered_deck(&s.decks.p1, book)?,
        card_data,
    )?;
    if !lowered.complete {
        return Err(format!(
            "{}: the line does not run to completion in our engine ({} of {} steps{}); fix it \
             sim-only before spending an oracle run",
            scenario.display(),
            lowered.steps_run,
            lowered.steps_total,
            if lowered.stall_reasons.is_empty() {
                String::new()
            } else {
                format!(": {}", lowered.stall_reasons.join("; "))
            }
        ));
    }
    let stem = scenario
        .file_stem()
        .and_then(|x| x.to_str())
        .ok_or_else(|| format!("unnamed scenario {}", scenario.display()))?;
    let job = crate::exam::job_spec::build_exam_job(
        stem,
        &s,
        book.resolve(&s.decks.p0.rest)?,
        book.resolve(&s.decks.p1.rest)?,
        &lowered.lowered_steps,
        &lowered.lowered_owners,
    )?;
    let json = serde_json::to_string_pretty(&job)
        .map_err(|e| format!("serializing job {}: {e}", job.job_id))?
        + "\n";

    let mut result = OracleExamResult {
        scenario: scenario.display().to_string(),
        clause: s.clause.clone(),
        ids: match &s.interaction {
            Some(i) => vec![i.id.clone()],
            None => s.covered_clauses(),
        },
        verdict: "unmeasured".to_string(),
        first_divergence: None,
        divergence: None,
        mismatch: None,
        reason: None,
        denominator: None,
        job_id: job.job_id.clone(),
        job_outcome: None,
        sidecar: None,
        backfilled: false,
        backfill_note: None,
        recorded: Vec::new(),
        refused: Vec::new(),
        stall: None,
        ours_vs_dcgo: None,
        ours_vs_ruling: None,
        ruling_q_id: None,
    };

    let run = match submit_and_wait(opts.root, &job.job_id, &json, opts.timeout, opts.poll) {
        Ok(run) => run,
        Err(e) => {
            result.reason = Some(e.message);
            result.stall = e.stall;
            return Ok(result);
        }
    };
    let outcome = outcome_name(run.outcome);
    let completed = run.outcome == JobOutcome::Completed;
    result.job_outcome = Some(outcome.to_string());
    result.sidecar = run.sidecar_path.as_ref().map(|p| p.display().to_string());
    let dcgo_said = if run.message.is_empty() {
        format!("DCGO job {outcome}")
    } else {
        format!("DCGO job {outcome}: {}", run.message)
    };
    if !completed {
        result.mismatch = parse_prompt_mismatch(&run.message).map(|(row, expected, asked)| MismatchAt {
            row,
            step: scenario_step_for_row(&lowered.wire_rows_per_step, row)
                .unwrap_or(lowered.wire_rows_per_step.len().saturating_sub(1)) as u32,
            expected,
            asked,
        });
    }
    let Some(sidecar) = run.sidecar_path.as_ref() else {
        result.reason = Some(format!("{dcgo_said} (no recording filed)"));
        return Ok(result);
    };
    let od = match crate::exam::oracle_diff::diff_against_sidecar(&text, sidecar, card_data) {
        Ok(od) => od,
        Err(e) => {
            result.reason = Some(if completed {
                e
            } else {
                format!("{dcgo_said} (its partial trace could not be diffed: {e})")
            });
            return Ok(result);
        }
    };
    result.denominator = Some(od.report.denominator());
    if !od.report.is_clean() {
        result.first_divergence = format!("{}", od.report).lines().next().map(str::to_string);
        result.divergence = od.report.first().and_then(|d| {
            d.diffs.first().map(|f| DivergenceAt {
                step: d.step,
                field: f.path.clone(),
                ours: f.ours.clone(),
                dcgo: f.dcgo.clone(),
            })
        });
    }
    // DCGO stopped the line early (a prompt mismatch, a turn cap) and nothing
    // diverged before it did: the clause was not measured.
    if !completed && od.report.divergences.is_empty() {
        result.reason = Some(format!("{dcgo_said} -- stopped before the line finished, with no divergence before it"));
        return Ok(result);
    }

    let ruling_ok = ruling_agrees(&s, &od.projections);
    let mut event = VerdictEvent::from_diff(&s, &od.report, ruling_ok, scenario.display().to_string());
    // The legs, structured (design D7): the differ's verdict and the ruling's,
    // so `ours = DCGO != ruling` (ours_wrong + DCGO fork candidate) and
    // `ours = ruling != DCGO` (dcgo_quirk) are read, not parsed from `reason`.
    result.ours_vs_dcgo = Some(od.report.is_clean());
    result.ours_vs_ruling = ruling_ok;
    result.ruling_q_id = s.expect_ruling.as_ref().map(|r| r.q_id.clone());
    if !completed {
        event.reason = Some(format!("{}; {dcgo_said}", event.reason.unwrap_or_default()));
    }
    let confirmed = event.verdict == Verdict::Confirmed;
    result.verdict = if confirmed { "confirmed" } else { "diverged" }.to_string();
    result.reason = event.reason.clone();

    if opts.backfill && confirmed {
        result.backfill_note = Some(match crate::exam::backfill::backfill_from_diff(
            &text,
            &od.projections,
            &od.dcgo,
            &od.pairing,
            &od.report,
            // The pre-submission lowering IS the sim-only replay CI runs.
            &lowered.projections,
        ) {
            Ok(b) => match std::fs::write(scenario, &b.text) {
                Ok(()) => {
                    result.backfilled = true;
                    backfill_message(scenario, &b.dropped)
                }
                Err(e) => format!("backfill skipped: writing {}: {e}", scenario.display()),
            },
            Err(reason) => format!("backfill skipped: {reason}"),
        });
    }

    if let (Some(dir), Some(ctj)) = (opts.verdicts_dir, opts.clause_text_json) {
        let mut recorder = VerdictRecorder::load(dir, ctj, opts.interaction_denominator)?;
        let recorded_at = chrono::Utc::now().to_rfc3339();
        let dcgo_build = build_commit(opts.build);
        for r in recorder.record(&event, &recorded_at) {
            match r {
                Ok(id) => {
                    recorder
                        .store
                        .set_provenance(&id, Some(run.job_id.clone()), dcgo_build.clone())?;
                    result.recorded.push(id);
                }
                Err(e) => result.refused.push(e),
            }
        }
        recorder.save(dir)?;
    }
    Ok(result)
}

#[cfg(test)]
mod mismatch_tests {
    use super::*;

    #[test]
    fn rows_map_to_the_scenario_step_that_put_them_on_the_wire() {
        // step 0: 1 row, step 1: 2 rows (a fold), step 2: 0 rows (sim-only), step 3: 1 row
        let wire = [1usize, 2, 0, 1];
        assert_eq!(scenario_step_for_row(&wire, 0), Some(0));
        assert_eq!(scenario_step_for_row(&wire, 1), Some(1));
        assert_eq!(scenario_step_for_row(&wire, 2), Some(1));
        assert_eq!(scenario_step_for_row(&wire, 3), Some(3));
        assert_eq!(scenario_step_for_row(&wire, 4), None);
    }

    #[test]
    fn dcgos_mismatch_messages_parse() {
        let m = parse_prompt_mismatch("prompt mismatch: step 12 expected prompt 'main_phase' but DCGO asked 'SelectDigiXrosClass'").unwrap();
        assert_eq!((m.0, m.1.as_deref(), m.2.as_deref()), (12, Some("main_phase"), Some("SelectDigiXrosClass")));
        let m = parse_prompt_mismatch("prompt mismatch: step 14 expected actor 0 but DCGO asked actor 1").unwrap();
        assert_eq!((m.0, m.1, m.2), (14, None, None));
        assert!(parse_prompt_mismatch("bad deck").is_none());
    }

    #[test]
    fn an_incomplete_selection_parses_as_that_prompt_on_both_sides() {
        // InputDriver.AbortIncomplete / CheckStall (DCGO fork, 2026-10-09):
        // the EX7-073 scope-neg wedge, now filed instead of held.
        for msg in [
            "prompt mismatch: step 20 selection did not complete 'SelectCardEffect' \
             (wanted [BT25-085], prompt needs 2 picks or cancel)",
            "prompt mismatch: step 20 selection did not complete 'SelectCardEffect' \
             (wanted [BT25-085], DCGO held a prompt open for 10s after step 19 with no new \
             prompt and no game progress)",
        ] {
            let m = parse_prompt_mismatch(msg).unwrap();
            assert_eq!(
                (m.0, m.1.as_deref(), m.2.as_deref()),
                (20, Some("SelectCardEffect"), Some("SelectCardEffect")),
                "{msg}"
            );
        }
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
    fn a_previous_submission_still_claimed_with_a_different_spec_is_refused_not_adopted() {
        // A timed-out earlier run of the same stem, from an OLDER version of the
        // scenario, still on the player would file ITS result first, and we
        // would diff the new line against it.
        let t = root();
        std::fs::write(t.path().join("claimed/exam-X-effect0.json"), "{\"older\": true}").unwrap();
        let started = std::time::Instant::now();
        let err = submit_and_wait(t.path(), "exam-X-effect0", "{}", Duration::from_secs(5),
                                  Duration::from_millis(20)).unwrap_err().to_string();
        assert!(started.elapsed() < Duration::from_secs(1), "refused up front, not after waiting");
        assert!(err.contains("still running") && err.contains("exam-X-effect0.json"), "{err}");
        assert!(!t.path().join("jobs/exam-X-effect0.json").exists(), "nothing may be submitted");
    }

    #[test]
    fn a_previous_submission_still_claimed_with_the_same_spec_is_adopted() {
        // The pilots' retry after a 300 s timeout found its own job still on the
        // player and was refused three times in a row, instantly: `oracle_retry
        // 3/3 spent`. The same spec on the player IS this run; wait for it.
        let t = root();
        let rec = t.path().join("rec.jsonl");
        std::fs::write(&rec, "").unwrap();
        std::fs::write(t.path().join("claimed/exam-X-effect0.json"), "{}").unwrap();
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
        assert!(!t.path().join("jobs/exam-X-effect0.json").exists(), "adopted, not resubmitted");
    }

    fn age(path: &std::path::Path, secs: u64) {
        let past = std::time::SystemTime::now() - Duration::from_secs(secs);
        std::fs::File::options().write(true).open(path).unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(past)).unwrap();
    }

    const LIMIT_1S: &str = "{\n  \"job_id\": \"exam-S-effect0\",\n  \"limits\": {\"max_turns\": 40, \"timeout_seconds\": 1}\n}\n";

    #[test]
    fn a_claim_past_its_limit_that_the_heartbeat_still_names_is_a_stall() {
        // Before the player enforced its own limit, a scripted selection that
        // could not complete its prompt held it forever, heartbeat fresh, the
        // claim ageing; two pilots wedged on one such job for 15 minutes. The
        // host check stays as the backstop for a player that cannot file.
        let t = root();
        let claim = t.path().join("claimed/exam-S-effect0.json");
        std::fs::write(&claim, LIMIT_1S).unwrap();
        age(&claim, 100);
        std::fs::write(t.path().join("harness.heartbeat"), "exam-S-effect0\n").unwrap();
        let started = std::time::Instant::now();
        let err = submit_and_wait(t.path(), "exam-S-effect0", LIMIT_1S, Duration::from_secs(5),
                                  Duration::from_millis(20)).unwrap_err();
        assert!(started.elapsed() < Duration::from_secs(2), "a stall is reported, not waited out");
        let msg = err.to_string();
        let stall = err.stall.expect("structured stall evidence");
        assert_eq!(stall.job_id, "exam-S-effect0");
        assert!(stall.claimed_for_s >= 100 && stall.limit_s == 1 && stall.heartbeat_names_it, "{stall:?}");
        assert!(msg.contains("stalled") && msg.contains("exam-S-effect0"), "{msg}");
        assert!(claim.exists(), "the stalled claim is evidence; the player restart clears it");
    }

    #[test]
    fn a_stall_is_also_caught_while_waiting_on_a_fresh_submission() {
        let t = root();
        let r = t.path().to_path_buf();
        let h = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(80));
            // The player claims the job and wedges: the claim ages past the
            // limit (set its mtime into the past) while the heartbeat names it.
            let claim = r.join("claimed/exam-S-effect0.json");
            std::fs::rename(r.join("jobs/exam-S-effect0.json"), &claim).unwrap();
            age(&claim, 100);
            std::fs::write(r.join("harness.heartbeat"), "exam-S-effect0\n").unwrap();
        });
        let err = submit_and_wait(t.path(), "exam-S-effect0", LIMIT_1S, Duration::from_secs(10),
                                  Duration::from_millis(20)).unwrap_err();
        h.join().unwrap();
        assert!(err.stall.is_some(), "{err}");
    }

    #[test]
    fn an_orphaned_claim_is_set_aside_and_the_job_resubmitted() {
        // The player was restarted under the earlier run: its claim lingers in
        // claimed/ but the heartbeat reads `idle`. Nothing will ever file it.
        let t = root();
        let claim = t.path().join("claimed/exam-S-effect0.json");
        std::fs::write(&claim, LIMIT_1S).unwrap();
        age(&claim, 100);
        std::fs::write(t.path().join("harness.heartbeat"), "idle\n").unwrap();
        let rec = t.path().join("rec.jsonl");
        let r = t.path().to_path_buf();
        let rec2 = rec.clone();
        let h = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            assert!(r.join("jobs/exam-S-effect0.json").exists(), "resubmitted for the live player");
            write_result(&r, "exam-S-effect0", "completed", &rec2, "");
        });
        let run = submit_and_wait(t.path(), "exam-S-effect0", LIMIT_1S, Duration::from_secs(5),
                                  Duration::from_millis(20)).unwrap();
        h.join().unwrap();
        assert_eq!(run.outcome, JobOutcome::Completed);
        assert!(!claim.exists(), "the orphan is out of claimed/");
        let aside: Vec<_> = std::fs::read_dir(t.path().join("aside")).unwrap()
            .filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).collect();
        assert!(aside.iter().any(|n| n.starts_with("exam-S-effect0")), "kept as evidence: {aside:?}");
    }

    #[test]
    fn a_timeout_names_the_other_job_the_player_is_stalled_on() {
        // Our job never got claimed because the player is wedged on ANOTHER
        // run's job: say so, with the evidence, instead of "is the player running?".
        let t = root();
        let other = t.path().join("claimed/exam-OTHER-effect2.json");
        std::fs::write(&other, LIMIT_1S.replace("exam-S-effect0", "exam-OTHER-effect2")).unwrap();
        age(&other, 100);
        std::fs::write(t.path().join("harness.heartbeat"), "exam-OTHER-effect2\n").unwrap();
        let err = submit_and_wait(t.path(), "exam-W-effect0", "{}", Duration::from_millis(100),
                                  Duration::from_millis(20)).unwrap_err();
        let msg = err.to_string();
        let stall = err.stall.expect("the other job's stall");
        assert_eq!(stall.job_id, "exam-OTHER-effect2");
        assert!(msg.contains("exam-OTHER-effect2"), "{msg}");
        assert!(!t.path().join("jobs/exam-W-effect0.json").exists(), "withdrawn as before");
    }

    #[test]
    fn a_result_caught_mid_write_is_retried_not_misread() {
        // DCGO writes result.json with File.WriteAllText: a poll can land
        // between the truncate and the write.
        let t = root();
        let rec = t.path().join("rec.jsonl");
        let r = t.path().to_path_buf();
        let rec2 = rec.clone();
        let h = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(60));
            std::fs::write(r.join("done/exam-P-effect0.result.json"), "{").unwrap();
            std::thread::sleep(Duration::from_millis(120));
            write_result(&r, "exam-P-effect0", "completed", &rec2, "");
        });
        let run = submit_and_wait(t.path(), "exam-P-effect0", "{}", Duration::from_secs(5),
                                  Duration::from_millis(20)).unwrap();
        h.join().unwrap();
        assert_eq!(run.outcome, JobOutcome::Completed);
    }

    #[test]
    fn a_stale_result_is_removed_before_submitting() {
        let t = root();
        let rec = t.path().join("old.jsonl");
        write_result(t.path(), "exam-X-effect0", "completed", &rec, "");
        // A leftover quarantine of the same id must not fail the new job either.
        std::fs::write(t.path().join("failed/exam-X-effect0.json"), "{}").unwrap();
        let err = submit_and_wait(t.path(), "exam-X-effect0", "{}", Duration::from_millis(200),
                                  Duration::from_millis(20)).unwrap_err().to_string();
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
                                  Duration::from_millis(20)).unwrap_err().to_string();
        h.join().unwrap();
        assert!(err.contains("exam-Z-effect2") && err.contains("quarantined"), "got {err}");
    }

    #[test]
    fn a_timed_out_job_no_player_claimed_is_withdrawn() {
        let t = root();
        let err = submit_and_wait(t.path(), "exam-W-effect0", "{}", Duration::from_millis(100),
                                  Duration::from_millis(20)).unwrap_err().to_string();
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

    #[test]
    fn the_backfill_line_names_what_it_left_out_readably() {
        let m = backfill_message(Path::new("s.yaml"), &["at 3: p1.hand".to_string()]);
        assert!(m.starts_with("wrote confirmed state into s.yaml"), "{m}");
        assert!(m.contains("left out 1 value(s)") && m.ends_with("at 3: p1.hand)"), "{m}");
        assert!(!m.contains("  "), "no runs of spaces: {m}");
        assert_eq!(backfill_message(Path::new("s.yaml"), &[]), "wrote confirmed state into s.yaml");
    }

    fn result(verdict: &str, refused: &[&str]) -> OracleExamResult {
        OracleExamResult {
            scenario: "s.yaml".into(),
            clause: "BT7-056#effect#0".into(),
            ids: vec!["BT7-056#effect#0".into()],
            verdict: verdict.into(),
            first_divergence: None,
            divergence: None,
            mismatch: None,
            reason: None,
            denominator: None,
            job_id: "exam-s".into(),
            job_outcome: Some("completed".into()),
            sidecar: None,
            backfilled: false,
            backfill_note: None,
            recorded: vec![],
            stall: None,
            refused: refused.iter().map(|s| s.to_string()).collect(),
            ours_vs_dcgo: None,
            ours_vs_ruling: None,
            ruling_q_id: None,
        }
    }

    #[test]
    fn the_three_way_legs_serialize_only_when_measured() {
        // The Python driver reads these keys (card-loop design D7 / follow-up
        // 9.8); an unmeasured line must not print `null` legs that a reader
        // could mistake for "disagree".
        let bare = serde_json::to_value(result("unmeasured", &[])).unwrap();
        assert!(bare.get("ours_vs_dcgo").is_none() && bare.get("ours_vs_ruling").is_none());
        assert!(bare.get("ruling_q_id").is_none());
        let mut r = result("diverged", &[]);
        r.ours_vs_dcgo = Some(true);
        r.ours_vs_ruling = Some(false);
        r.ruling_q_id = Some("Q1601".into());
        let v = serde_json::to_value(r).unwrap();
        assert_eq!(v["ours_vs_dcgo"], serde_json::json!(true));
        assert_eq!(v["ours_vs_ruling"], serde_json::json!(false));
        assert_eq!(v["ruling_q_id"], serde_json::json!("Q1601"));
    }

    #[test]
    fn a_confirmed_run_whose_verdict_was_refused_is_not_a_clean_confirmation() {
        // The orphan rule: a verdict keyed outside the denominator covers
        // nothing, whatever the diff said.
        assert!(result("confirmed", &[]).is_clean_confirmation());
        assert!(!result("confirmed", &["refusing ... orphan"]).is_clean_confirmation());
        assert!(!result("diverged", &[]).is_clean_confirmation());
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

#[cfg(test)]
mod oracle_exam_tests {
    //! The one-call loop against a fake oracle node: a temp harness root and a
    //! thread that plays DCGO's part by filing a real scripted-v19 recording
    //! and sidecar (tests/fixtures/oracle) for the submitted job. No Unity.
    use super::*;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    fn repo() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
    }

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/oracle").join(name)
    }

    /// A harness root: queue dirs, plus the enable marker and a fresh
    /// heartbeat when asked for.
    fn node(live: bool, enabled: bool) -> tempfile::TempDir {
        let t = tempfile::tempdir().unwrap();
        for d in ["jobs", "claimed", "done", "failed"] {
            std::fs::create_dir_all(t.path().join(d)).unwrap();
        }
        if enabled {
            std::fs::write(t.path().join("harness.enabled"), "test\n").unwrap();
        }
        if live {
            std::fs::write(t.path().join("harness.heartbeat"), "").unwrap();
        }
        t
    }

    /// What the fake player files for the job.
    struct Filing {
        outcome: &'static str,
        message: &'static str,
        /// Cut the sidecar to this many lines (DCGO stopped early).
        sidecar_lines: Option<usize>,
    }

    /// Plays DCGO: waits for `jobs/<job>.json`, files the fixture recording
    /// and sidecar beside each other, then the job's result.
    fn play(root: &Path, work: &Path, stem: &str, f: Filing) -> std::thread::JoinHandle<()> {
        let (root, work, stem) = (root.to_path_buf(), work.to_path_buf(), stem.to_string());
        std::thread::spawn(move || {
            let job = format!("exam-{stem}");
            let queued = root.join("jobs").join(format!("{job}.json"));
            for _ in 0..1000 {
                if queued.exists() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            std::fs::rename(&queued, root.join("done").join(format!("{job}.json"))).unwrap();
            let rec_dir = work.join("rec");
            std::fs::create_dir_all(&rec_dir).unwrap();
            let rec = rec_dir.join(format!("{stem}.jsonl"));
            std::fs::copy(fixture(&format!("{stem}.jsonl")), &rec).unwrap();
            let side = std::fs::read_to_string(fixture(&format!("{stem}.state.jsonl"))).unwrap();
            let side: String = match f.sidecar_lines {
                Some(n) => side.lines().take(n).map(|l| format!("{l}\n")).collect(),
                None => side,
            };
            std::fs::write(rec_dir.join(format!("{stem}.state.jsonl")), side).unwrap();
            let body = serde_json::json!({
                "job_id": job, "outcome": f.outcome, "recording_path": rec.display().to_string(),
                "steps": 4, "duration_seconds": 1.0, "message": f.message,
            });
            std::fs::write(root.join("done").join(format!("{job}.result.json")), body.to_string())
                .unwrap();
        })
    }

    struct Bench {
        root: tempfile::TempDir,
        work: tempfile::TempDir,
        cards: PathBuf,
        decks: PathBuf,
        clauses: PathBuf,
        none: PathBuf,
        store: PathBuf,
    }

    fn bench(live: bool, enabled: bool) -> Bench {
        let work = tempfile::tempdir().unwrap();
        let clauses = work.path().join("clauses.json");
        std::fs::write(
            &clauses,
            r#"{"clauses":[
            {"id":"ST23-06#effect#0","label":"[On Play]","text":"a"},
            {"id":"ST23-04#effect#0","label":"[On Play]","text":"b"}]}"#,
        )
        .unwrap();
        Bench {
            root: node(live, enabled),
            cards: repo().join("data/cards.json"),
            decks: repo().join("qa/dcgo-exams/ST23/glowing_dawn_pool.json"),
            clauses,
            none: work.path().join("no-denominator.json"),
            store: work.path().join("store"),
            work,
        }
    }

    impl Bench {
        fn opts(&self, timeout: Duration) -> OracleExamOptions<'_> {
            OracleExamOptions {
                root: self.root.path(),
                build: None,
                cards_json: &self.cards,
                decks: Some(&self.decks),
                verdicts_dir: Some(&self.store),
                clause_text_json: Some(&self.clauses),
                interaction_denominator: &self.none,
                backfill: true,
                timeout,
                poll: Duration::from_millis(20),
            }
        }
        fn scenario(&self, stem: &str) -> PathBuf {
            let p = self.work.path().join(format!("{stem}.yaml"));
            std::fs::copy(fixture(&format!("{stem}.yaml")), &p).unwrap();
            p
        }
        fn stored(&self, clause: &str) -> Option<crate::exam::verdict::ClauseVerdict> {
            VerdictStore::load_dir(&self.store).ok()?.get(clause).cloned()
        }
        fn play(&self, stem: &str, f: Filing) -> std::thread::JoinHandle<()> {
            play(self.root.path(), self.work.path(), stem, f)
        }
    }

    const COMPLETED: Filing = Filing { outcome: "completed", message: "", sidecar_lines: None };

    #[test]
    fn a_clean_run_confirms_records_provenance_and_backfills() {
        let b = bench(true, true);
        let sc = b.scenario("ST23-06-effect0");
        let h = b.play("ST23-06-effect0", COMPLETED);
        let r = run_oracle_exam(&sc, &b.opts(Duration::from_secs(60))).unwrap();
        h.join().unwrap();
        assert_eq!(r.verdict, "confirmed", "{r:?}");
        assert_eq!(r.job_id, "exam-ST23-06-effect0");
        assert!(r.backfilled, "{r:?}");
        assert!(std::fs::read_to_string(&sc).unwrap().contains("_backfilled"));
        let row = b.stored("ST23-06#effect#0").expect("verdict recorded");
        assert_eq!(row.verdict, Verdict::Confirmed);
        assert_eq!(row.job_id.as_deref(), Some("exam-ST23-06-effect0"));
        assert_eq!(r.recorded, vec!["ST23-06#effect#0".to_string()]);
    }

    #[test]
    fn a_diverged_run_is_recorded_diverged_and_never_backfilled() {
        let b = bench(true, true);
        let sc = b.scenario("ST23-04-effect0");
        let before = std::fs::read_to_string(&sc).unwrap();
        let h = b.play("ST23-04-effect0", COMPLETED);
        let r = run_oracle_exam(&sc, &b.opts(Duration::from_secs(60))).unwrap();
        h.join().unwrap();
        assert_eq!(r.verdict, "diverged", "{r:?}");
        assert!(
            r.first_divergence.as_deref().unwrap_or("").starts_with("DIVERGED at step 14"),
            "{r:?}"
        );
        assert!(!r.backfilled);
        assert_eq!(std::fs::read_to_string(&sc).unwrap(), before);
        assert_eq!(b.stored("ST23-04#effect#0").unwrap().verdict, Verdict::Diverged);
        // Spec 4.2(6): the first divergence as fields, not only a line of text.
        let d = r.divergence.expect("structured first divergence");
        assert_eq!(d.step, 14);
        assert!(!d.field.is_empty() && d.ours != d.dcgo, "{d:?}");
    }

    #[test]
    fn a_failed_job_that_diverged_before_it_stopped_is_diverged_with_dcgos_message() {
        let b = bench(true, true);
        let sc = b.scenario("ST23-04-effect0");
        let h = b.play(
            "ST23-04-effect0",
            Filing {
                outcome: "failed",
                message: "prompt mismatch: step 20 expected prompt 'main_phase' but DCGO asked 'SelectHandEffect'",
                sidecar_lines: None,
            },
        );
        let r = run_oracle_exam(&sc, &b.opts(Duration::from_secs(60))).unwrap();
        h.join().unwrap();
        assert_eq!(r.verdict, "diverged", "{r:?}");
        assert!(r.reason.as_deref().unwrap_or("").contains("prompt mismatch"), "{r:?}");
        assert_eq!(r.job_outcome.as_deref(), Some("failed"));
    }

    #[test]
    fn a_prompt_mismatch_names_the_scenario_step_not_dcgos_row() {
        // ST23-04-effect0 has sim-only rows, so DCGO's row index and the
        // scenario step drift apart; the harness knows the wire rows per step.
        let b = bench(true, true);
        let sc = b.scenario("ST23-04-effect0");
        let h = b.play(
            "ST23-04-effect0",
            Filing {
                outcome: "failed",
                message: "prompt mismatch: step 14 expected actor 0 but DCGO asked actor 1",
                sidecar_lines: None,
            },
        );
        let r = run_oracle_exam(&sc, &b.opts(Duration::from_secs(60))).unwrap();
        h.join().unwrap();
        let m = r.mismatch.expect("a prompt-mismatch job carries its mismatch");
        assert_eq!(m.row, 14);
        let s = crate::exam::scenario::Scenario::from_yaml(&std::fs::read_to_string(&sc).unwrap()).unwrap();
        assert!((m.step as usize) < s.steps.len(), "{m:?}");
        assert!(m.step >= 14, "sim-only rows before it push the step past the row: {m:?}");
        assert_eq!(m.expected, None);
        assert_eq!(m.asked, None);
    }

    #[test]
    fn a_failed_job_with_no_divergence_leaves_the_clause_unmeasured_and_unrecorded() {
        let b = bench(true, true);
        let sc = b.scenario("ST23-06-effect0");
        let h = b.play(
            "ST23-06-effect0",
            Filing {
                outcome: "failed",
                message: "prompt mismatch: step 2 expected prompt 'main_phase' but DCGO asked 'OptionalSkill'",
                sidecar_lines: Some(2),
            },
        );
        let r = run_oracle_exam(&sc, &b.opts(Duration::from_secs(60))).unwrap();
        h.join().unwrap();
        assert_eq!(r.verdict, "unmeasured", "{r:?}");
        assert!(r.reason.as_deref().unwrap_or("").contains("prompt mismatch"), "{r:?}");
        assert!(r.recorded.is_empty());
        assert!(b.stored("ST23-06#effect#0").is_none(), "an unmeasured run must not write a verdict");
    }

    #[test]
    fn a_timeout_is_unmeasured_with_the_reason() {
        let b = bench(true, true);
        let sc = b.scenario("ST23-06-effect0");
        let r = run_oracle_exam(&sc, &b.opts(Duration::from_millis(200))).unwrap();
        assert_eq!(r.verdict, "unmeasured");
        assert!(r.reason.as_deref().unwrap_or("").contains("timed out"), "{r:?}");
        assert!(b.stored("ST23-06#effect#0").is_none());
    }

    #[test]
    fn a_quarantined_job_is_unmeasured_with_the_reason() {
        let b = bench(true, true);
        let sc = b.scenario("ST23-06-effect0");
        let root = b.root.path().to_path_buf();
        let h = std::thread::spawn(move || {
            let q = root.join("jobs/exam-ST23-06-effect0.json");
            for _ in 0..1000 {
                if q.exists() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            std::fs::rename(&q, root.join("failed/exam-ST23-06-effect0.json")).unwrap();
        });
        let r = run_oracle_exam(&sc, &b.opts(Duration::from_secs(60))).unwrap();
        h.join().unwrap();
        assert_eq!(r.verdict, "unmeasured");
        assert!(r.reason.as_deref().unwrap_or("").contains("quarantined"), "{r:?}");
    }

    #[test]
    fn no_go_refuses_before_submitting() {
        let b = bench(true, false);
        let sc = b.scenario("ST23-06-effect0");
        let err = run_oracle_exam(&sc, &b.opts(Duration::from_secs(1))).unwrap_err();
        assert!(err.contains("NO-GO"), "{err}");
        assert_eq!(std::fs::read_dir(b.root.path().join("jobs")).unwrap().count(), 0);
    }

    #[test]
    fn no_live_player_refuses_before_submitting() {
        let b = bench(false, true);
        let sc = b.scenario("ST23-06-effect0");
        let err = run_oracle_exam(&sc, &b.opts(Duration::from_secs(1))).unwrap_err();
        assert!(err.contains("no live player"), "{err}");
        assert_eq!(std::fs::read_dir(b.root.path().join("jobs")).unwrap().count(), 0);
    }
}
