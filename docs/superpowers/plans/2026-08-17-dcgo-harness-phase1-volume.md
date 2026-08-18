# DCGO Harness Phase 1 (Volume) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let DCGO play unattended bot-vs-bot games from a filesystem job queue, so a recording corpus can be generated and triaged without a human playing anything.

**Architecture:** DCGO gains a `Digimon.Harness` namespace whose `JobWatcher` polls a jobs directory, claims a job by atomic rename, applies it (decks, seed, auto mode, time scale), plays the game, and files the result. A Rust CLI (`dcgo-harness`) submits jobs, reports queue status, and triages the resulting corpus through the existing `dcgo-replay` core.

**Tech Stack:** Rust (clap 4, serde, serde_json) for the host CLI; C# / Unity `JsonUtility` for the DCGO side; existing `dcgo-replay` crate as a library dependency.

## Global Constraints

- **Spec:** `docs/superpowers/specs/2026-08-17-dcgo-automation-harness-design.md`. Phase 1 only — no `DeckStacker`, no `scripted`/`recorded` policies, no `StateDumper`.
- **DCGO lives in the base repo** (CLAUDE.md rule 29). Edit it at `$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO`. Never `git submodule update --init DCGO` from a worktree.
- **Per-worktree cargo target** (rule 31). Prefix cargo commands with `CARGO_TARGET_DIR='D:\cargo-target-wt\<worktree-name>'`.
- **All source under `code/`** (rule 24). The new crate is `code/tools/dcgo-harness/`.
- **Jobs carry card ID strings**, never DCGO deck codes. DCGO owns the encoding.
- **Determinism is required**: every job sets `UnityEngine.Random.InitState(job.seed)` before the game starts.
- **No silent skipping**: any status or triage output states submitted / completed / partial / failed counts.
- Rust edition 2021. C# must compile under Unity's C# 9 (no file-scoped namespaces, no records).

---

### Task 1: Job spec types and directory protocol (Rust)

**Files:**
- Create: `code/tools/dcgo-harness/Cargo.toml`
- Create: `code/tools/dcgo-harness/src/lib.rs`
- Create: `code/tools/dcgo-harness/src/job.rs`
- Modify: `Cargo.toml` (workspace members list, after `"code/tools/dcgo-replay",`)

**Interfaces:**
- Consumes: nothing.
- Produces: `dcgo_harness::job::{JobSpec, JobDecks, JobLimits, JobResult, JobOutcome}`; `JobSpec::to_json(&self) -> Result<String, String>`; `JobSpec::from_json(&str) -> Result<JobSpec, String>`; `JobResult::from_json(&str) -> Result<JobResult, String>`.

- [ ] **Step 1: Write the failing test**

Create `code/tools/dcgo-harness/src/job.rs` containing only this test module for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_spec_round_trips_through_json() {
        let spec = JobSpec {
            job_id: "vol-0042".to_string(),
            policy: "ai".to_string(),
            decks: JobDecks {
                p0: vec!["EX12-035".to_string(), "EX12-001".to_string()],
                p1: vec!["BT16-082".to_string()],
            },
            first_player: 0,
            seed: 12345,
            limits: JobLimits {
                max_turns: 40,
                timeout_seconds: 180,
            },
        };
        let text = spec.to_json().expect("serialize");
        let back = JobSpec::from_json(&text).expect("deserialize");
        assert_eq!(back.job_id, "vol-0042");
        assert_eq!(back.seed, 12345);
        assert_eq!(back.decks.p0.len(), 2);
        assert_eq!(back.limits.max_turns, 40);
    }

    #[test]
    fn job_spec_tolerates_unknown_fields() {
        // Forward compatibility: a phase-2 job carrying `deck_order` must not
        // break a phase-1 reader.
        let text = r#"{
            "job_id": "vol-1",
            "policy": "ai",
            "decks": {"p0": ["EX12-035"], "p1": ["BT16-082"]},
            "first_player": 1,
            "seed": 7,
            "limits": {"max_turns": 40, "timeout_seconds": 180},
            "deck_order": {"p0": ["EX12-035"]},
            "dump_state": true
        }"#;
        let spec = JobSpec::from_json(text).expect("unknown fields must be ignored");
        assert_eq!(spec.first_player, 1);
        assert_eq!(spec.seed, 7);
    }

    #[test]
    fn job_result_parses_outcomes() {
        let text = r#"{
            "job_id": "vol-1",
            "outcome": "completed",
            "recording_path": "recordings/x.jsonl",
            "steps": 61,
            "duration_seconds": 12.5,
            "message": ""
        }"#;
        let result = JobResult::from_json(text).expect("parse");
        assert_eq!(result.outcome, JobOutcome::Completed);
        assert_eq!(result.steps, 61);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
CARGO_TARGET_DIR='D:\cargo-target-wt\quizzical-ishizaka-07b190' cargo test -p dcgo-harness
```

Expected: FAIL — `error: package ID specification 'dcgo-harness' did not match any packages`.

- [ ] **Step 3: Create the crate and register it in the workspace**

`code/tools/dcgo-harness/Cargo.toml`:

```toml
[package]
name = "dcgo-harness"
version = "0.1.0"
edition = "2021"
description = "Submits DCGO harness jobs, reports queue status, and triages the resulting recording corpus. Phase 1 of the DCGO automation harness."

[lib]
path = "src/lib.rs"

[[bin]]
name = "dcgo-harness"
path = "src/main.rs"

[dependencies]
dcgo-replay = { path = "../dcgo-replay" }
digimon-engine = { path = "../../digimon-engine" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }
```

In the root `Cargo.toml`, add to `members` immediately after `"code/tools/dcgo-replay",`:

```toml
    "code/tools/dcgo-harness",
```

`code/tools/dcgo-harness/src/lib.rs`:

```rust
//! Host side of the DCGO automation harness: job submission, queue status,
//! and corpus triage. The DCGO client itself only reads and writes files in
//! the job directories — see
//! `docs/superpowers/specs/2026-08-17-dcgo-automation-harness-design.md`.

pub mod job;
```

- [ ] **Step 4: Write the job types**

Prepend to `code/tools/dcgo-harness/src/job.rs`, above the test module:

```rust
//! Job spec and result types, plus the directory protocol.
//!
//! A job is one JSON file whose *state is its directory*:
//! `jobs/` → (atomic rename) `claimed/` → `done/` or `failed/`.
//! Atomic rename is what makes crash recovery legible — a job sitting in
//! `claimed/` past its timeout is a hung Unity run, not a lost one.

use serde::{Deserialize, Serialize};

/// Card ID lists for both seats. Deliberately NOT DCGO deck codes: the code is
/// a base-n encoding over DCGO's internal `CEntity_Base.CardIndex`, so DCGO
/// owns the encoding and we send identities it resolves itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDecks {
    pub p0: Vec<String>,
    pub p1: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobLimits {
    /// Abandon the game past this many turns. Guards against a bot that loops
    /// forever; the partial recording is still filed and still useful.
    pub max_turns: u32,
    /// Wall-clock budget. The CLI (not Unity) enforces this by age of the
    /// claimed file, so a hung Unity process is still detected.
    pub timeout_seconds: u64,
}

/// One unattended DCGO game. Phase 1 only emits `policy: "ai"`; the field
/// exists so phase-2 scripted jobs share the reader.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSpec {
    pub job_id: String,
    pub policy: String,
    pub decks: JobDecks,
    pub first_player: u8,
    /// Seeds `UnityEngine.Random.InitState` so the game is reproducible. A
    /// divergence found in game 137 of an overnight batch is worthless if it
    /// cannot be re-run.
    pub seed: u64,
    pub limits: JobLimits,
}

