# Oracle Readiness Plan 3 — One-Call Oracle Loop, Introspection, and Lints

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** An agent gets an oracle verdict for a scenario in ONE call (CLI `exam --oracle` or MCP `exam_probe(sim_only: false)`), can see our pending prompt and board at any step, and gets deck-budget and ambiguous-play problems reported before a run.

**Architecture:** Move the job builder and the oracle diff out of the `dcgo-harness` binary (`main.rs`) into library modules so the CLI and the MCP server share one pipeline. A new `exam/oracle.rs` submits a job into the harness root, waits on `done/<job-id>.result.json`, derives the sidecar, diffs, records the verdict, and backfills. Introspection snapshots each lowered step. Lints extend `validate_yaml` with an optional deck book.

**Tech Stack:** Rust (`dcgo-harness`, serde, clap, tempfile).

**Spec:** `docs/superpowers/specs/2026-10-04-dcgo-oracle-readiness-design.md` §4.2, §4.3, §4.4. Depends on Plan 1 (verdict `dirty` writes, `--backfill`, `default_harness_root`).

## Global Constraints

- Worktree only; verify `git rev-parse --show-toplevel` ends in `.claude/worktrees/dcgo-effect-translation`.
- Per-worktree cargo target (`CARGO_TARGET_DIR="D:/cargo-target/dcgo-effect-translation"`).
- Recordings are located ONLY through `done/<job-id>.result.json`'s `recording_path` — never "newest file" (several agents share one queue).
- Before submitting a job, remove any previous `done/<job-id>.json` and `done/<job-id>.result.json` so a re-emitted job id cannot return a stale recording.
- `confirmed` only from a CLEAN oracle diff; refactors in Tasks 1–2 must not change CLI output byte-for-byte.
- Oracle preflight (`node::health`) runs before any submission and refuses on NO-GO.
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## File Map

| File | Change | Responsibility |
|---|---|---|
| `code/tools/dcgo-harness/src/exam/job_spec.rs` | Create (moved from `main.rs`) | `ExamJobSpec`, `ScriptedInput`, `build_exam_job`, `job_deck_top_first` |
| `code/tools/dcgo-harness/src/exam/oracle_diff.rs` | Create (moved from `main.rs`) | `decks_from_recording`, `diff_against_sidecar` |
| `code/tools/dcgo-harness/src/exam/oracle.rs` | Create | `submit_and_wait`, `run_oracle_exam` |
| `code/tools/dcgo-harness/src/exam/mod.rs` | Modify | Register modules |
| `code/tools/dcgo-harness/src/main.rs` | Modify | Use the library modules; `exam --oracle`; `--inspect` |
| `code/tools/dcgo-harness/src/exam/run.rs` | Modify | `run_one` oracle mode; step snapshots |
| `code/tools/dcgo-harness/src/mcp/handlers.rs`, `tools.rs` | Modify | `exam_probe`/`run_scenario` oracle + `inspect_step` params |
| `code/tools/dcgo-harness/src/exam/validate.rs` | Modify | `deck-budget` rule |
| `code/tools/dcgo-harness/src/exam/adapter.rs` | Modify | Ambiguous-play message suggests `hand.<i>` |
| `docs/DCGO_EXAM.md` | Modify | Document the one-call route |

---

### Task 1: Move the job builder into the library (no behavior change)

**Files:**
- Create: `code/tools/dcgo-harness/src/exam/job_spec.rs`
- Modify: `code/tools/dcgo-harness/src/exam/mod.rs`, `code/tools/dcgo-harness/src/main.rs`

**Interfaces:**
- Produces: `pub struct ExamJobSpec` (+ `ExamDeckOrder`, `ScriptedInput`, all fields `pub`), `pub fn job_deck_top_first(...)`, `pub fn build_exam_job(stem: &str, s: &Scenario, entry_p0: &DeckEntry, entry_p1: &DeckEntry, lowered: &[LoweredStep], owners: &[usize]) -> Result<ExamJobSpec, String>` — same signatures as today in `main.rs`, now `pub` in `dcgo_harness::exam::job_spec`.

- [ ] **Step 1: Capture today's output as the reference**

Run:
```bash
cargo build -p dcgo-harness
```
```bash
D:/cargo-target/dcgo-effect-translation/debug/dcgo-harness.exe exam --scenario qa/dcgo-exams/EX10/EX10-025-effect0.yaml --sim-only --cards-json data/cards.json --decks qa/dcgo-exams/EX10/rocks_pool.json --emit-job C:/Users/james/AppData/Local/Temp/job-ref
```
Expected: `C:/Users/james/AppData/Local/Temp/job-ref/exam-EX10-025-effect0.json` written.

