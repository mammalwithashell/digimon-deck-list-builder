//! `dcgo-harness` — submit DCGO harness jobs, report queue status, triage the
//! resulting corpus.
//!
//! Exit codes:
//!   0 — command succeeded.
//!   1 — command ran but reported failures (e.g. triage found divergences).
//!   2 — argument or I/O error.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde::Deserialize;

use dcgo_harness::job::{JobLimits, DIR_CLAIMED, DIR_DONE, DIR_FAILED, DIR_JOBS};

/// Marker file whose presence tells DCGO the harness is on. Must match
/// HarnessConfig.EnabledMarkerPath on the C# side.
const MARKER_FILE: &str = "harness.enabled";
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
    /// Create the marker file that lets DCGO claim jobs.
    Enable,
    /// Remove the marker file so DCGO ignores the queue.
    Disable,
    /// Report queue counts; sweep overdue claims.
    Status {
        /// Also requeue/quarantine claims older than their budget.
        #[arg(long, default_value_t = false)]
        sweep: bool,
        /// Timeout used when sweeping.
        #[arg(long, default_value_t = 180)]
        timeout_seconds: u64,
    },
    /// Replay every recording in the corpus and rank distinct divergences.
    Triage {
        /// Directory of .jsonl recordings.
        #[arg(long)]
        corpus: PathBuf,
        /// Path to data/cards.json.
        #[arg(long)]
        cards_json: PathBuf,
    },
    /// Run exam scenarios: sim-only (no Unity), or diff against a DCGO sidecar.
    ///
    /// Always prints the full denominator — scenarios seen, lowered, run,
    /// diffed, failed — because a batch where most scenarios died must never
    /// read as a pass.
    Exam {
        /// Scenario file, or a directory searched recursively for *.yaml.
        #[arg(long)]
        scenario: PathBuf,
        /// Run the line in our engine and check `assert:` only. No Unity.
        #[arg(long)]
        sim_only: bool,
        /// DCGO state sidecar to diff against: a `.state.jsonl` file, or a
        /// directory holding one `<scenario-stem>.state.jsonl` per scenario.
        /// Required unless --sim-only.
        #[arg(long)]
        sidecar: Option<PathBuf>,
        /// Path to data/cards.json.
        #[arg(long)]
        cards_json: PathBuf,
        /// Deck-pool JSON resolving each scenario's `rest:` name, in the
        /// `submit --decks` format. Defaults to `starter_decks.json` beside
        /// --cards-json.
        #[arg(long)]
        decks: Option<PathBuf>,
    },
    /// Build a standalone DCGO player and stamp its manifest.
    Build {
        /// Unity editor executable.
        #[arg(long, default_value = "C:/Program Files/Unity/Hub/Editor/2021.3.45f2/Editor/Unity.exe")]
        unity: PathBuf,
        /// DCGO project path (base repo, not a worktree -- CLAUDE.md rule 29).
        #[arg(long)]
        project: PathBuf,
        /// Where the player goes. Must be outside the DCGO submodule.
        #[arg(long)]
        output: PathBuf,
    },
    /// Ensure a DCGO oracle is running against a build.
    Up {
        /// Build directory containing manifest.json.
        #[arg(long)]
        build: PathBuf,
    },
    /// Stop the running DCGO oracle.
    Down,
    /// Supervise a batch end-to-end: keep the oracle running, restart it on
    /// a hang (dead process, or a live process stuck on one job), and drain
    /// the queue. Exits 0 when the queue drains, 1 if the restart budget
    /// runs out with work remaining.
    Watch {
        /// Build directory containing manifest.json.
        #[arg(long)]
        build: PathBuf,
        /// Seconds between polls.
        #[arg(long, default_value_t = 15)]
        poll_seconds: u64,
        /// Heartbeat age past which the process is considered hung (mode 1).
        #[arg(long, default_value_t = dcgo_harness::daemon::DEFAULT_STALE_SECONDS)]
        stale_seconds: u64,
        /// How many hang-triggered restarts to allow before giving up.
        #[arg(long, default_value_t = 3)]
        max_restarts: u32,
        /// Recordings corpus directory, used as the forward-progress signal
        /// that breaks the tie on an overdue claim (mode 2 is necessary but
        /// not sufficient -- see `watch::classify`). Falls back to the
        /// player log's own mtime under --root when omitted.
        #[arg(long)]
        corpus: Option<PathBuf>,
        /// How old the progress signal may be before an overdue claim is
        /// judged truly stalled rather than just a long game.
        #[arg(long, default_value_t = dcgo_harness::watch::DEFAULT_PROGRESS_STALE_SECONDS)]
        progress_stale_seconds: u64,
    },
}