impl JobSpec {
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    pub fn from_json(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| e.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobOutcome {
    /// Game reached its natural end and the recording has a `game_end` row.
    Completed,
    /// Game was abandoned (turn cap); the recording is usable but truncated.
    Partial,
    /// DCGO could not run the job (bad deck, crash).
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    pub job_id: String,
    pub outcome: JobOutcome,
    pub recording_path: String,
    pub steps: u32,
    pub duration_seconds: f64,
    /// Failure detail. Empty on success.
    #[serde(default)]
    pub message: String,
}

impl JobResult {
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    pub fn from_json(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| e.to_string())
    }
}

/// The four job directories, relative to the harness root.
pub const DIR_JOBS: &str = "jobs";
pub const DIR_CLAIMED: &str = "claimed";
pub const DIR_DONE: &str = "done";
pub const DIR_FAILED: &str = "failed";
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
CARGO_TARGET_DIR='D:\cargo-target-wt\quizzical-ishizaka-07b190' cargo test -p dcgo-harness
```

Expected: PASS — `test result: ok. 3 passed`.

Note: the `[[bin]]` target has no `main.rs` yet, so `cargo build` fails until Task 2. `cargo test -p dcgo-harness --lib` passes now; if the bin target blocks the run, temporarily comment out the `[[bin]]` block and restore it in Task 2.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml code/tools/dcgo-harness
git commit -m "dcgo-harness: job spec types and directory protocol"
```

---

### Task 2: `submit` — sample deck pairs into job files (Rust)

**Files:**
- Create: `code/tools/dcgo-harness/src/pool.rs`
- Create: `code/tools/dcgo-harness/src/main.rs`
- Modify: `code/tools/dcgo-harness/src/lib.rs`

**Interfaces:**
- Consumes: `job::{JobSpec, JobDecks, JobLimits, DIR_JOBS}` from Task 1.
- Produces: `pool::{DeckPool, PooledDeck}`; `pool::load_pool(path: &Path) -> Result<DeckPool, String>`; `pool::build_jobs(pool: &DeckPool, count: u32, base_seed: u64, limits: &JobLimits) -> Result<Vec<JobSpec>, String>`.

- [ ] **Step 1: Write the failing test**

Create `code/tools/dcgo-harness/src/pool.rs` with only this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::JobLimits;

    fn pool() -> DeckPool {
        DeckPool {
            decks: vec![
                PooledDeck { name: "a".into(), cards: vec!["EX12-035".into()], eggs: vec!["EX12-001".into()] },
                PooledDeck { name: "b".into(), cards: vec!["BT16-082".into()], eggs: vec!["BT14-001".into()] },
                PooledDeck { name: "c".into(), cards: vec!["BT17-102".into()], eggs: vec!["BT14-001".into()] },
            ],
        }
    }

    #[test]
    fn build_jobs_emits_requested_count_with_unique_ids() {
        let jobs = build_jobs(&pool(), 5, 100, &JobLimits { max_turns: 40, timeout_seconds: 180 })
            .expect("build");
        assert_eq!(jobs.len(), 5);
        let mut ids: Vec<&str> = jobs.iter().map(|j| j.job_id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 5, "job ids must be unique");
    }

    #[test]
    fn build_jobs_is_deterministic_for_a_given_base_seed() {
        let limits = JobLimits { max_turns: 40, timeout_seconds: 180 };
        let a = build_jobs(&pool(), 6, 42, &limits).expect("build");
        let b = build_jobs(&pool(), 6, 42, &limits).expect("build");
        let seeds_a: Vec<u64> = a.iter().map(|j| j.seed).collect();
        let seeds_b: Vec<u64> = b.iter().map(|j| j.seed).collect();
        assert_eq!(seeds_a, seeds_b);
        assert_eq!(a[3].decks.p0, b[3].decks.p0);
    }

    #[test]
    fn build_jobs_never_mirrors_a_deck_against_itself() {
        let jobs = build_jobs(&pool(), 12, 1, &JobLimits { max_turns: 40, timeout_seconds: 180 })
            .expect("build");
        for j in &jobs {
            assert_ne!(j.decks.p0, j.decks.p1, "mirror matches waste corpus slots");
        }
    }

    #[test]
    fn build_jobs_alternates_first_player() {
        let jobs = build_jobs(&pool(), 4, 1, &JobLimits { max_turns: 40, timeout_seconds: 180 })
            .expect("build");
        let firsts: Vec<u8> = jobs.iter().map(|j| j.first_player).collect();
        assert!(firsts.contains(&0) && firsts.contains(&1), "both seats must go first");
    }

    #[test]
    fn build_jobs_rejects_a_pool_too_small_to_pair() {
        let small = DeckPool { decks: vec![PooledDeck { name: "solo".into(), cards: vec![], eggs: vec![] }] };
        let err = build_jobs(&small, 1, 1, &JobLimits { max_turns: 40, timeout_seconds: 180 })
            .expect_err("one deck cannot form a pair");
        assert!(err.contains("at least 2"), "error should say what is wrong: {}", err);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
CARGO_TARGET_DIR='D:\cargo-target-wt\quizzical-ishizaka-07b190' cargo test -p dcgo-harness --lib
```

Expected: FAIL — `cannot find type DeckPool in this scope` (the `pool` module is not declared yet, so also expect an unresolved-module error once added).

- [ ] **Step 3: Write the pool implementation**

Prepend to `code/tools/dcgo-harness/src/pool.rs`:

```rust
//! Deck-pool sampling. The CLI owns deck *selection* (which matchups the
//! corpus exercises); DCGO owns deck *encoding*.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::job::{JobDecks, JobLimits, JobSpec};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PooledDeck {
    pub name: String,
    /// Main deck, 50 card IDs.
    pub cards: Vec<String>,
    /// Digitama deck, up to 5 card IDs.
    #[serde(default)]
    pub eggs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckPool {
    pub decks: Vec<PooledDeck>,
}

pub fn load_pool(path: &Path) -> Result<DeckPool, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("reading deck pool {}: {}", path.display(), e))?;
    serde_json::from_str(&text).map_err(|e| format!("parsing deck pool: {}", e))
}

/// Build `count` jobs by walking deck pairs deterministically.
///
/// Determinism matters twice over: the per-job `seed` makes the *game*
/// reproducible, and a deterministic pairing walk makes the *batch*
/// reproducible, so "rerun batch 42" produces the same matchups.
///
/// Pairing walks p0 through the pool in order and offsets p1 by a stride that
/// is coprime-ish with the pool size, which spreads matchups without ever
/// pairing a deck against itself.
pub fn build_jobs(
    pool: &DeckPool,
    count: u32,
    base_seed: u64,
    limits: &JobLimits,
) -> Result<Vec<JobSpec>, String> {
    let n = pool.decks.len();
    if n < 2 {
        return Err(format!(
            "deck pool needs at least 2 decks to form a matchup, found {}",
            n
        ));
    }

    let mut jobs = Vec::with_capacity(count as usize);
    for i in 0..count as usize {
        let a = i % n;
        // Offset by at least 1 so p0 != p1, and vary it so matchups spread.
        let b = (a + 1 + (i / n) % (n - 1)) % n;
        let p0 = &pool.decks[a];
        let p1 = &pool.decks[b];

        // Eggs ride the same list; DCGO's card-kind routing separates them.
        let mut p0_cards = p0.cards.clone();
        p0_cards.extend(p0.eggs.iter().cloned());
        let mut p1_cards = p1.cards.clone();
        p1_cards.extend(p1.eggs.iter().cloned());

        jobs.push(JobSpec {
            job_id: format!("vol-{:05}", i),
            policy: "ai".to_string(),
            decks: JobDecks {
                p0: p0_cards,
                p1: p1_cards,
            },
            // Alternate the opening seat so first-player advantage does not
            // bias which lines the corpus covers.
            first_player: (i % 2) as u8,
            seed: base_seed.wrapping_add(i as u64),
            limits: limits.clone(),
        });
    }
    Ok(jobs)
}
```

Add `Clone` to `JobLimits` in `job.rs` if it is missing (the derive list in Task 1 already includes it).

Update `code/tools/dcgo-harness/src/lib.rs`:

```rust
//! Host side of the DCGO automation harness: job submission, queue status,
//! and corpus triage. The DCGO client itself only reads and writes files in
//! the job directories — see
//! `docs/superpowers/specs/2026-08-17-dcgo-automation-harness-design.md`.

pub mod job;
pub mod pool;
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
CARGO_TARGET_DIR='D:\cargo-target-wt\quizzical-ishizaka-07b190' cargo test -p dcgo-harness --lib
```

Expected: PASS — `test result: ok. 8 passed`.

- [ ] **Step 5: Write the CLI with the `submit` subcommand**

Create `code/tools/dcgo-harness/src/main.rs`:

```rust
//! `dcgo-harness` — submit DCGO harness jobs, report queue status, triage the
//! resulting corpus.
//!
//! Exit codes:
//!   0 — command succeeded.
//!   1 — command ran but reported failures (e.g. triage found divergences).
//!   2 — argument or I/O error.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use dcgo_harness::job::{JobLimits, DIR_CLAIMED, DIR_DONE, DIR_FAILED, DIR_JOBS};
use dcgo_harness::pool;