- [ ] **Step 2: Move the code**

Cut from `main.rs`, in order: `struct ExamJobSpec`, `struct ExamDeckOrder`, `struct ScriptedInput` (with its `impl` blocks, if any), `fn job_deck_top_first`, `fn build_exam_job`, and every private helper that only they call (find with `grep -n "fn " code/tools/dcgo-harness/src/main.rs` between `struct ExamJobSpec` and the end of `build_exam_job`). Paste into a new `exam/job_spec.rs`, make the three structs, their fields, `job_deck_top_first` and `build_exam_job` `pub`, and add at the top the `use` lines they need (copy them from `main.rs`'s imports, rewriting `dcgo_harness::` to `crate::`). In `exam/mod.rs` add `pub mod job_spec;`. In `main.rs` add `use dcgo_harness::exam::job_spec::{build_exam_job, ExamJobSpec};` (plus anything else the remaining code names).

- [ ] **Step 3: Build, test, compare output byte-for-byte**

Run: `cargo test -p dcgo-harness`
Expected: PASS.

Run the Step 1 command again with `--emit-job C:/Users/james/AppData/Local/Temp/job-new`, then:
Run: `cmp C:/Users/james/AppData/Local/Temp/job-ref/exam-EX10-025-effect0.json C:/Users/james/AppData/Local/Temp/job-new/exam-EX10-025-effect0.json && echo IDENTICAL`
Expected: `IDENTICAL`.

- [ ] **Step 4: Commit**

```bash
git add code/tools/dcgo-harness/src/exam/job_spec.rs code/tools/dcgo-harness/src/exam/mod.rs code/tools/dcgo-harness/src/main.rs
git commit -m "harness: move the exam job builder into the library (no behavior change)" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Move the oracle diff into the library (no behavior change)

**Files:**
- Create: `code/tools/dcgo-harness/src/exam/oracle_diff.rs`
- Modify: `code/tools/dcgo-harness/src/exam/mod.rs`, `code/tools/dcgo-harness/src/main.rs` (`exam_one`, `decks_from_recording`)

**Interfaces:**
- Produces:

```rust
pub struct OracleDiff {
    pub report: crate::exam::differ::DiffReport,
    /// Our projections incl. the trailing state (what `assert:` talks about).
    pub projections: Vec<crate::exam::projection::StateProjection>,
    /// DCGO's aligned projections.
    pub dcgo: Vec<crate::exam::projection::StateProjection>,
}

/// Lower `scenario_text` against the decks recorded beside `sidecar`, align the
/// DCGO trace to the scenario origin, pair by lowered step, and diff.
pub fn diff_against_sidecar(
    scenario_text: &str,
    sidecar: &std::path::Path,
    card_data: &std::collections::HashMap<String, digimon_engine::CardData>,
) -> Result<OracleDiff, String>;

pub fn decks_from_recording(text: &str) -> Result<(Vec<String>, Vec<String>), String>;
```

- [ ] **Step 1: Capture today's oracle output as the reference**

Using the preserved Track A sidecar for `EX10-025-effect0` (path from `C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness/done/exam-EX10-025-effect0.result.json` → `recording_path`, with `.state.jsonl`):

Run: `D:/cargo-target/dcgo-effect-translation/debug/dcgo-harness.exe exam --scenario qa/dcgo-exams/EX10/EX10-025-effect0.yaml --sidecar "<sidecar>" --cards-json data/cards.json --all-diffs > C:/Users/james/AppData/Local/Temp/oracle-ref.txt`
Expected: the file contains `CLEAN (...)`.

- [ ] **Step 2: Move the code**

Move `fn decks_from_recording` from `main.rs` into `exam/oracle_diff.rs` as `pub fn`. Then extract, from `exam_one`'s oracle path, everything from building decks via `decks_from_recording` through `let report = diff_paired(&ours_for_diff, &dcgo, &pairing);` into `diff_against_sidecar`, returning `OracleDiff { report, projections, dcgo }`. The recording text it needs is read from the sidecar path with `.state.jsonl` replaced by `.jsonl` (the same derivation `exam_one` uses today). In `exam_one`, replace the moved block with:

```rust
    let od = dcgo_harness::exam::oracle_diff::diff_against_sidecar(&text, &sidecar_path, card_data)?;
    let (report, projections, dcgo) = (od.report, od.projections, od.dcgo);
```

keeping the print block, the `--backfill` block (Plan 1 Task 6) and the verdict-event construction exactly as they are. Register `pub mod oracle_diff;` in `exam/mod.rs`.

- [ ] **Step 3: Verify byte-identical output**

Run: `cargo test -p dcgo-harness`
Run the Step 1 command again into `oracle-new.txt`, then `cmp C:/Users/james/AppData/Local/Temp/oracle-ref.txt C:/Users/james/AppData/Local/Temp/oracle-new.txt && echo IDENTICAL`
Expected: PASS and `IDENTICAL`.

- [ ] **Step 4: Commit**

```bash
git add code/tools/dcgo-harness/src/exam/oracle_diff.rs code/tools/dcgo-harness/src/exam/mod.rs code/tools/dcgo-harness/src/main.rs
git commit -m "harness: move the oracle sidecar diff into the library (no behavior change)" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Submit a job and wait for its own result

**Files:**
- Create: `code/tools/dcgo-harness/src/exam/oracle.rs`
- Modify: `code/tools/dcgo-harness/src/exam/mod.rs`

**Interfaces:**
- Consumes: `crate::job::{JobResult, JobOutcome, DIR_JOBS, DIR_DONE, DIR_FAILED}` (`JobResult::from_json`).
- Produces:

```rust
pub struct OracleRun {
    pub job_id: String,
    pub recording_path: std::path::PathBuf,
    pub sidecar_path: std::path::PathBuf,
}

/// Write `job_json` to `<root>/jobs/<job_id>.json` (temp file + rename), after
/// removing any stale `done/<job_id>.json` / `done/<job_id>.result.json`; poll
/// for this job's own result until `timeout`.
pub fn submit_and_wait(
    root: &std::path::Path,
    job_id: &str,
    job_json: &str,
    timeout: std::time::Duration,
    poll: std::time::Duration,
) -> Result<OracleRun, String>;
```

- [ ] **Step 1: Write the failing tests** (bottom of `oracle.rs`, `#[cfg(test)] mod tests`)

```rust
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

    fn write_result(root: &std::path::Path, job: &str, outcome: &str, rec: &std::path::Path) {
        let body = serde_json::json!({
            "job_id": job, "outcome": outcome,
            "recording_path": rec.display().to_string(),
            "steps": 4, "duration_seconds": 1.0, "message": ""
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
            write_result(&r, "exam-X-effect0", "completed", &rec2);
        });
        let run = submit_and_wait(t.path(), "exam-X-effect0", "{}", Duration::from_secs(5),
                                  Duration::from_millis(20)).unwrap();
        h.join().unwrap();
        assert_eq!(run.sidecar_path, t.path().join("rec.state.jsonl"));
        assert!(t.path().join("jobs/exam-X-effect0.json").exists());
    }

    #[test]
    fn a_stale_result_is_removed_before_submitting() {
        let t = root();
        let rec = t.path().join("old.jsonl");
        write_result(t.path(), "exam-X-effect0", "completed", &rec);
        let err = submit_and_wait(t.path(), "exam-X-effect0", "{}", Duration::from_millis(200),
                                  Duration::from_millis(20)).unwrap_err();
        assert!(err.contains("timed out"), "stale result must not be returned; got {err}");
    }

    #[test]
    fn failed_outcome_is_an_error_naming_the_job() {
        let t = root();
        let r = t.path().to_path_buf();
        let h = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            write_result(&r, "exam-Y-effect1", "failed", std::path::Path::new("x.jsonl"));
        });
        let err = submit_and_wait(t.path(), "exam-Y-effect1", "{}", Duration::from_secs(5),
                                  Duration::from_millis(20)).unwrap_err();
        h.join().unwrap();
        assert!(err.contains("exam-Y-effect1") && err.contains("failed"), "got {err}");
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p dcgo-harness oracle::tests`
Expected: compile error — module/functions missing (after adding `pub mod oracle;` to `exam/mod.rs`).

- [ ] **Step 3: Implement**

```rust
//! One oracle round-trip for one job: submit, wait for THIS job's result,
//! locate its sidecar. Never "newest recording" -- several agents share a queue.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::job::{JobOutcome, JobResult};

pub struct OracleRun {
    pub job_id: String,
    pub recording_path: PathBuf,
    pub sidecar_path: PathBuf,
}

pub fn submit_and_wait(
    root: &Path,
    job_id: &str,
    job_json: &str,
    timeout: Duration,
    poll: Duration,
) -> Result<OracleRun, String> {
    let done = root.join(crate::job::DIR_DONE);
    let result_path = done.join(format!("{job_id}.result.json"));
    for stale in [done.join(format!("{job_id}.json")), result_path.clone()] {
        if stale.exists() {
            std::fs::remove_file(&stale)
                .map_err(|e| format!("removing stale {}: {e}", stale.display()))?;
        }
    }
    let jobs = root.join(crate::job::DIR_JOBS);
    std::fs::create_dir_all(&jobs).map_err(|e| format!("creating {}: {e}", jobs.display()))?;
    let tmp = jobs.join(format!(".{job_id}.json.tmp"));
    let dest = jobs.join(format!("{job_id}.json"));
    std::fs::write(&tmp, job_json).map_err(|e| format!("writing {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, &dest).map_err(|e| format!("submitting {}: {e}", dest.display()))?;

    let failed_dir = root.join(crate::job::DIR_FAILED);
    let started = Instant::now();
    loop {
        if result_path.exists() {
            let text = std::fs::read_to_string(&result_path)
                .map_err(|e| format!("reading {}: {e}", result_path.display()))?;
            let result = JobResult::from_json(&text)
                .map_err(|e| format!("parsing {}: {e}", result_path.display()))?;
            if result.outcome != JobOutcome::Completed {
                let outcome = format!("{:?}", result.outcome).to_lowercase();
                return Err(format!("oracle job {job_id} {outcome}: {}", result.message));
            }
            let recording_path = PathBuf::from(&result.recording_path);
            let sidecar_path = PathBuf::from(
                result.recording_path.trim_end_matches(".jsonl").to_string() + ".state.jsonl",
            );
            return Ok(OracleRun { job_id: job_id.to_string(), recording_path, sidecar_path });
        }
        if failed_dir.join(format!("{job_id}.json")).exists() {
            return Err(format!("oracle job {job_id} failed (moved to failed/)"));
        }
        if started.elapsed() >= timeout {
            return Err(format!(
                "oracle job {job_id} timed out after {}s (is the player running? `node status`)",
                timeout.as_secs()
            ));
        }
        std::thread::sleep(poll);
    }
}
```

If `JobResult::from_json` returns a different error type or `JobOutcome` lacks `PartialEq`, adapt the two lines (derive `PartialEq` on `JobOutcome` in `job.rs` if needed).

- [ ] **Step 4: Run tests**

Run: `cargo test -p dcgo-harness oracle::tests`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add code/tools/dcgo-harness/src/exam/oracle.rs code/tools/dcgo-harness/src/exam/mod.rs code/tools/dcgo-harness/src/job.rs
git commit -m "harness: submit an exam job and wait for its own result" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: `run_oracle_exam` + CLI `exam --oracle`

**Files:**
- Modify: `code/tools/dcgo-harness/src/exam/oracle.rs`
- Modify: `code/tools/dcgo-harness/src/main.rs`

**Interfaces:**
- Produces:

```rust
#[derive(Debug, serde::Serialize)]
pub struct OracleExamResult {
    pub scenario: String,
    pub clause: String,
    pub verdict: String,            // "confirmed" | "diverged"
    pub first_divergence: Option<String>,
    pub denominator: String,        // DiffReport::denominator()
    pub job_id: String,
    pub sidecar: String,
    pub backfilled: bool,
}

pub struct OracleExamOptions<'a> {
    pub root: &'a Path,
    pub build: Option<&'a Path>,
    pub cards_json: &'a Path,
    pub decks: Option<&'a Path>,
    pub verdicts_dir: Option<&'a Path>,
    pub clause_text_json: Option<&'a Path>,
    pub backfill: bool,
    pub timeout: Duration,
}

pub fn run_oracle_exam(scenario: &Path, opts: &OracleExamOptions) -> Result<OracleExamResult, String>;
```

- [ ] **Step 1: Add verdict provenance to the store**

In `exam/verdict.rs`, inside `impl VerdictStore`, add (and a unit test that records a row, calls it, and checks both fields):

```rust
    /// Stamp the oracle provenance on a recorded row and mark its card dirty.
    pub fn set_provenance(
        &mut self,
        clause_id: &str,
        job_id: Option<String>,
        dcgo_build: Option<String>,
    ) -> Result<(), String> {
        let row = self
            .clauses
            .get_mut(clause_id)
            .ok_or_else(|| format!("no stored verdict for `{clause_id}`"))?;
        row.job_id = job_id;
        row.dcgo_build = dcgo_build;
        self.dirty.insert(row.card_id.clone());
        Ok(())
    }
```

- [ ] **Step 2: Implement `run_oracle_exam`**

Append to `exam/oracle.rs` (the `load_card_data_at` call mirrors the one in `main.rs::run_exam`; if its signature differs, copy that call exactly):

```rust
use crate::exam::verdict::{record_scenario_verdict, ClauseTextBook, Verdict, VerdictStore};

#[derive(Debug, serde::Serialize)]
pub struct OracleExamResult {
    pub scenario: String,
    pub clause: String,
    pub verdict: String,
    pub first_divergence: Option<String>,
    pub denominator: String,
    pub job_id: String,
    pub sidecar: String,
    pub backfilled: bool,
}

pub struct OracleExamOptions<'a> {
    pub root: &'a Path,
    pub build: Option<&'a Path>,
    pub cards_json: &'a Path,
    pub decks: Option<&'a Path>,
    pub verdicts_dir: Option<&'a Path>,
    pub clause_text_json: Option<&'a Path>,
    pub backfill: bool,
    pub timeout: Duration,
}

fn build_commit(build: Option<&Path>) -> Option<String> {
    let text = std::fs::read_to_string(build?.join("manifest.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    v.get("dcgo_commit").and_then(|c| c.as_str()).map(str::to_string)
}

pub fn run_oracle_exam(scenario: &Path, opts: &OracleExamOptions) -> Result<OracleExamResult, String> {
    let h = crate::node::health(opts.root, opts.build);
    if !h.go {
        return Err(format!("oracle NO-GO:\n{}", h.describe()));
    }
    let text = std::fs::read_to_string(scenario)
        .map_err(|e| format!("reading {}: {e}", scenario.display()))?;
    let s = crate::exam::scenario::Scenario::from_yaml(&text)?;
    let card_data = dcgo_replay::load_card_data_at(opts.cards_json)?;
    let book = crate::exam::deckbook::DeckBook::load(opts.decks, opts.cards_json)?;
    let p0 = crate::exam::deckbook::ordered_deck(&s.decks.p0, &book)?;
    let p1 = crate::exam::deckbook::ordered_deck(&s.decks.p1, &book)?;
    let lowered = crate::exam::run::lower_and_run(&s, p0, p1, &card_data)?;
    if !lowered.complete {
        return Err(format!("line does not run in our engine: {:?}", lowered.stall_reasons));
    }
    let stem = scenario
        .file_stem()
        .and_then(|x| x.to_str())
        .ok_or("scenario has no file stem")?;
    let job = crate::exam::job_spec::build_exam_job(
        stem,
        &s,
        book.resolve(&s.decks.p0.rest)?,
        book.resolve(&s.decks.p1.rest)?,
        &lowered.lowered_steps,
        &lowered.lowered_owners,
    )?;
    let json = serde_json::to_string_pretty(&job).map_err(|e| e.to_string())? + "\n";
    let run = submit_and_wait(opts.root, &job.job_id, &json, opts.timeout, Duration::from_secs(2))?;
    let od = crate::exam::oracle_diff::diff_against_sidecar(&text, &run.sidecar_path, &card_data)?;
    let clean = od.report.is_clean();

    let mut backfilled = false;
    if opts.backfill && clean {
        let updated = crate::exam::backfill::backfill_from_diff(&text, &od.projections, &od.report)?;
        std::fs::write(scenario, updated)
            .map_err(|e| format!("writing backfilled {}: {e}", scenario.display()))?;
        backfilled = true;
    }

    let first_divergence = (!clean)
        .then(|| format!("{}", od.report).lines().next().unwrap_or_default().to_string());
    if let (Some(dir), Some(ctj)) = (opts.verdicts_dir, opts.clause_text_json) {
        let clause_book = ClauseTextBook::load(ctj)?;
        let mut store = VerdictStore::load_dir(dir)?;
        record_scenario_verdict(
            &mut store,
            &clause_book,
            &s.clause,
            if clean { Verdict::Confirmed } else { Verdict::Diverged },
            Some(scenario.display().to_string()),
            first_divergence.clone(),
            chrono::Utc::now().to_rfc3339(),
        )?;
        store.set_provenance(&s.clause, Some(run.job_id.clone()), build_commit(opts.build))?;
        store.save_dir(dir)?;
    }

    Ok(OracleExamResult {
        scenario: scenario.display().to_string(),
        clause: s.clause.clone(),
        verdict: if clean { "confirmed" } else { "diverged" }.to_string(),
        first_divergence,
        denominator: od.report.denominator(),
        job_id: run.job_id,
        sidecar: run.sidecar_path.display().to_string(),
        backfilled,
    })
}
```

If `dcgo_replay` is not a dependency of the library target (only of the binary), add it to `[dependencies]` in `code/tools/dcgo-harness/Cargo.toml` with the same spec the binary uses.

- [ ] **Step 3: CLI flag**

In `Command::Exam` add:

```rust
        /// Submit each scenario to the oracle, wait, diff, record (one call).
        /// Needs --root or a default harness root (see `node::default_harness_root`).
        #[arg(long)]
        oracle: bool,
        /// Oracle player build directory, for the preflight check.
        #[arg(long)]
        build: Option<PathBuf>,
        /// Seconds to wait for each oracle job.
        #[arg(long, default_value_t = 300)]
        oracle_timeout: u64,
```

In the `Command::Exam` dispatch, before calling `run_exam`, add a branch:

```rust
            if *oracle {
                let root = args.root.clone()
                    .or_else(dcgo_harness::node::default_harness_root)
                    .ok_or("--oracle needs --root or DCGO_HARNESS_ROOT")?;
                let opts = dcgo_harness::exam::oracle::OracleExamOptions {
                    root: &root,
                    build: build.as_deref(),
                    cards_json,
                    decks: decks.as_deref(),
                    verdicts_dir: verdicts.as_deref(),
                    clause_text_json: clause_text_json.as_deref(),
                    backfill: *backfill,
                    timeout: std::time::Duration::from_secs(*oracle_timeout),
                };
                let mut any_diverged = false;
                for path in collect_scenario_paths(scenario)? {
                    let r = dcgo_harness::exam::oracle::run_oracle_exam(&path, &opts)?;
                    any_diverged |= r.verdict != "confirmed";
                    println!("{}", serde_json::to_string(&r).map_err(|e| e.to_string())?);
                }
                return Ok(if any_diverged { ExitCode::from(1) } else { ExitCode::SUCCESS });
            }
```

Refuse `--oracle` together with `--sim-only` or `--sidecar` (add a check printing a clear error).

- [ ] **Step 4: Build and test**

Run: `cargo test -p dcgo-harness`
Expected: PASS.

- [ ] **Step 5: Live acceptance (needs the oracle node)**

Run: `D:/cargo-target/dcgo-effect-translation/debug/dcgo-harness.exe --root "C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness" exam --oracle --build D:/dcgo-build/scripted-v17 --scenario qa/dcgo-exams/EX8/EX8-067-effect0.yaml --cards-json data/cards.json --decks qa/dcgo-exams/EX10/rocks_pool.json`
Expected: one JSON line with `"verdict":"confirmed"` within ~60 s and no other commands.

- [ ] **Step 6: Commit**

```bash
git add code/tools/dcgo-harness/src/exam/oracle.rs code/tools/dcgo-harness/src/exam/verdict.rs code/tools/dcgo-harness/src/main.rs code/tools/dcgo-harness/Cargo.toml
git commit -m "harness: exam --oracle submits, waits, diffs, and records in one call" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: MCP `exam_probe` / `run_scenario` reach the oracle

**Files:**
- Modify: `code/tools/dcgo-harness/src/exam/run.rs` (`run_one`)
- Modify: `code/tools/dcgo-harness/src/mcp/handlers.rs`, `code/tools/dcgo-harness/src/mcp/tools.rs`

- [ ] **Step 1: Wire oracle mode**

In `run_one` (`exam/run.rs` ~L175), replace the unconditional `Err("oracle mode (sim_only: false) needs a DCGO state sidecar ...")` with a call to `crate::exam::oracle::run_oracle_exam(path, &opts)` where `opts.root` is the passed root or `crate::node::default_harness_root()`, `cards_json`/`decks` use the existing `DEFAULT_CARDS_JSON` / deck-pool resolution, `verdicts_dir` = `Some("qa/qa-reports/exam-verdicts")`, `clause_text_json` = `Some("qa/exam-clause-text.json")`, `backfill: true`, `timeout: 300 s`. Change `run_one`'s return type to an enum or `serde_json::Value` so the sim and oracle payloads can both be returned; update its two callers in `handlers.rs` (`run_scenario`, `exam_probe`) to pass the payload through. In `exam_probe`, keep the scratch-file pattern but write the scratch scenario under `qa/dcgo-exams/_probe/` (not the temp dir) so its job id is stable and `--backfill` has a real file; delete it afterwards.

- [ ] **Step 2: Update tool descriptions**

In `tools.rs`, change the `exam_probe` and `run_scenario` descriptions: `sim_only=false` now "submits to the oracle node, waits for this job's own result, diffs, records the verdict and backfills asserts (one call, ~15-60 s); preflights node health first". Add an optional `"timeout_seconds"` integer param to both.

- [ ] **Step 3: Test**

Add a handler test using a temp root with no player (no `harness.enabled`): `run_scenario` with `sim_only: false` must return `isError` text containing `NO-GO`. Run: `cargo test -p dcgo-harness`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add code/tools/dcgo-harness/src/exam/run.rs code/tools/dcgo-harness/src/mcp/handlers.rs code/tools/dcgo-harness/src/mcp/tools.rs
git commit -m "mcp: exam_probe and run_scenario reach the oracle in one call" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Introspection — pending prompt and board at step N

**Files:**
- Modify: `code/tools/dcgo-harness/src/exam/run.rs` (`LoweredRun`, `lower_and_run`)
- Modify: `code/tools/dcgo-harness/src/main.rs` (`--inspect`), `mcp/handlers.rs` + `tools.rs` (`inspect_step`)

**Interfaces:**
- Produces:

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct StepSnapshot {
    pub step: usize,
    /// Debug-formatted SelectionKind of the prompt live BEFORE this step, if any.
    pub pending_kind: Option<String>,
    pub pending_optional: Option<bool>,
    pub pending_prompt: Option<String>,
    /// Legal action ids with the card identity each addresses, when resolvable.
    pub candidates: Vec<(u16, Option<String>)>,
}
```

`LoweredRun` gains `pub snapshots: Vec<StepSnapshot>`.

- [ ] **Step 1: Write the failing test** (in `run.rs` tests, using `test_support::load_card_data()` and an existing small scenario, e.g. `qa/dcgo-exams/EX10/EX10-025-effect0.yaml` with `qa/dcgo-exams/EX10/rocks_pool.json`)

```rust
    #[test]
    fn lowered_run_snapshots_every_step() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let yaml = std::fs::read_to_string(root.join("qa/dcgo-exams/EX10/EX10-025-effect0.yaml")).unwrap();
        let s = crate::exam::scenario::Scenario::from_yaml(&yaml).unwrap();
        let book = crate::exam::deckbook::DeckBook::load(
            Some(root.join("qa/dcgo-exams/EX10/rocks_pool.json").as_path()),
            root.join("data/cards.json").as_path(),
        ).unwrap();
        let p0 = crate::exam::deckbook::ordered_deck(&s.decks.p0, &book).unwrap();
        let p1 = crate::exam::deckbook::ordered_deck(&s.decks.p1, &book).unwrap();
        let run = lower_and_run(&s, p0, p1, &crate::exam::test_support::load_card_data()).unwrap();
        assert_eq!(run.snapshots.len(), s.steps.len());
        assert!(run.snapshots.iter().any(|sn| sn.pending_kind.is_some()),
                "a line with select rows must show at least one live prompt");
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p dcgo-harness lowered_run_snapshots_every_step`
Expected: compile error — `snapshots` missing.

- [ ] **Step 3: Implement**

In `lower_and_run`'s per-step loop (find it with `grep -n "for (i, step)" code/tools/dcgo-harness/src/exam/run.rs code/tools/dcgo-harness/src/exam/adapter.rs`), push a snapshot BEFORE applying step `i`:

```rust
        snapshots.push(StepSnapshot {
            step: i,
            pending_kind: game.pending_selection.as_ref().map(|p| format!("{:?}", p.kind)),
            pending_optional: game.pending_selection.as_ref().map(|p| p.is_optional),
            pending_prompt: game.pending_selection.as_ref().map(|p| p.prompt.clone()),
            candidates: game
                .pending_selection
                .as_ref()
                .map(|p| p.valid_action_ids.iter().map(|id| (*id, None)).collect())
                .unwrap_or_default(),
        });
```

and return `snapshots` in `LoweredRun`. (Card identities per id are a follow-up; ids alone already answer "what is live here".)

CLI: add `#[arg(long)] inspect: Option<usize>` to `Command::Exam` (sim-only only); after lowering, print `serde_json::to_string_pretty(&run.snapshots[n])` plus the projection at step `n`. MCP: add optional `inspect_step` to `exam_probe`; when set, include `"inspect": {snapshot, projection}` in the sim payload.

- [ ] **Step 4: Run tests**

Run: `cargo test -p dcgo-harness`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add code/tools/dcgo-harness/src
git commit -m "exam: snapshot the live prompt at every lowered step; --inspect / inspect_step" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Lints — deck budget and ambiguous plays

**Files:**
- Modify: `code/tools/dcgo-harness/src/exam/validate.rs`
- Modify: `code/tools/dcgo-harness/src/exam/adapter.rs` (the two `LowerError::Ambiguous` message sites, ~L464 and ~L563)
- Modify: `code/tools/dcgo-harness/src/mcp/handlers.rs` (`exam_validate` passes a deck book when `decks` is given)

**Interfaces:**
- Produces: `pub fn validate_yaml_with_decks(text: &str, known_clause_ids: Option<&[String]>, book: Option<&DeckBook>) -> Vec<Finding>`; `validate_yaml` delegates with `book: None`. New rule id `deck-budget`.

- [ ] **Step 1: Write the failing test** (in `validate.rs` tests)

```rust
    #[test]
    fn deck_budget_flags_more_stacked_copies_than_the_deck_holds() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let book = crate::exam::deckbook::DeckBook::load(
            Some(root.join("qa/dcgo-exams/EX10/rocks_pool.json").as_path()),
            root.join("data/cards.json").as_path(),
        ).unwrap();
        let yaml = r#"
card: EX10-025
clause: EX10-025#effect#0
seed: 1
decks:
  p0: { stack: [EX10-025, EX10-025, EX10-025, EX10-025, EX10-025], rest: rocks-exam }
  p1: { stack: [], rest: rocks-exam }
steps: []
"#;
        let findings = validate_yaml_with_decks(yaml, None, Some(&book));
        assert!(findings.iter().any(|f| f.rule == "deck-budget"), "{findings:?}");
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p dcgo-harness deck_budget_flags`
Expected: compile error — function missing.

- [ ] **Step 3: Implement**

`validate_yaml_with_decks` runs `validate_yaml`'s existing rules, then for each seat `p0`/`p1` with a `rest:` name: `book.resolve(rest)` → count each id in `entry.main` + `entry.eggs`; count each id in `stack:`; for any id where stacked > available push `Finding { rule: "deck-budget".into(), message: format!("p{n} stacks {k}x {id} but deck `{rest}` holds {m}"), guide_topic: "decks".into() }`; an unknown `rest:` pushes `deck-budget` with the resolver's error text.

In `adapter.rs`, in both `LowerError::Ambiguous { intent, matches }` arms, append a hint when every match is a hand play id:

```rust
                        LowerError::Ambiguous { intent, matches } => {
                            let hand: Vec<String> = matches
                                .iter()
                                .filter_map(|id| id.checked_sub(digimon_engine::action::space::PLAY_HAND_START))
                                .filter(|i| *i < 64)
                                .map(|i| format!("hand.{i}"))
                                .collect();
                            let hint = if hand.len() == matches.len() {
                                format!(" Pin one: {}.", hand.join(", "))
                            } else {
                                String::new()
                            };
                            format!(
                                "step {i}: {intent} is ambiguous -- {matches:?} all match. \
                                 Narrow the step; picking arbitrarily would silently answer \
                                 a different question than the scenario asks.{hint}"
                            )
                        }
```

(Use the action-space path the adapter already imports for `PLAY_HAND_START`.) In `handlers.rs::exam_validate`, accept an optional `decks` param: when present, `DeckBook::load(Some(decks), "data/cards.json")` and call `validate_yaml_with_decks`.

- [ ] **Step 4: Run tests**

Run: `cargo test -p dcgo-harness`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add code/tools/dcgo-harness/src/exam/validate.rs code/tools/dcgo-harness/src/exam/adapter.rs code/tools/dcgo-harness/src/mcp/handlers.rs
git commit -m "exam: deck-budget lint and hand.<i> hints for ambiguous plays" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Document the one-call route

**Files:**
- Modify: `docs/DCGO_EXAM.md`

- [ ] **Step 1: Update the doc**

In "Quick start", add first:

```bash
# One call: preflight, submit, wait for this job's own result, diff, record, backfill.
dcgo-harness --root "$ROOT" exam --oracle --build <player-dir> \
    --scenario qa/dcgo-exams/EX10/EX10-025-effect0.yaml --cards-json data/cards.json \
    --decks qa/dcgo-exams/EX10/rocks_pool.json \
    --verdicts --clause-text-json qa/exam-clause-text.json --backfill
```

In "Running a campaign → The oracle route today", replace the paragraph stating `exam_probe(sim_only: false)` is not wired with: "`exam_probe(sim_only: false)` and `exam --oracle` submit the scenario, wait on `done/<job-id>.result.json`, diff against that job's own sidecar, record the verdict, and backfill asserts — one call. `G-TOOLING-EXAM-PROBE-NO-ORACLE-MODE` is closed." Also add `--inspect N` / `inspect_step` and the `deck-budget` lint to "The agent surface (MCP)" table.

- [ ] **Step 2: Commit**

```bash
git add docs/DCGO_EXAM.md
git commit -m "docs: the one-call oracle route, --inspect, and the deck-budget lint" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
