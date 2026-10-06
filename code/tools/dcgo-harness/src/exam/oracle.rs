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
    let claimed = root.join(crate::job::DIR_CLAIMED).join(format!("{job_id}.json"));
    if claimed.exists() {
        return Err(format!(
            "an earlier {job_id} is still running on a player ({} exists): submitting now \
             would hand back THAT run's result. Wait for it to finish, or clear an orphaned \
             claim with `dcgo-harness --root <root> status --sweep`.",
            claimed.display()
        ));
    }
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
                        return Err(e);
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
/// (or `... expected actor A but DCGO asked actor B`) into
/// `(row, expected prompt, asked prompt)`.
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
        _ => (None, None),
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
    };

    let run = match submit_and_wait(opts.root, &job.job_id, &json, opts.timeout, opts.poll) {
        Ok(run) => run,
        Err(e) => {
            result.reason = Some(e);
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

    let mut event = VerdictEvent::from_diff(
        &s,
        &od.report,
        ruling_agrees(&s, &od.projections),
        scenario.display().to_string(),
    );
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
    fn a_previous_submission_still_claimed_is_refused_not_adopted() {
        // A timed-out earlier run of the same stem still on the player would
        // file ITS result first, and we would diff the new line against it.
        let t = root();
        std::fs::write(t.path().join("claimed/exam-X-effect0.json"), "{}").unwrap();
        let started = std::time::Instant::now();
        let err = submit_and_wait(t.path(), "exam-X-effect0", "{}", Duration::from_secs(5),
                                  Duration::from_millis(20)).unwrap_err();
        assert!(started.elapsed() < Duration::from_secs(1), "refused up front, not after waiting");
        assert!(err.contains("still running") && err.contains("exam-X-effect0.json"), "{err}");
        assert!(!t.path().join("jobs/exam-X-effect0.json").exists(), "nothing may be submitted");
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
            refused: refused.iter().map(|s| s.to_string()).collect(),
        }
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