#[derive(Parser, Debug)]
#[command(about = "Drive unattended DCGO games from a filesystem job queue.")]
struct Args {
    /// Harness root: the directory holding jobs/ claimed/ done/ failed/.
    #[arg(long)]
    root: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Write N job files into jobs/.
    Submit {
        /// How many games to queue.
        #[arg(long)]
        count: u32,
        /// Deck pool JSON: {"decks":[{"name":..,"cards":[..],"eggs":[..]}]}.
        #[arg(long)]
        decks: PathBuf,
        /// Base seed; job i gets base_seed + i.
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Abandon a game past this many turns.
        #[arg(long, default_value_t = 40)]
        max_turns: u32,
        /// Wall-clock budget per job.
        #[arg(long, default_value_t = 180)]
        timeout_seconds: u64,
    },
}

fn main() -> ExitCode {
    let args = Args::parse();
    match run(&args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::from(2)
        }
    }
}

fn run(args: &Args) -> Result<ExitCode, String> {
    for dir in [DIR_JOBS, DIR_CLAIMED, DIR_DONE, DIR_FAILED] {
        let path = args.root.join(dir);
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("creating {}: {}", path.display(), e))?;
    }

    match &args.command {
        Command::Submit {
            count,
            decks,
            seed,
            max_turns,
            timeout_seconds,
        } => {
            let deck_pool = pool::load_pool(decks)?;
            let limits = JobLimits {
                max_turns: *max_turns,
                timeout_seconds: *timeout_seconds,
            };
            let jobs = pool::build_jobs(&deck_pool, *count, *seed, &limits)?;
            let jobs_dir = args.root.join(DIR_JOBS);
            for spec in &jobs {
                let path = jobs_dir.join(format!("{}.json", spec.job_id));
                std::fs::write(&path, spec.to_json()?)
                    .map_err(|e| format!("writing {}: {}", path.display(), e))?;
            }
            println!(
                "submitted {} job(s) to {}",
                jobs.len(),
                jobs_dir.display()
            );
            Ok(ExitCode::SUCCESS)
        }
    }
}
```

- [ ] **Step 6: Verify the CLI end to end**

```bash
CARGO_TARGET_DIR='D:\cargo-target-wt\quizzical-ishizaka-07b190' cargo run -p dcgo-harness -- --root /tmp/hx submit --count 3 --decks code/tools/dcgo-harness/testdata/pool.json
```

First create `code/tools/dcgo-harness/testdata/pool.json`:

```json
{
  "decks": [
    {"name": "ex12-blue", "cards": ["EX12-035", "EX12-035"], "eggs": ["EX12-001"]},
    {"name": "bt16-ukkomon", "cards": ["BT16-082", "BT16-082"], "eggs": ["BT14-001"]}
  ]
}
```

Expected: `submitted 3 job(s) to /tmp/hx/jobs`, and `ls /tmp/hx/jobs` shows `vol-00000.json vol-00001.json vol-00002.json`.

- [ ] **Step 7: Commit**

```bash
git add code/tools/dcgo-harness
git commit -m "dcgo-harness: submit command with deterministic deck-pair sampling"
```

---

### Task 3: `status` — queue state, timeouts, quarantine (Rust)

**Files:**
- Create: `code/tools/dcgo-harness/src/queue.rs`
- Modify: `code/tools/dcgo-harness/src/lib.rs`
- Modify: `code/tools/dcgo-harness/src/main.rs`

**Interfaces:**
- Consumes: `job::{JobResult, JobOutcome, DIR_JOBS, DIR_CLAIMED, DIR_DONE, DIR_FAILED}` from Task 1.
- Produces: `queue::{QueueStatus, sweep_timeouts}`; `queue::QueueStatus { pending: usize, claimed: usize, completed: usize, partial: usize, failed: usize }`; `queue::scan(root: &Path) -> Result<QueueStatus, String>`; `queue::classify_claimed(age_seconds: u64, timeout_seconds: u64, prior_failures: u32) -> ClaimVerdict`; `queue::ClaimVerdict::{Running, Requeue, Quarantine}`.

- [ ] **Step 1: Write the failing test**

Create `code/tools/dcgo-harness/src/queue.rs` with only this test module:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

```bash
CARGO_TARGET_DIR='D:\cargo-target-wt\quizzical-ishizaka-07b190' cargo test -p dcgo-harness --lib queue
```

Expected: FAIL — `cannot find function classify_claimed in this scope`.

- [ ] **Step 3: Write the queue implementation**

Prepend to `code/tools/dcgo-harness/src/queue.rs`:

```rust
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
```

Add `pub mod queue;` to `code/tools/dcgo-harness/src/lib.rs`.

- [ ] **Step 4: Run tests to verify they pass**

```bash
CARGO_TARGET_DIR='D:\cargo-target-wt\quizzical-ishizaka-07b190' cargo test -p dcgo-harness --lib
```

Expected: PASS — `test result: ok. 13 passed`.

- [ ] **Step 5: Add the `status` subcommand**

In `code/tools/dcgo-harness/src/main.rs`, add to the `Command` enum:

```rust
    /// Report queue counts; sweep overdue claims.
    Status {
        /// Also requeue/quarantine claims older than their budget.
        #[arg(long, default_value_t = false)]
        sweep: bool,
        /// Timeout used when sweeping.
        #[arg(long, default_value_t = 180)]
        timeout_seconds: u64,
    },
```

Add to the `match &args.command` block:

```rust
        Command::Status {
            sweep,
            timeout_seconds,
        } => {
            if *sweep {
                let (requeued, quarantined) =
                    dcgo_harness::queue::sweep_timeouts(&args.root, *timeout_seconds)?;
                if requeued > 0 || quarantined > 0 {
                    println!("swept: requeued={} quarantined={}", requeued, quarantined);
                }
            }
            let status = dcgo_harness::queue::scan(&args.root)?;
            println!("{}", status.summary());
            Ok(ExitCode::SUCCESS)
        }
```

- [ ] **Step 6: Verify the command runs**

```bash
CARGO_TARGET_DIR='D:\cargo-target-wt\quizzical-ishizaka-07b190' cargo run -p dcgo-harness -- --root /tmp/hx status
```

Expected: `pending=3 claimed=0 completed=0 partial=0 failed=0`.

- [ ] **Step 7: Commit**

```bash
git add code/tools/dcgo-harness
git commit -m "dcgo-harness: status command with timeout sweep and quarantine"
```

---

### Task 4: `triage` — cluster corpus divergences by signature (Rust)

**Files:**
- Create: `code/tools/dcgo-harness/src/triage.rs`
- Modify: `code/tools/dcgo-harness/src/lib.rs`
- Modify: `code/tools/dcgo-harness/src/main.rs`

**Interfaces:**
- Consumes: `queue::{QueueStatus, scan}` from Task 3.
- Produces: `triage::{Signature, Cluster, TriageReport}`; `triage::signature_of(kind: &str, action_id: u16, card_at_slot: Option<&str>) -> Signature`; `triage::cluster(findings: &[Finding]) -> Vec<Cluster>`; `triage::Finding { game_id: String, step: u32, kind: String, action_id: u16, card_at_slot: Option<String>, recording_path: String }`.

- [ ] **Step 1: Write the failing test**

Create `code/tools/dcgo-harness/src/triage.rs` with only this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn finding(game: &str, kind: &str, action_id: u16, card: Option<&str>) -> Finding {
        Finding {
            game_id: game.to_string(),
            step: 9,
            kind: kind.to_string(),
            action_id,
            card_at_slot: card.map(|c| c.to_string()),
            recording_path: format!("recordings/{}.jsonl", game),
        }
    }

    #[test]
    fn one_bug_across_many_games_collapses_to_one_cluster() {
        let findings: Vec<Finding> = (0..50)
            .map(|i| finding(&format!("g{}", i), "illegal_action", 1040, Some("EX10-010")))
            .collect();
        let clusters = cluster(&findings);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].count, 50);
    }

    #[test]
    fn two_different_bugs_on_one_card_stay_apart() {
        // Same card, different action ranges — genuinely different defects.
        let findings = vec![
            finding("g1", "illegal_action", 1040, Some("EX10-010")),
            finding("g2", "illegal_action", 114, Some("EX10-010")),
        ];
        let clusters = cluster(&findings);
        assert_eq!(clusters.len(), 2, "field-effect and attack bugs are not one bug");
    }

    #[test]
    fn clusters_are_ranked_most_frequent_first() {
        let mut findings = vec![finding("g1", "illegal_action", 114, Some("A"))];
        for i in 0..5 {
            findings.push(finding(&format!("h{}", i), "illegal_action", 1040, Some("B")));
        }
        let clusters = cluster(&findings);
        assert_eq!(clusters[0].count, 5);
        assert_eq!(clusters[1].count, 1);
    }

    #[test]
    fn each_cluster_names_a_concrete_recording_to_reproduce_from() {
        let findings = vec![finding("g7", "illegal_action", 1040, Some("EX10-010"))];
        let clusters = cluster(&findings);
        assert_eq!(clusters[0].example_recording, "recordings/g7.jsonl");
        assert_eq!(clusters[0].example_step, 9);
    }

    #[test]
    fn report_refuses_a_clean_verdict_without_completed_games() {
        let report = TriageReport {
            status: crate::queue::QueueStatus { pending: 0, claimed: 0, completed: 0, partial: 0, failed: 200 },
            clusters: Vec::new(),
        };
        let text = report.render();
        assert!(text.contains("failed=200"), "denominator must appear: {}", text);
        assert!(
            !text.to_lowercase().contains("no divergences found"),
            "a batch with zero completed games must not read as a pass: {}",
            text
        );
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
CARGO_TARGET_DIR='D:\cargo-target-wt\quizzical-ishizaka-07b190' cargo test -p dcgo-harness --lib triage
```