fn main() -> ExitCode {
    let args = Args::parse();
    // Run the real work on a worker thread with a large stack. `triage`
    // replays recordings through `dcgo_replay::replay_recording`, which
    // constructs the engine's `CardEffectRegistry` — that recurses deeply
    // enough to overflow the OS-default main-thread stack on Windows
    // (~1 MB), aborting with STATUS_STACK_OVERFLOW. `RUST_MIN_STACK` only
    // governs spawned threads, not `main`, so the fix is to spawn one
    // explicitly. Mirrors `digimon-engine-cli/src/main.rs`.
    let stack_size = std::env::var("RUST_MIN_STACK")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(256 * 1024 * 1024);
    std::thread::Builder::new()
        .stack_size(stack_size)
        .spawn(move || match run(&args) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("error: {}", e);
                ExitCode::from(2)
            }
        })
        .expect("failed to spawn worker thread")
        .join()
        .expect("worker thread panicked")
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
        Command::Enable => {
            let marker = args.root.join(MARKER_FILE);
            std::fs::write(&marker, "enabled by dcgo-harness
")
                .map_err(|e| format!("writing {}: {}", marker.display(), e))?;
            println!("harness ENABLED ({})", marker.display());
            println!("DCGO will claim jobs on its next Play. Run 'disable' when done.");
            Ok(ExitCode::SUCCESS)
        }
        Command::Disable => {
            let marker = args.root.join(MARKER_FILE);
            match std::fs::remove_file(&marker) {
                Ok(()) => println!("harness disabled ({} removed)", marker.display()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    println!("harness already disabled (no {})", marker.display())
                }
                Err(e) => return Err(format!("removing {}: {}", marker.display(), e)),
            }
            Ok(ExitCode::SUCCESS)
        }
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
            // Surface the enable state. A queue full of pending jobs with the
            // harness switched off looks exactly like a hung DCGO otherwise --
            // the same silent-failure shape the triage denominator guards.
            let enabled = args.root.join(MARKER_FILE).exists();
            println!("harness: {}", if enabled { "ENABLED" } else { "disabled" });
            // A queue that is not draining looks identical whether DCGO is
            // stopped, hung, or simply switched off. Say which.
            match dcgo_harness::daemon::read_pid(&args.root) {
                Some(pid) if dcgo_harness::daemon::pid_alive(pid) => {
                    let health = dcgo_harness::daemon::classify_heartbeat(
                        dcgo_harness::daemon::heartbeat_age(&args.root),
                        dcgo_harness::daemon::DEFAULT_STALE_SECONDS,
                    );
                    println!("process: pid {}, heartbeat {:?}", pid, health);
                }
                Some(pid) => println!("process: pid {} recorded but not alive (crashed)", pid),
                None => println!("process: not running"),
            }
            println!("{}", status.summary());
            if !enabled && status.pending > 0 {
                println!(
                    "note: {} job(s) queued but the harness is disabled -- run 'enable'.",
                    status.pending
                );
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Triage { corpus, cards_json } => {
            use dcgo_harness::triage::{cluster, scan_corpus, TriageReport};

            let card_data = dcgo_replay::load_card_data_at(cards_json)
                .map_err(|e| format!("loading cards.json: {}", e))?;

            // Shared with `dcgo-replay` so both tools agree on what counts
            // as a recording in the corpus and process it in the same
            // (sorted, deterministic) order — see F3/F5b.
            let recording_paths = dcgo_replay::collect_recording_paths(corpus)
                .map_err(|e| format!("collecting corpus {}: {}", corpus.display(), e))?;

            // G3 (second triage review pass): the read -> parse -> replay ->
            // tally loop used to live inline here, exercised only by a
            // manual corpus run — and this handler is exactly where the
            // earlier Critical finding (F1: wrong denominator reaching the
            // report) actually lived. It now lives in `triage::scan_corpus`,
            // which has direct unit-test coverage over an on-disk fixture
            // corpus; this handler just calls it and prints the report.
            let (stats, findings) = scan_corpus(&recording_paths, &card_data);

            let status = dcgo_harness::queue::scan(&args.root)?;
            let report = TriageReport {
                status,
                corpus_stats: stats,
                clusters: cluster(&findings),
            };
            print!("{}", report.render());
            // The exit code follows the verdict, not just `findings`: a
            // corpus that failed to replay at all (inconclusive) or that
            // failed for reasons `cluster()` doesn't track (unclustered
            // failures — F6) must both exit non-zero, not just the case
            // where clustering itself found something (F2).
            Ok(if report.is_conclusive() && report.is_clean() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        Command::Exam {
            scenario,
            sim_only,
            sidecar,
            cards_json,
            decks,
        } => run_exam(scenario, *sim_only, sidecar.as_deref(), cards_json, decks.as_deref()),
        Command::Build {
            unity,
            project,
            output,
        } => {
            let req = dcgo_harness::build::BuildRequest {
                unity_exe: unity.clone(),
                project_path: project.clone(),
                output_dir: output.clone(),
            };
            let m = dcgo_harness::build::run(&req)?;
            println!("built {}", output.display());
            println!("  dcgo_commit       {}", m.dcgo_commit);
            println!("  artifact_sha256   {}", m.artifact_sha256);
            println!("  action_space_hash {}", m.action_space_hash);
            Ok(ExitCode::SUCCESS)
        }
        Command::Up { build } => {
            println!("{}", dcgo_harness::daemon::up(&args.root, build)?);
            Ok(ExitCode::SUCCESS)
        }
        Command::Down => {
            println!("{}", dcgo_harness::daemon::down(&args.root)?);
            Ok(ExitCode::SUCCESS)
        }
        Command::Watch {
            build,
            poll_seconds,
            stale_seconds,
            max_restarts,
            corpus,
            progress_stale_seconds,
        } => {
            let outcome = dcgo_harness::watch::run(
                &args.root,
                build,
                std::time::Duration::from_secs(*poll_seconds),
                *stale_seconds,
                *max_restarts,
                corpus.as_deref(),
                *progress_stale_seconds,
            )?;

            // Always print the full denominator, win or lose -- a batch
            // where most jobs died must never read as a success.
            println!(
                "watch: {}",
                if outcome.drained {
                    "drained"
                } else {
                    "restart budget exhausted with work remaining"
                }
            );
            println!(
                "watch: restarts used {}/{}",
                outcome.restarts_used, outcome.max_restarts
            );
            if outcome.events.is_empty() {
                println!("watch: no hangs detected");
            } else {
                for (i, event) in outcome.events.iter().enumerate() {
                    println!("watch: hang #{}: {}", i + 1, event);
                }
            }
            println!("{}", outcome.final_status.summary());

            Ok(if outcome.drained {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
    }
}

// ---------------------------------------------------------------------------
// `exam` — run scripted clause scenarios, sim-only or against the DCGO oracle.
// ---------------------------------------------------------------------------

/// One scenario's outcome. Kept as data so the denominator below counts the
/// same facts the per-scenario lines printed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExamOutcome {
    /// Parsed, lowered, ran, and its checks passed.
    Passed,
    /// Failed before the line could be lowered (parse, deck, or lowering).
    LowerFailed,
    /// Lowered and ran, but the checks failed (assertion or oracle diff).
    CheckFailed,
}

fn run_exam(
    scenario: &Path,
    sim_only: bool,
    sidecar: Option<&Path>,
    cards_json: &Path,
    decks: Option<&Path>,
) -> Result<ExitCode, String> {
    if !sim_only && sidecar.is_none() {
        return Err(
            "exam needs --sidecar <dcgo .state.jsonl> unless --sim-only. Without an \
             oracle trace there is nothing to diff against, and silently degrading \
             to an assertion-only run would report oracle agreement nobody measured."
                .to_string(),
        );
    }

    // A path that does not exist is an error (`collect_scenario_paths`), so a
    // typo can never masquerade as a clean exam. A path that DOES exist but
    // holds no scenarios yet is a legitimate state -- it just has to say, in
    // the denominator, that it measured nothing.
    let paths = collect_scenario_paths(scenario)?;
    if paths.is_empty() {
        println!(
            "exam: scenarios seen 0 / lowered 0 / run 0 / diffed 0 / failed 0"
        );
        println!(
            "exam: NOTHING MEASURED -- no *.yaml scenarios under {}. This run \
             establishes nothing about any clause; it is not a pass.",
            scenario.display()
        );
        return Ok(ExitCode::SUCCESS);
    }

    let card_data = dcgo_replay::load_card_data_at(cards_json)
        .map_err(|e| format!("loading {}: {}", cards_json.display(), e))?;
    let book = DeckBook::load(decks, cards_json)?;

    let mut seen = 0u32;
    let mut lowered = 0u32;
    let mut ran = 0u32;
    let mut diffed = 0u32;
    let mut outcomes: Vec<ExamOutcome> = Vec::new();

    for path in &paths {
        seen += 1;
        println!("exam: {}", path.display());
        let outcome = exam_one(
            path,
            sim_only,
            sidecar,
            paths.len(),
            &card_data,
            &book,
            &mut lowered,
            &mut ran,
            &mut diffed,
        );
        match &outcome {
            Ok(o) => outcomes.push(*o),
            Err(e) => {
                println!("  FAILED: {e}");
                outcomes.push(ExamOutcome::LowerFailed);
            }
        }
    }

    let lower_failed = outcomes
        .iter()
        .filter(|o| **o == ExamOutcome::LowerFailed)
        .count();
    let check_failed = outcomes
        .iter()
        .filter(|o| **o == ExamOutcome::CheckFailed)
        .count();
    let failed = lower_failed + check_failed;

    // The full denominator, every run, pass or fail.
    println!(
        "exam: scenarios seen {seen} / lowered {lowered} / run {ran} / diffed {diffed} / failed {failed}"
    );
    if lower_failed > 0 {
        println!(
            "exam: {lower_failed} scenario(s) never lowered -- they measured NOTHING, \
             which is not the same as agreeing."
        );
    }
    println!(
        "exam: mode {}",
        if sim_only {
            "sim-only (no oracle: this can only re-check what a previous oracle run confirmed)"
        } else {
            "oracle diff"
        }
    );

    Ok(if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

#[allow(clippy::too_many_arguments)]
fn exam_one(
    path: &Path,
    sim_only: bool,
    sidecar: Option<&Path>,
    scenario_count: usize,
    card_data: &std::collections::HashMap<String, digimon_engine::CardData>,
    book: &DeckBook,
    lowered: &mut u32,
    ran: &mut u32,
    diffed: &mut u32,
) -> Result<ExamOutcome, String> {
    use dcgo_harness::exam::adapter::ScenarioAdapter;
    use dcgo_harness::exam::differ::diff;
    use dcgo_harness::exam::projection::{align_to_scenario_origin, parse_sidecar, StateProjection};
    use dcgo_harness::exam::scenario::Scenario;
    use digimon_engine::runners::replay::ReplaySession;

    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("reading {}: {e}", path.display()))?;
    let s = Scenario::from_yaml(&text)?;

    // Resolve the oracle artifacts BEFORE building our game, because in oracle
    // mode the decks come from the recording, not from the scenario's `rest:`.
    let oracle: Option<(std::path::PathBuf, String)> = if sim_only {
        None
    } else {
        let sp = resolve_sidecar(sidecar.expect("checked by run_exam"), path, scenario_count)?;
        let rp = {
            let s = sp.to_string_lossy();
            std::path::PathBuf::from(
                s.strip_suffix(".state.jsonl")
                    .map(|stem| format!("{stem}.jsonl"))
                    .unwrap_or_else(|| s.to_string()),
            )
        };
        let rt = std::fs::read_to_string(&rp).map_err(|e| {
            format!(
                "reading the recording beside the sidecar ({}): {e}. It is required both                  to align the oracle trace to the scenario's origin and to take DCGO's                  post-shuffle deck order.",
                rp.display()
            )
        })?;
        Some((sp, rt))
    };

    // In oracle mode, take the decks VERBATIM from DCGO's own post-shuffle
    // order rather than from `rest:`.
    //
    // Otherwise the two engines play different games and every scenario
    // diverges on hand contents at step 0. `Game::new_with_ordered_decks` is
    // the replay constructor and does NOT shuffle -- its doc says "the order IS
    // the recorded post-shuffle order" -- so feeding it a decklist gave both
    // seats the identical top five cards while DCGO had seeded-shuffled each.
    // Observed exactly that: ours p0.hand == p1.hand, DCGO's two hands distinct
    // and different again from ours.
    //
    // Reimplementing DCGO's Fisher-Yates over its Xoshiro256** stream in Rust
    // would be the alternative, and it would be a second copy of a shuffle whose
    // definition lives in DCGO -- exactly the kind of mirrored table the job
    // spec avoids by sending card IDs instead of deck codes.
    let (deck_p0, deck_p1) = match &oracle {
        Some((_, rt)) => decks_from_recording(rt)?,
        None => (
            ordered_deck(&s.decks.p0, book)?,
            ordered_deck(&s.decks.p1, book)?,
        ),
    };

    // Lowering resolves every symbolic step against the live mask, so an
    // illegal or ambiguous line fails HERE -- in milliseconds, before any
    // Unity time is spent.
    let adapter = ScenarioAdapter::from_scenario(&s, deck_p0, deck_p1, card_data)?;
    *lowered += 1;
    println!("  lowered {} step(s): {:?}", s.steps.len(), adapter.lowered_action_ids());

    let mut session = ReplaySession::with_source(Box::new(adapter), card_data, false)
        .map_err(|e| format!("building the replay session: {e:?}"))?;

    // Step exactly as many times as the line is long, projecting after each.
    // A bounded loop rather than `while !is_complete()`: a source that stops
    // advancing would otherwise spin forever and read as a hang.
    let mut projections = vec![StateProjection::from_game(&session.game, 0)];
    for i in 0..s.steps.len() as u32 {
        session.step();
        projections.push(StateProjection::from_game(&session.game, i + 1));
    }
    *ran += 1;

    if !session.is_complete() {
        return Ok(fail(format!(
            "the line did not run to completion: {} of {} steps",
            session.current_step(),
            session.total_steps()
        )));
    }

    if sim_only {
        let (checked, failures) = check_assertions(&s, &projections);
        for f in &failures {
            println!("  ASSERT FAILED: {f}");
        }
        println!(
            "  assert: {checked} check(s) over {} assertion block(s), {} failed",
            s.assertions.len(),
            failures.len()
        );
        if s.assertions.is_empty() {
            // Not a failure -- a scenario can legitimately be authored before
            // the oracle has run -- but it must never be silently counted as
            // a passing gate.
            println!(
                "  note: no `assert:` block, so this scenario checked NOTHING. \
                 Run the oracle pass and backfill before trusting it in CI."
            );
        }
        return Ok(if failures.is_empty() {
            ExamOutcome::Passed
        } else {
            ExamOutcome::CheckFailed
        });
    }

    let (sidecar_path, recording_text) = oracle.expect("built above when !sim_only");
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
    // StateDumper writes one before each decision and stops -- 14 vs 13 for a
    // 13-step line. Dropping our trailing entry aligns the two; keeping it made
    // the differ report TRUNCATED (correctly -- it refuses to call an unequal
    // comparison clean) on a run that had no divergence at all.
    //
    // `projections` itself keeps the trailing state, because an `assert:` block
    // legitimately wants to talk about the position AFTER the final step.
    let ours_for_diff: Vec<_> = projections
        .iter()
        .take(s.steps.len())
        .cloned()
        .collect();
    let report = diff(&ours_for_diff, &dcgo);
    *diffed += 1;
    println!("  {report}");

    Ok(if report.is_clean() {
        ExamOutcome::Passed
    } else {
        ExamOutcome::CheckFailed
    })
}

fn fail(message: String) -> ExamOutcome {
    println!("  FAILED: {message}");
    ExamOutcome::CheckFailed
}

/// Every `*.yaml` / `*.yml` under `root`, sorted. A file path is returned as
/// itself, whatever its extension, so `--scenario one.yaml` always works.
fn collect_scenario_paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    if root.is_file() {
        return Ok(vec![root.to_path_buf()]);
    }
    if !root.is_dir() {
        return Err(format!("no such scenario path: {}", root.display()));
    }
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir)
            .map_err(|e| format!("reading {}: {e}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("reading {}: {e}", dir.display()))?;
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if matches!(
                p.extension().and_then(|e| e.to_str()),
                Some("yaml") | Some("yml")
            ) {
                out.push(p);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Which `.state.jsonl` belongs to this scenario.
///

/// Both seats' decks in DCGO's own post-shuffle order, taken from a recording's
/// `game_start` row.
///
/// The row is written from one seat's perspective (`my_player_id`), so the two
/// lists have to be assigned by that id rather than positionally. Egg cards are
/// appended: `Game::new_inner` splits them out by card kind, and the scenario's
/// deck argument is one flat list per seat.
fn decks_from_recording(text: &str) -> Result<(Vec<String>, Vec<String>), String> {
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

/// A single sidecar file paired with a whole directory of scenarios is an
/// error, not a fallback: diffing every scenario against one trace would
/// manufacture divergences that say nothing about any of them.
fn resolve_sidecar(
    sidecar: &Path,
    scenario_path: &Path,
    scenario_count: usize,
) -> Result<PathBuf, String> {
    if sidecar.is_dir() {
        let stem = scenario_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("unnamed scenario {}", scenario_path.display()))?;
        return Ok(sidecar.join(format!("{stem}.state.jsonl")));
    }
    if scenario_count > 1 {
        return Err(format!(
            "--sidecar {} is a single file but {scenario_count} scenarios were \
             selected; pass a directory holding one <stem>.state.jsonl per scenario",
            sidecar.display()
        ));
    }
    Ok(sidecar.to_path_buf())
}

// --- assertions -------------------------------------------------------------

/// Check every `assert:` block against the projected trace.
///
/// Returns `(checks_made, failures)`. An unknown key is a **failure**, not a
/// skip: silently ignoring it would let a typo'd assertion report a pass while
/// checking nothing.
fn check_assertions(
    s: &dcgo_harness::exam::scenario::Scenario,
    projections: &[dcgo_harness::exam::projection::StateProjection],
) -> (u32, Vec<String>) {
    let mut checked = 0u32;
    let mut failures = Vec::new();

    for a in &s.assertions {
        let Some(p) = projections.iter().find(|p| p.step == a.at) else {
            failures.push(format!(
                "at {}: no projected state for that step (the trace has {:?})",
                a.at,
                projections.iter().map(|p| p.step).collect::<Vec<_>>()
            ));
            continue;
        };
        for (key, expected) in &a.that {
            if key == dcgo_harness::exam::backfill::GENERATED_MARKER {
                continue; // provenance metadata, not a projection path
            }
            checked += 1;
            match projected_value(p, key) {
                None => failures.push(format!(
                    "at {}: unknown assertion key `{key}` -- supported: {}",
                    a.at,
                    ASSERTION_KEYS.join(", ")
                )),
                Some(actual) => {
                    if !values_equal(expected, &actual) {
                        failures.push(format!(
                            "at {}: {key} expected {} but our engine has {}",
                            a.at,
                            render_value(expected),
                            render_value(&actual)
                        ));
                    }
                }
            }
        }
    }

    (checked, failures)
}

const ASSERTION_KEYS: &[&str] = &[
    "turn",
    "phase",
    "memory",
    "p0.memory",
    "p1.memory",
    "p{0,1}.security",
    "p{0,1}.hand",
    "p{0,1}.trash",
    "p{0,1}.field",
];

fn projected_value(
    p: &dcgo_harness::exam::projection::StateProjection,
    key: &str,
) -> Option<serde_yml::Value> {
    let v = |x: Result<serde_yml::Value, _>| x.ok();
    match key {
        "turn" => v(serde_yml::to_value(p.turn)),
        "phase" => v(serde_yml::to_value(&p.phase)),
        // The projection pins memory to player 0's perspective, so `p1.memory`
        // is its negation rather than a second stored field.
        "memory" | "p0.memory" => v(serde_yml::to_value(p.memory)),
        "p1.memory" => v(serde_yml::to_value(-p.memory)),
        _ => {
            let (seat, field) = key.split_once('.')?;
            let s = match seat {
                "p0" => &p.p0,
                "p1" => &p.p1,
                _ => return None,
            };
            match field {
                "security" => v(serde_yml::to_value(s.security)),
                "hand" => v(serde_yml::to_value(&s.hand)),
                "trash" => v(serde_yml::to_value(&s.trash)),
                "field" => v(serde_yml::to_value(&s.field)),
                _ => None,
            }
        }
    }
}

/// Compare an authored value with a projected one.
///
/// Sequences compare as **multisets**: the projection sorts zones on
/// construction because zone order is representation, not semantics, and an
/// author who wrote a hand in a different order is making the same claim.
/// Numbers compare numerically so `5` and `5.0` are not a divergence.
fn values_equal(expected: &serde_yml::Value, actual: &serde_yml::Value) -> bool {
    use serde_yml::Value;
    match (expected, actual) {
        (Value::Sequence(a), Value::Sequence(b)) => {
            if a.len() != b.len() {
                return false;
            }
            let mut a: Vec<String> = a.iter().map(render_value).collect();
            let mut b: Vec<String> = b.iter().map(render_value).collect();
            a.sort();
            b.sort();
            a == b
        }
        (Value::Number(a), Value::Number(b)) => match (a.as_f64(), b.as_f64()) {
            (Some(x), Some(y)) => x == y,
            _ => a == b,
        },
        _ => expected == actual,
    }
}

fn render_value(v: &serde_yml::Value) -> String {
    serde_yml::to_string(v)
        .unwrap_or_else(|_| format!("{v:?}"))
        .trim_end()
        .replace('\n', " ")
}

// --- decks ------------------------------------------------------------------

/// Resolves a scenario's `rest:` name to a card list.
///
/// Two sources, both already in the repo: the harness deck-pool JSON that
/// `submit --decks` consumes, and `data/starter_decks.json`. Nothing here
/// invents a deck — an unknown name is an error naming what IS available,
/// because a silently-substituted deck would change the line under test.
struct DeckBook {
    /// lowercased name -> card ids (main + eggs; `Game::new` routes by kind).
    by_name: BTreeMap<String, Vec<String>>,
    source: String,
}

#[derive(Debug, Deserialize)]
struct StarterDeckFile {
    starter_decks: Vec<StarterDeck>,
}

#[derive(Debug, Deserialize)]
struct StarterDeck {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    set: String,
    #[serde(default)]
    main_deck: Vec<String>,
    #[serde(default)]
    egg_deck: Vec<String>,
}

impl DeckBook {
    fn load(decks: Option<&Path>, cards_json: &Path) -> Result<DeckBook, String> {
        let mut by_name: BTreeMap<String, Vec<String>> = BTreeMap::new();

        if let Some(path) = decks {
            let pool = pool::load_pool(path)?;
            for d in pool.decks {
                let mut cards = d.cards.clone();
                cards.extend(d.eggs.iter().cloned());
                by_name.insert(d.name.to_lowercase(), cards);
            }
            return Ok(DeckBook {
                by_name,
                source: path.display().to_string(),
            });
        }

        let path = cards_json
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("starter_decks.json");
        let text = std::fs::read_to_string(&path).map_err(|e| {
            format!(
                "no --decks given and the default deck book {} is unreadable: {e}",
                path.display()
            )
        })?;
        let file: StarterDeckFile = serde_json::from_str(&text)
            .map_err(|e| format!("parsing {}: {e}", path.display()))?;
        for d in file.starter_decks {
            let mut cards = d.main_deck.clone();
            cards.extend(d.egg_deck.iter().cloned());
            // Registered under every name a scenario might reasonably use.
            for alias in [&d.id, &d.name, &d.set] {
                if !alias.is_empty() {
                    by_name.insert(alias.to_lowercase(), cards.clone());
                }
            }
        }
        Ok(DeckBook {
            by_name,
            source: path.display().to_string(),
        })
    }

    fn resolve(&self, rest: &str) -> Result<&Vec<String>, String> {
        self.by_name.get(&rest.to_lowercase()).ok_or_else(|| {
            format!(
                "unknown deck `{rest}` (from the scenario's `rest:`); {} knows: {}",
                self.source,
                self.by_name.keys().cloned().collect::<Vec<_>>().join(", ")
            )
        })
    }
}

/// Build one seat's ordered deck: the named list, with the scenario's `stack`
/// moved to the top.
///
/// `Player::draw` **pops the end** of the list, so the top of the deck is the
/// LAST element. The stack is therefore appended in reverse, which makes
/// `stack[0]` the first card drawn — the order an author reads it in.
fn ordered_deck(
    seat: &dcgo_harness::exam::scenario::ScenarioSeat,
    book: &DeckBook,
) -> Result<Vec<String>, String> {
    let list = book.resolve(&seat.rest)?;
    let mut remainder = list.clone();
    for id in &seat.stack {
        match remainder.iter().position(|c| c == id) {
            Some(i) => {
                remainder.remove(i);
            }
            None => {
                return Err(format!(
                    "stacked card {id} is not in deck `{}` -- stacking it would \
                     silently change the deck list the scenario claims to use",
                    seat.rest
                ))
            }
        }
    }
    remainder.extend(seat.stack.iter().rev().cloned());
    Ok(remainder)
}