Expected: FAIL — `cannot find type Finding in this scope`.

- [ ] **Step 3: Write the triage implementation**

Prepend to `code/tools/dcgo-harness/src/triage.rs`:

```rust
//! Corpus triage: collapse many recordings' divergences into a ranked list of
//! distinct defects, each with a concrete repro.

use std::collections::HashMap;

use crate::queue::QueueStatus;

/// One divergence from one recording.
#[derive(Debug, Clone)]
pub struct Finding {
    pub game_id: String,
    pub step: u32,
    pub kind: String,
    pub action_id: u16,
    /// Card occupying the board slot the action referenced, when the failure
    /// is board-addressed. This is what makes two bugs on different cards
    /// distinguishable.
    pub card_at_slot: Option<String>,
    pub recording_path: String,
}

/// What makes two findings "the same bug":
/// (failure kind, action-space range, card at the referenced slot).
///
/// Coarse enough that fifty recordings hitting one card's bug collapse into a
/// single ranked entry; specific enough that a field-effect bug and an attack
/// bug on the same card stay apart.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Signature {
    pub kind: String,
    pub range: &'static str,
    pub card: String,
}

fn action_range(action_id: u16) -> &'static str {
    match action_id {
        0..=29 => "play_hand",
        30..=59 => "hand_effect_or_selection",
        60 => "hatch",
        61 => "move_from_breeding",
        62 => "pass",
        63..=92 => "dna_digivolve",
        93 => "concede",
        100..=399 => "attack",
        400..=999 => "digivolve",
        1000..=1149 => "field_effect",
        1150..=1194 => "trash_effect",
        2000..=2191 => "source_select",
        _ => "other",
    }
}

pub fn signature_of(kind: &str, action_id: u16, card_at_slot: Option<&str>) -> Signature {
    Signature {
        kind: kind.to_string(),
        range: action_range(action_id),
        card: card_at_slot.unwrap_or("-").to_string(),
    }
}

#[derive(Debug, Clone)]
pub struct Cluster {
    pub signature: Signature,
    pub count: usize,
    pub example_recording: String,
    pub example_step: u32,
}

/// Group findings by signature and rank most-frequent first.
pub fn cluster(findings: &[Finding]) -> Vec<Cluster> {
    let mut by_sig: HashMap<Signature, Cluster> = HashMap::new();
    for f in findings {
        let sig = signature_of(&f.kind, f.action_id, f.card_at_slot.as_deref());
        by_sig
            .entry(sig.clone())
            .and_modify(|c| c.count += 1)
            .or_insert(Cluster {
                signature: sig,
                count: 1,
                example_recording: f.recording_path.clone(),
                example_step: f.step,
            });
    }
    let mut out: Vec<Cluster> = by_sig.into_values().collect();
    // Ties broken by signature so output is stable run to run.
    out.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.signature.card.cmp(&b.signature.card))
            .then_with(|| a.signature.range.cmp(b.signature.range))
    });
    out
}

pub struct TriageReport {
    pub status: QueueStatus,
    pub clusters: Vec<Cluster>,
}

impl TriageReport {
    /// Render the report. The denominator line is unconditional: a batch where
    /// 180 of 200 jobs died on deck-import errors must never read as a pass.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("corpus: {}\n", self.status.summary()));

        if self.status.completed == 0 {
            out.push_str(
                "VERDICT: inconclusive — no games completed, so nothing was actually checked.\n",
            );
            return out;
        }

        if self.clusters.is_empty() {
            out.push_str(&format!(
                "VERDICT: no divergences across {} completed game(s).\n",
                self.status.completed
            ));
            return out;
        }

        out.push_str(&format!(
            "VERDICT: {} distinct divergence(s) across {} completed game(s).\n\n",
            self.clusters.len(),
            self.status.completed
        ));
        for (i, c) in self.clusters.iter().enumerate() {
            out.push_str(&format!(
                "{}. [{}x] {} in {} on {}\n   repro: dcgo-replay --input {} --cards-json data/cards.json --verbose   (step {})\n",
                i + 1,
                c.count,
                c.signature.kind,
                c.signature.range,
                c.signature.card,
                c.example_recording,
                c.example_step
            ));
        }
        out
    }
}
```

Add `pub mod triage;` to `code/tools/dcgo-harness/src/lib.rs`.

- [ ] **Step 4: Run tests to verify they pass**

```bash
CARGO_TARGET_DIR='D:\cargo-target-wt\quizzical-ishizaka-07b190' cargo test -p dcgo-harness --lib
```

Expected: PASS — `test result: ok. 18 passed`.

- [ ] **Step 5: Wire `triage` to the real corpus**

In `main.rs`, add the subcommand:

```rust
    /// Replay every recording in the corpus and rank distinct divergences.
    Triage {
        /// Directory of .jsonl recordings.
        #[arg(long)]
        corpus: PathBuf,
        /// Path to data/cards.json.
        #[arg(long)]
        cards_json: PathBuf,
    },
```

And the handler, which reuses the `dcgo-replay` library rather than duplicating replay logic:

```rust
        Command::Triage { corpus, cards_json } => {
            use dcgo_harness::triage::{cluster, Finding, TriageReport};

            let card_data = dcgo_replay::load_card_data_at(cards_json)
                .map_err(|e| format!("loading cards.json: {}", e))?;

            let mut findings: Vec<Finding> = Vec::new();
            let entries = std::fs::read_dir(corpus)
                .map_err(|e| format!("reading corpus {}: {}", corpus.display(), e))?;
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().map(|x| x != "jsonl").unwrap_or(true) {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&path) else { continue };
                let text = text.trim_start_matches('\u{feff}').to_string();
                let Ok(recording) = dcgo_replay::parse_jsonl(&text) else { continue };
                let outcome = dcgo_replay::replay_recording(
                    &recording,
                    &card_data,
                    &dcgo_replay::ReplayConfig::default(),
                );
                if let dcgo_replay::ReplayOutcome::Fail(dcgo_replay::ReplayFail::IllegalAction(ia)) =
                    &outcome
                {
                    findings.push(Finding {
                        game_id: recording.start.game_id.clone(),
                        step: ia.step,
                        kind: "illegal_action".to_string(),
                        action_id: ia.action_id,
                        card_at_slot: None,
                        recording_path: path.display().to_string(),
                    });
                }
            }

            let status = dcgo_harness::queue::scan(&args.root)?;
            let report = TriageReport {
                status,
                clusters: cluster(&findings),
            };
            print!("{}", report.render());
            Ok(if findings.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
```

If `dcgo_replay::load_card_data_at` does not exist, add it to `code/tools/dcgo-replay/src/lib.rs` by extracting the loader currently inlined in that crate's `main.rs`, and re-export `parse_jsonl`, `replay_recording`, `ReplayConfig`, `ReplayOutcome`, `ReplayFail` from its `lib.rs`.

- [ ] **Step 6: Verify against the existing recordings**

```bash
CARGO_TARGET_DIR='D:\cargo-target-wt\quizzical-ishizaka-07b190' cargo run -p dcgo-harness -- --root /tmp/hx triage --corpus "/c/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_recordings" --cards-json data/cards.json
```

Expected: a `corpus: pending=... completed=0 ...` line followed by `VERDICT: inconclusive — no games completed`. That is correct: those recordings predate the harness, so the queue has no results. It proves the denominator guard fires.

- [ ] **Step 7: Commit**

```bash
git add code/tools/dcgo-harness code/tools/dcgo-replay
git commit -m "dcgo-harness: triage command clustering corpus divergences by signature"
```

---

### Task 5: DCGO harness config and job model (C#)

**Files:**
- Create: `$BASE_DCGO/Assets/Scripts/Script/Harness/HarnessConfig.cs`
- Create: `$BASE_DCGO/Assets/Scripts/Script/Harness/HarnessJob.cs`

Where `BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"`.

**Interfaces:**
- Consumes: the JSON written by Task 2's `submit`.
- Produces: `Digimon.Harness.HarnessConfig` (static: `Enabled`, `Root`, `TimeScale`, `PollSeconds`); `Digimon.Harness.HarnessJob` with fields `job_id`, `policy`, `decks` (`HarnessJobDecks` with `p0`/`p1` string arrays), `first_player`, `seed`, `limits` (`HarnessJobLimits` with `max_turns`, `timeout_seconds`); `HarnessJob.Parse(string json)`.

- [ ] **Step 1: Write `HarnessConfig.cs`**

```csharp
using System.IO;
using UnityEngine;

namespace Digimon.Harness
{
    /// <summary>
    /// Configuration for the unattended job harness. Mirrors
    /// <see cref="Digimon.Recording.RecorderConfig"/>'s shape so both mods are
    /// configured the same way.
    /// </summary>
    public static class HarnessConfig
    {
        /// <summary>
        /// Master switch. When false the JobWatcher never bootstraps and DCGO
        /// behaves exactly as upstream. Default OFF so a normal play session is
        /// never hijacked by a stale job file.
        /// </summary>
        public static bool Enabled { get; set; } = false;

        /// <summary>
        /// Harness root holding jobs/ claimed/ done/ failed/. Defaults beside
        /// the recorder's output so both live under persistentDataPath.
        /// </summary>
        public static string Root { get; set; } =
            Path.Combine(Application.persistentDataPath, "dcgo_harness");

        /// <summary>
        /// Time multiplier while a job runs. A corpus of hundreds of games is
        /// worthless if each spends 40s in animation. Raised, not unbounded:
        /// very high scales can starve coroutines that yield per-frame.
        /// </summary>
        public static float TimeScale { get; set; } = 8f;

        /// <summary>How often the watcher looks for new jobs.</summary>
        public static float PollSeconds { get; set; } = 1f;

        public static string JobsDir => Path.Combine(Root, "jobs");
        public static string ClaimedDir => Path.Combine(Root, "claimed");
        public static string DoneDir => Path.Combine(Root, "done");
        public static string FailedDir => Path.Combine(Root, "failed");
    }
}
```

- [ ] **Step 2: Write `HarnessJob.cs`**

```csharp
using System;
using UnityEngine;

namespace Digimon.Harness
{
    /// <summary>
    /// One unattended game, as written by the `dcgo-harness submit` CLI.
    /// </summary>
    /// <remarks>
    /// Parsed with Unity's <see cref="JsonUtility"/>, which handles nested
    /// [Serializable] classes and arrays and silently ignores unknown fields —
    /// exactly the forward-compatibility we want, since phase-2 jobs will carry
    /// `deck_order` and `inputs` that a phase-1 client must tolerate.
    ///
    /// Field names are snake_case to match the JSON verbatim; JsonUtility has no
    /// name-mapping attribute, so the C# fields wear the wire names.
    /// </remarks>
    [Serializable]
    public class HarnessJob
    {
        public string job_id;
        public string policy;
        public HarnessJobDecks decks;
        public int first_player;
        public long seed;
        public HarnessJobLimits limits;

        /// <summary>Parse a job file. Returns null when the text is unusable.</summary>
        public static HarnessJob Parse(string json)
        {
            if (string.IsNullOrEmpty(json)) return null;
            try
            {
                HarnessJob job = JsonUtility.FromJson<HarnessJob>(json);
                if (job == null || string.IsNullOrEmpty(job.job_id)) return null;
                if (job.decks == null || job.decks.p0 == null || job.decks.p1 == null) return null;
                if (job.limits == null) job.limits = new HarnessJobLimits();
                return job;
            }
            catch (Exception e)
            {
                Debug.LogError("[Harness] job parse failed: " + e.Message);
                return null;
            }
        }
    }

    [Serializable]
    public class HarnessJobDecks
    {
        public string[] p0;
        public string[] p1;
    }

    [Serializable]
    public class HarnessJobLimits
    {
        public int max_turns = 40;
        public int timeout_seconds = 180;
    }
}
```

- [ ] **Step 3: Verify it compiles in Unity**

Open the Unity project (base repo `DCGO/`). Wait for the compile to finish and check the Console.

Expected: no compile errors. If the Console shows errors mentioning `Digimon.Harness`, fix them before proceeding — every later C# task builds on this.

- [ ] **Step 4: Commit (in the base repo)**

```bash
cd "$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"
git add Assets/Scripts/Script/Harness
git commit -m "Harness: config and job model"
```

---

### Task 6: JobWatcher claims a job and applies it (C#)

**Files:**
- Create: `$BASE_DCGO/Assets/Scripts/Script/Harness/JobWatcher.cs`
- Modify: `$BASE_DCGO/Assets/Scripts/Script/CardObjectController.cs` (add a deck override; the `RandomDeck` field is declared at line 21)

**Interfaces:**
- Consumes: `HarnessConfig`, `HarnessJob` from Task 5.
- Produces: `Digimon.Harness.JobWatcher.Instance`; `JobWatcher.CurrentJob` (`HarnessJob`, null when idle); `CardObjectController.HarnessDeckOverrideP0` / `HarnessDeckOverrideP1` (`static DeckData`, null when unset).

- [ ] **Step 1: Add the deck override to `CardObjectController`**

`CardObjectController.DeckRecipie` currently picks P0's deck from `ContinuousController.instance.BattleDeckData` and the AI's from the private `RandomDeck`. Add static overrides just below the `RandomDeck` declaration (line 21):

```csharp
    /// <summary>
    /// [Harness mod] When set, these replace the normal deck sources so an
    /// unattended job fully specifies both seats. Null in normal play.
    /// </summary>
    public static DeckData HarnessDeckOverrideP0 = null;
    public static DeckData HarnessDeckOverrideP1 = null;
```

Then, at the top of both deck-returning methods (`DeckRecipie`, and the digitama equivalent that begins around line 215), insert the override check. For `DeckRecipie`:

```csharp
        // [Harness mod] A job specifies both decks outright; short-circuit the
        // BattleDeckData / RandomDeck selection entirely.
        if (HarnessDeckOverrideP0 != null && HarnessDeckOverrideP1 != null)
        {
            DeckData chosen = (player == MasterPlayer) ? HarnessDeckOverrideP0 : HarnessDeckOverrideP1;
            return RandomUtility.ShuffledDeckCards(chosen.DeckCards());
        }
```

And the same at the top of the digitama method, with `chosen.DigitamaDeckCards()` instead of `chosen.DeckCards()`.

- [ ] **Step 2: Write `JobWatcher.cs`**

```csharp
using System;
using System.Collections;
using System.IO;
using System.Linq;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Digimon.Harness
{
    /// <summary>
    /// Polls the harness jobs directory, claims one job at a time, applies it,
    /// and starts a game. Replaces the blind BattleScene reload that
    /// <c>TurnStateMachine</c> performs at game end under plain auto mode with a
    /// job-driven one.
    /// </summary>
    public class JobWatcher : MonoBehaviour
    {
        public static JobWatcher Instance { get; private set; }

        /// <summary>The job currently being played, or null when idle.</summary>
        public HarnessJob CurrentJob { get; private set; }

        /// <summary>Path of the claimed job file, used when filing the result.</summary>
        public string ClaimedPath { get; private set; }

        public DateTime StartedUtc { get; private set; }

        [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.BeforeSceneLoad)]
        private static void Bootstrap()
        {
            if (!HarnessConfig.Enabled) return;
            if (Instance != null) return;

            var go = new GameObject("JobWatcher");
            DontDestroyOnLoad(go);
            Instance = go.AddComponent<JobWatcher>();
        }

        private void Start()
        {
            Directory.CreateDirectory(HarnessConfig.JobsDir);
            Directory.CreateDirectory(HarnessConfig.ClaimedDir);
            Directory.CreateDirectory(HarnessConfig.DoneDir);
            Directory.CreateDirectory(HarnessConfig.FailedDir);
            StartCoroutine(PollLoop());
        }

        private IEnumerator PollLoop()
        {
            while (true)
            {
                if (CurrentJob == null)
                {
                    TryClaimAndStart();
                }
                yield return new WaitForSecondsRealtime(HarnessConfig.PollSeconds);
            }
        }

        private void TryClaimAndStart()
        {
            string[] files;
            try
            {
                files = Directory.GetFiles(HarnessConfig.JobsDir, "*.json");
            }
            catch (Exception e)
            {
                Debug.LogError("[Harness] listing jobs failed: " + e.Message);
                return;
            }
            if (files.Length == 0) return;

            Array.Sort(files, StringComparer.Ordinal);
            string source = files[0];
            string claimed = Path.Combine(HarnessConfig.ClaimedDir, Path.GetFileName(source));

            // Atomic rename IS the claim. If it throws, another process (or a
            // stale handle) got there first; just try again next poll.
            try
            {
                File.Move(source, claimed);
            }
            catch (Exception)
            {
                return;
            }

            HarnessJob job = HarnessJob.Parse(SafeRead(claimed));
            if (job == null)
            {
                Fail(claimed, "unparseable job file");
                return;
            }

            if (!ApplyJob(job))
            {
                Fail(claimed, "could not apply job (deck resolution failed)");
                return;
            }

            CurrentJob = job;
            ClaimedPath = claimed;
            StartedUtc = DateTime.UtcNow;
            Debug.Log("[Harness] started job " + job.job_id);

            // Same handoff the auto-mode restart uses.
            ContinuousController.instance.isAI = true;
            SceneManager.LoadScene("BattleScene");
        }

        /// <summary>
        /// Configure the game from the job: decks, seed, auto mode, time scale.
        /// </summary>
        private bool ApplyJob(HarnessJob job)
        {
            DeckData p0 = DeckBuilder.FromCardIds("harness-p0", job.decks.p0);
            DeckData p1 = DeckBuilder.FromCardIds("harness-p1", job.decks.p1);
            if (p0 == null || p1 == null) return false;

            CardObjectController.HarnessDeckOverrideP0 = p0;
            CardObjectController.HarnessDeckOverrideP1 = p1;
            ContinuousController.instance.BattleDeckData = p0;

            // Auto mode is what actually plays the game: it drives the local
            // seat's mulligan, breeding, and main phase. Without this the job
            // would load a board and then sit waiting for a human.
            ContinuousController.instance.isAI = true;
            if (GManager.instance != null)
            {
                GManager.instance.isAuto = true;
            }

            // Determinism: every random draw in this game derives from the
            // job's seed, so a divergence found in game 137 can be re-run.
            UnityEngine.Random.InitState(unchecked((int)job.seed));

            Time.timeScale = HarnessConfig.TimeScale;
            return true;
        }

        private void Fail(string claimedPath, string message)
        {
            Debug.LogError("[Harness] " + message + " (" + claimedPath + ")");
            try
            {
                string dest = Path.Combine(HarnessConfig.FailedDir, Path.GetFileName(claimedPath));
                if (File.Exists(dest)) File.Delete(dest);
                File.Move(claimedPath, dest);
            }
            catch (Exception e)
            {
                Debug.LogError("[Harness] could not file failure: " + e.Message);
            }
            CurrentJob = null;
            ClaimedPath = null;
        }

        private static string SafeRead(string path)
        {
            try { return File.ReadAllText(path); }
            catch (Exception) { return null; }
        }
    }
}
```

- [ ] **Step 3: Write the card-ID → `DeckData` builder**

Create `$BASE_DCGO/Assets/Scripts/Script/Harness/DeckBuilder.cs`:

```csharp
using System.Collections.Generic;
using System.Linq;
using UnityEngine;

namespace Digimon.Harness
{
    /// <summary>
    /// Builds a <see cref="DeckData"/> from card ID strings.
    /// </summary>
    /// <remarks>
    /// Jobs carry card IDs ("EX12-035"), never DCGO deck codes. The deck code is
    /// a base-n encoding over DCGO's internal <c>CEntity_Base.CardIndex</c>, so
    /// reimplementing it host-side would duplicate a table that rots whenever
    /// DCGO re-indexes. Resolving here keeps the encoding owned by the codebase
    /// that defines it.
    ///
    /// Digitama cards are separated by kind rather than by a second list, so the
    /// job can ship one flat card-ID array per seat.
    /// </remarks>
    public static class DeckBuilder
    {
        public static DeckData FromCardIds(string deckName, string[] cardIds)
        {
            if (cardIds == null || cardIds.Length == 0)
            {
                Debug.LogError("[Harness] deck '" + deckName + "' has no cards");
                return null;
            }

            var main = new List<CEntity_Base>();
            var digitama = new List<CEntity_Base>();

            foreach (string id in cardIds)
            {
                CEntity_Base entity = ContinuousController.instance.SortedCardList
                    .FirstOrDefault(e => e.CardID == id);
                if (entity == null)
                {
                    Debug.LogError("[Harness] unknown card id '" + id + "' in deck '" + deckName + "'");
                    return null;
                }

                if (entity.IsDigitama())
                {
                    digitama.Add(entity);
                }
                else
                {
                    main.Add(entity);
                }
            }

            string code = DeckData.GetDeckCode(deckName, main, digitama, null);
            if (!DeckData.IsValidDeckCode(code))
            {
                Debug.LogError("[Harness] deck '" + deckName + "' produced an invalid deck code");
                return null;
            }
            return new DeckData(code);
        }
    }
}
```

If `CEntity_Base` has no `IsDigitama()` helper, replace that call with DCGO's own digitama test — find it by grepping for how `DigitamaDeckCards` distinguishes them:

```bash
cd "$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"
grep -rn "Digitama" Assets/Scripts/Script/CEntity_Base.cs | head
```

Use whatever predicate that reveals (likely a `CardKind` comparison).

- [ ] **Step 4: Verify a job is claimed**

1. In Unity, set `HarnessConfig.Enabled = true` (temporarily, by editing the default in `HarnessConfig.cs`).
2. Submit jobs pointing at the same root Unity uses:

```bash
CARGO_TARGET_DIR='D:\cargo-target-wt\quizzical-ishizaka-07b190' cargo run -p dcgo-harness -- --root "/c/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness" submit --count 1 --decks code/tools/dcgo-harness/testdata/pool.json
```

3. Press Play in Unity.

Expected: the Console logs `[Harness] started job vol-00000`, the file moves from `jobs/` to `claimed/`, and a battle starts. If it logs `unknown card id`, the `testdata/pool.json` cards are not in DCGO's registry — replace them with IDs from a real deck.

- [ ] **Step 5: Commit (in the base repo)**

```bash
cd "$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"
git add Assets/Scripts/Script/Harness Assets/Scripts/Script/CardObjectController.cs
git commit -m "Harness: JobWatcher claims a job, resolves decks, seeds RNG"
```

---

### Task 7: Auto mode covers breeding for both seats (C#)

**Files:**
- Modify: `$BASE_DCGO/Assets/Scripts/Script/TurnStateMachine.cs:875`

**Interfaces:**
- Consumes: nothing new.
- Produces: no new API; changes recorded behavior so `action_id` 60 (`HATCH`) and 61 (`MOVE_FROM_BREEDING`) appear for the local seat.

- [ ] **Step 1: Understand what is there now**

At `TurnStateMachine.cs:875` the local seat under auto mode skips breeding entirely:

```csharp
                if (gameContext.TurnPlayer.isYou && GManager.instance.isAuto && GManager.instance.IsAI)
                {
                    gameContext.TurnPhase = GameContext.phase.Main;
                }
```

The opponent seat, in the `else` branch just below, runs a real AI breeding decision:

```csharp
                if (GManager.instance.IsAI)
                {
                    bool doHatch = RandomUtility.IsSucceedProbability(0.85f);

                    if (gameContext.TurnPlayer.CanHatch)
                    {
                        doHatch = true;
                    }

                    SetBreedingPhase(gameContext.TurnPlayer.PlayerID, doHatch);
                }
```

Left as-is, every generated game has P0 never hatching and never moving from breeding — a whole mechanic silently missing from the corpus while everything appears to work.

- [ ] **Step 2: Replace the skip with the same AI decision**

```csharp
                if (gameContext.TurnPlayer.isYou && GManager.instance.isAuto && GManager.instance.IsAI)
                {
                    // [Harness mod] Upstream auto mode skipped the local seat's
                    // breeding entirely (jumped straight to Main), so P0 never
                    // hatched and never moved from the breeding area. Run the
                    // same decision the opponent seat uses so the recording
                    // corpus covers breeding for both players.
                    bool doHatch = RandomUtility.IsSucceedProbability(0.85f);

                    if (gameContext.TurnPlayer.CanHatch)
                    {
                        doHatch = true;
                    }

                    SetBreedingPhase(gameContext.TurnPlayer.PlayerID, doHatch);
                }
```

- [ ] **Step 3: Verify both seats breed**

Run one harness job to completion, then inspect the recording:

```bash
python -c "
import json,glob,os
d='/c/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_recordings'
p=max(glob.glob(os.path.join(d,'*.jsonl')), key=os.path.getmtime)
actors={0:set(),1:set()}
for l in open(p,encoding='utf-8-sig'):
    l=l.strip()
    if not l: continue
    o=json.loads(l)
    if o.get('type')=='action' and o.get('action_id') in (60,61):
        actors[o['actor']].add(o['action_id'])
print(p); print('P0 breeding actions:',sorted(actors[0])); print('P1 breeding actions:',sorted(actors[1]))
"
```

Expected: **both** lines non-empty. If `P0 breeding actions: []`, the fix did not take effect.

- [ ] **Step 4: Commit (in the base repo)**

```bash
cd "$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"
git add Assets/Scripts/Script/TurnStateMachine.cs
git commit -m "Harness: auto mode breeds on both seats

Upstream auto mode skipped the local seat's breeding phase entirely, so a
generated corpus would have P0 never hatching and never moving from the
breeding area -- a whole mechanic missing while everything looked fine."
```

---

### Task 8: File the result and take the next job (C#)

**Files:**
- Create: `$BASE_DCGO/Assets/Scripts/Script/Harness/JobResultWriter.cs`
- Modify: `$BASE_DCGO/Assets/Scripts/Script/TurnStateMachine.cs:3605` (the auto-mode restart block)

**Interfaces:**
- Consumes: `JobWatcher.Instance`, `HarnessConfig` from Tasks 5–6.
- Produces: `Digimon.Harness.JobResultWriter.FileResult(string outcome, int steps, string message)`.

- [ ] **Step 1: Write `JobResultWriter.cs`**

```csharp
using System;
using System.IO;
using UnityEngine;

namespace Digimon.Harness
{
    /// <summary>
    /// Writes a job's result sidecar and moves the claimed job file to done/.
    /// </summary>
    public static class JobResultWriter
    {
        /// <param name="outcome">"completed", "partial", or "failed".</param>
        public static void FileResult(string outcome, int steps, string message)
        {
            JobWatcher watcher = JobWatcher.Instance;
            if (watcher == null || watcher.CurrentJob == null) return;

            string jobId = watcher.CurrentJob.job_id;
            double seconds = (DateTime.UtcNow - watcher.StartedUtc).TotalSeconds;
            string recordingPath = Digimon.Recording.GameRecorder.Instance != null
                ? Digimon.Recording.GameRecorder.Instance.CurrentRecordingPath
                : "";

            // Hand-built JSON: JsonUtility cannot emit the snake_case shape the
            // Rust reader expects without a mirror DTO, and this is four fields.
            string json =
                "{\n" +
                "  \"job_id\": " + Quote(jobId) + ",\n" +
                "  \"outcome\": " + Quote(outcome) + ",\n" +
                "  \"recording_path\": " + Quote(recordingPath) + ",\n" +
                "  \"steps\": " + steps + ",\n" +
                "  \"duration_seconds\": " + seconds.ToString("0.00", System.Globalization.CultureInfo.InvariantCulture) + ",\n" +
                "  \"message\": " + Quote(message ?? "") + "\n" +
                "}\n";

            try
            {
                Directory.CreateDirectory(HarnessConfig.DoneDir);
                File.WriteAllText(Path.Combine(HarnessConfig.DoneDir, jobId + ".result.json"), json);

                if (!string.IsNullOrEmpty(watcher.ClaimedPath) && File.Exists(watcher.ClaimedPath))
                {
                    string dest = Path.Combine(HarnessConfig.DoneDir, Path.GetFileName(watcher.ClaimedPath));
                    if (File.Exists(dest)) File.Delete(dest);
                    File.Move(watcher.ClaimedPath, dest);
                }
            }
            catch (Exception e)
            {
                Debug.LogError("[Harness] filing result failed: " + e.Message);
            }

            watcher.ClearCurrentJob();
        }

        private static string Quote(string s)
        {
            if (s == null) return "\"\"";
            return "\"" + s.Replace("\\", "\\\\").Replace("\"", "\\\"") + "\"";
        }
    }
}
```

- [ ] **Step 2: Add the two members `JobResultWriter` needs**

In `JobWatcher.cs`, add:

```csharp
        /// <summary>Release the current job so the poll loop claims the next one.</summary>
        public void ClearCurrentJob()
        {
            CurrentJob = null;
            ClaimedPath = null;
        }
```

In `Digimon.Recording.GameRecorder`, expose the path it is already writing to:

```csharp
        /// <summary>Path of the JSONL file for the game in progress, or "" when idle.</summary>
        public string CurrentRecordingPath { get; private set; } = "";
```

Set it where the writer is opened (the same method that creates `_writer`), assigning the full file path.

- [ ] **Step 3: Hook the end-of-game restart**

At `TurnStateMachine.cs:3605` the auto-mode block currently reloads the scene blindly:

```csharp
        if (GManager.instance.isAuto && GManager.instance.IsAI)
        {
            ContinuousController.instance.isAI = true;
            UnityEngine.SceneManagement.SceneManager.LoadScene("BattleScene");
        }
```

Replace with:

```csharp
        // [Harness mod] Under the job harness, file the result first; the
        // JobWatcher poll loop then claims the next job and loads the scene
        // itself. Plain auto mode (no harness) keeps the blind reload.
        if (Digimon.Harness.JobWatcher.Instance != null
            && Digimon.Harness.JobWatcher.Instance.CurrentJob != null)
        {
            Digimon.Harness.JobResultWriter.FileResult("completed", 0, "");
        }
        else if (GManager.instance.isAuto && GManager.instance.IsAI)
        {
            ContinuousController.instance.isAI = true;
            UnityEngine.SceneManagement.SceneManager.LoadScene("BattleScene");
        }
```

- [ ] **Step 4: Enforce `limits.max_turns`**

The job carries a turn cap but nothing enforces it yet. Without it, a bot that
loops forever (our own engine hit exactly this class of bug — the CannotAttack
mask loop) hangs the batch instead of yielding a usable partial recording.

Add to `JobWatcher`:

```csharp
        private int _turnsSeen;

        /// <summary>
        /// Called at the start of each turn. Abandons the job past its turn cap,
        /// filing a \partial\ result — the truncated recording is still a valid
        /// parity input, and an abandoned game beats a hung batch.
        /// </summary>
        public void NotifyTurnStarted()
        {
            if (CurrentJob == null) return;
            _turnsSeen++;
            if (_turnsSeen <= CurrentJob.limits.max_turns) return;

            Debug.LogWarning("[Harness] job " + CurrentJob.job_id + " hit the turn cap; abandoning");
            JobResultWriter.FileResult("partial", _turnsSeen, "exceeded max_turns");
            _turnsSeen = 0;
            // Reloading kills the running game; the poll loop claims the next job.
            SceneManager.LoadScene("BattleScene");
        }
```

Reset the counter when a job is claimed — in `TryClaimAndStart`, just after
`StartedUtc = DateTime.UtcNow;`:

```csharp
            _turnsSeen = 0;
```

Then call it from the turn-start path in `TurnStateMachine`. Find it with:

```bash
cd "$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"
grep -n "TurnCount\|StartTurn\|IEnumerator TurnStart" Assets/Scripts/Script/TurnStateMachine.cs | head
```

At the top of whichever coroutine begins a turn, add:

```csharp
        // [Harness mod] turn-cap check; no-op outside a harness job.
        if (Digimon.Harness.JobWatcher.Instance != null)
        {
            Digimon.Harness.JobWatcher.Instance.NotifyTurnStarted();
        }
```

- [ ] **Step 5: Verify two jobs drain back to back**

```bash
CARGO_TARGET_DIR='D:\cargo-target-wt\quizzical-ishizaka-07b190' cargo run -p dcgo-harness -- --root "/c/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness" submit --count 2 --decks code/tools/dcgo-harness/testdata/pool.json
```

Press Play in Unity and wait.

```bash
CARGO_TARGET_DIR='D:\cargo-target-wt\quizzical-ishizaka-07b190' cargo run -p dcgo-harness -- --root "/c/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness" status
```

Expected: `pending=0 claimed=0 completed=2 partial=0 failed=0`, and two new `.jsonl` files in `dcgo_recordings/`.

- [ ] **Step 6: Commit (in the base repo, then bump the gitlink)**

```bash
cd "$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"
git add Assets/Scripts/Script/Harness Assets/Scripts/Script/TurnStateMachine.cs Assets/Scripts/Script/Recording/GameRecorder.cs
git commit -m "Harness: file job results and chain to the next job"
git push fork add-recording-mod-r2
```

Then in the worktree:

```bash
DCGO_SHA=$(cd "$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO" && git rev-parse HEAD)
git update-index --cacheinfo 160000,$DCGO_SHA,DCGO
git commit -m "Bump DCGO gitlink: job harness phase 1"
```

---

### Task 9: Determinism check, throughput measurement, and docs

**Files:**
- Create: `docs/DCGO_HARNESS.md`
- Modify: `CLAUDE.md` (the DCGO recording-pipeline bullet under "Documentation")

**Interfaces:**
- Consumes: everything from Tasks 1–8.
- Produces: documentation only.

- [ ] **Step 1: Golden smoke job — verify determinism, the assumption phases 2 and 3 rest on**

This is the spec's "golden smoke job": the only end-to-end gate the C# side has,
since the `Tests~/` asmdef is disabled and cannot reference `Assembly-CSharp`.

Submit the same job twice with an identical seed and compare the recordings:

```bash
ROOT="/c/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness"
REC="/c/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_recordings"
CARGO_TARGET_DIR='D:\cargo-target-wt\quizzical-ishizaka-07b190' cargo run -p dcgo-harness -- --root "$ROOT" submit --count 1 --seed 777 --decks code/tools/dcgo-harness/testdata/pool.json
```

Run it in Unity, note the newest recording, then repeat with the same `--seed 777` and compare the recorded action streams:

```bash
python -c "
import json,glob,os,sys
d='$REC'
fs=sorted(glob.glob(os.path.join(d,'*.jsonl')), key=os.path.getmtime)[-2:]
def stream(p):
    out=[]
    for l in open(p,encoding='utf-8-sig'):
        l=l.strip()
        if not l: continue
        o=json.loads(l)
        if o.get('type')=='action': out.append((o['actor'],o['action_id']))
    return out
a,b=stream(fs[0]),stream(fs[1])
print('identical' if a==b else 'DIVERGED at step %d' % next(i for i,(x,y) in enumerate(zip(a,b)) if x!=y))
print(len(a),'vs',len(b),'actions')
"
```

Expected: `identical`.

**If it diverges, stop and report before starting phase 2 or 3.** Both later phases assume a job is reproducible from its spec; if DCGO's decisions depend on frame timing, that assumption is false and the design needs revisiting. Record the finding in `docs/DCGO_HARNESS.md` either way.

- [ ] **Step 2: Measure throughput**

Submit 10 jobs, time the drain, and record games-per-minute at the configured `TimeScale`:

```bash
ROOT="/c/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness"
CARGO_TARGET_DIR='D:\cargo-target-wt\quizzical-ishizaka-07b190' cargo run -p dcgo-harness -- --root "$ROOT" submit --count 10 --decks code/tools/dcgo-harness/testdata/pool.json
date +%s
```

Run in Unity until `status` reports `completed=10`, then `date +%s` again. Record the rate. If a 200-game batch would exceed a couple of hours, raise `HarnessConfig.TimeScale` and re-measure — but check that games still complete correctly, since very high scales can starve per-frame coroutines.

- [ ] **Step 3: Write `docs/DCGO_HARNESS.md`**

Write it to this outline, filling the two measured values from Steps 1-2:

```markdown
# DCGO Job Harness

Generates a DCGO recording corpus without anyone playing games by hand, then
triages it. Design: `docs/superpowers/specs/2026-08-17-dcgo-automation-harness-design.md`.

## Enabling it
Set `HarnessConfig.Enabled = true` (Digimon.Harness). Default OFF so a stale job
file can never hijack a normal play session.

## Directories
`<persistentDataPath>/dcgo_harness/{jobs,claimed,done,failed}`. A job file moves
between them by atomic rename; that rename IS the claim.

## Job and result shapes
[paste the JobSpec JSON from the plan, and one real .result.json]

## Commands
    dcgo-harness --root <root> submit --count 200 --decks pool.json
    dcgo-harness --root <root> status --sweep
    dcgo-harness --root <root> triage --corpus <recordings> --cards-json data/cards.json

## Measured behavior
- Determinism: [result of Step 1 — identical, or the divergence found]
- Throughput: [games/minute at TimeScale=N from Step 2]

## Rules that matter
- A job overdue past `timeout_seconds` is requeued once, then quarantined.
  Never retried indefinitely: one poisonous job would otherwise stall the batch.
- `status` and `triage` always print pending/claimed/completed/partial/failed.
  A batch where most jobs died must never read as a pass.
- The corpus is derived data and is NOT committed. Only triage reports and
  minimized regression fixtures are.
```

- [ ] **Step 4: Add it to `CLAUDE.md`**

Under "Documentation", extend the existing DCGO recording-pipeline bullet:

```markdown
- **DCGO recording pipeline**: `docs/DCGO_BUILD.md` (build the mod) + `docs/DCGO_RECORDING_SCHEMA.md` (JSONL format) + `docs/DCGO_HARNESS.md` (unattended job harness: generate a corpus without playing games by hand, then triage it) — modded DCGO client that records games as 2192-action-space JSONL, consumed by `code/tools/dcgo-replay/` as an additional Rust-engine faithfulness oracle
```

- [ ] **Step 5: Run the full test suite**

```bash
CARGO_TARGET_DIR='D:\cargo-target-wt\quizzical-ishizaka-07b190' cargo test -p dcgo-harness
CARGO_TARGET_DIR='D:\cargo-target-wt\quizzical-ishizaka-07b190' cargo test -p dcgo-replay
CARGO_TARGET_DIR='D:\cargo-target-wt\quizzical-ishizaka-07b190' RUST_MIN_STACK=268435456 cargo test -p digimon-engine --lib
```

Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add docs/DCGO_HARNESS.md CLAUDE.md
git commit -m "docs: DCGO job harness, with measured determinism and throughput"
```

---

## Done when

- `dcgo-harness submit --count N` queues N jobs; DCGO drains them unattended.
- `status` reports `pending/claimed/completed/partial/failed` and never hides a failure.
- `triage` clusters divergences across the corpus and refuses a clean verdict with zero completed games.
- Recordings show breeding actions (60/61) for **both** seats.
- Two runs of the same seed produce identical action streams — or the divergence is documented and phases 2–3 are re-scoped.
