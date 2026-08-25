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
use serde::{Deserialize, Serialize};

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
        /// Record one per-clause verdict per scenario into this store:
        /// Confirmed on a CLEAN oracle diff, Diverged on a divergence. Bare
        /// `--verdicts` targets the standard store. Ignored under --sim-only
        /// (sim-only cannot confirm anything). Requires --clause-text-json.
        #[arg(
            long,
            num_args = 0..=1,
            default_missing_value = "qa/qa-reports/dcgo_exam_verdicts.json"
        )]
        verdicts: Option<PathBuf>,
        /// `clause_coverage extract` output supplying each clause's label and
        /// text (hashed into the verdict as its drift fingerprint). A verdict
        /// is REFUSED for any clause id absent from this file.
        #[arg(long)]
        clause_text_json: Option<PathBuf>,
        /// Also write one DCGO scripted-job JSON per lowered scenario into
        /// this directory (sim-only: in oracle mode the decks come from the
        /// recording, not the deck book).
        #[arg(long)]
        emit_job: Option<PathBuf>,
        /// Print EVERY divergence field-by-field, not just the lead plus a
        /// downstream count. The default is lead-only because a report ranking
        /// fifty consequences beside one cause is a report nobody finishes -
        /// but triaging a multi-gate line otherwise means DERIVING which rows
        /// diverged from that count instead of reading them.
        #[arg(long, alias = "verbose")]
        all_diffs: bool,
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
            verdicts,
            clause_text_json,
            emit_job,
            all_diffs,
        } => run_exam(
            scenario,
            *sim_only,
            sidecar.as_deref(),
            cards_json,
            decks.as_deref(),
            verdicts.as_deref(),
            clause_text_json.as_deref(),
            emit_job.as_deref(),
            *all_diffs,
        ),
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

/// What one oracle-mode scenario run established about its clause, before the
/// verdict store gets involved.
struct VerdictEvent {
    clause_id: String,
    verdict: dcgo_harness::exam::verdict::Verdict,
    reason: Option<String>,
    scenario_path: String,
}

#[allow(clippy::too_many_arguments)]
fn run_exam(
    scenario: &Path,
    sim_only: bool,
    sidecar: Option<&Path>,
    cards_json: &Path,
    decks: Option<&Path>,
    verdicts: Option<&Path>,
    clause_text_json: Option<&Path>,
    emit_job: Option<&Path>,
    all_diffs: bool,
) -> Result<ExitCode, String> {
    use dcgo_harness::exam::verdict::{ClauseTextBook, VerdictStore};

    if !sim_only && sidecar.is_none() {
        return Err(
            "exam needs --sidecar <dcgo .state.jsonl> unless --sim-only. Without an \
             oracle trace there is nothing to diff against, and silently degrading \
             to an assertion-only run would report oracle agreement nobody measured."
                .to_string(),
        );
    }
    if emit_job.is_some() && !sim_only {
        return Err(
            "--emit-job needs --sim-only: in oracle mode the decks come from the \
             recording, not the deck book, so the emitted job would not describe \
             the game that was lowered."
                .to_string(),
        );
    }

    // Resolve the verdict store up front so a bad path or missing clause-text
    // file fails before any scenario runs.
    let mut verdict_ctx: Option<(PathBuf, ClauseTextBook, VerdictStore)> = match verdicts {
        Some(path) if sim_only => {
            println!(
                "exam: --verdicts ignored under --sim-only -- sim-only cannot confirm \
                 anything, so the store at {} is left untouched.",
                path.display()
            );
            None
        }
        Some(path) => {
            let ctj = clause_text_json.ok_or_else(|| {
                "--verdicts needs --clause-text-json <extract output>: the scenario \
                 file only names a clause id, and the verdict must carry the clause's \
                 label and text sha256 from the clause_coverage denominator."
                    .to_string()
            })?;
            let book = ClauseTextBook::load(ctj)?;
            let store = if path.exists() {
                VerdictStore::load(path)?
            } else {
                VerdictStore::default()
            };
            Some((path.to_path_buf(), book, store))
        }
        None => None,
    };

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
    let mut events: Vec<VerdictEvent> = Vec::new();

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
            emit_job,
            all_diffs,
            &mut lowered,
            &mut ran,
            &mut diffed,
        );
        match outcome {
            Ok((o, event)) => {
                outcomes.push(o);
                events.extend(event);
            }
            Err(e) => {
                println!("  FAILED: {e}");
                outcomes.push(ExamOutcome::LowerFailed);
            }
        }
    }

    // Write the verdict store, refusing orphan clause ids. A refusal is loud
    // AND fails the run: a scenario keyed outside the denominator is a
    // scenario that covers nothing, whatever its diff said.
    let mut verdicts_refused = 0usize;
    if let Some((store_path, book, store)) = verdict_ctx.as_mut() {
        let recorded_at = chrono::Utc::now().to_rfc3339();
        for ev in &events {
            match dcgo_harness::exam::verdict::record_scenario_verdict(
                store,
                book,
                &ev.clause_id,
                ev.verdict,
                Some(ev.scenario_path.clone()),
                ev.reason.clone(),
                recorded_at.clone(),
            ) {
                Ok(()) => println!(
                    "exam: verdict {} recorded for {} ({})",
                    ev.verdict, ev.clause_id, ev.scenario_path
                ),
                Err(e) => {
                    println!("exam: VERDICT NOT RECORDED: {e}");
                    verdicts_refused += 1;
                }
            }
        }
        // Tell the store what every clause's text hashes to right now, so
        // stale verdicts from before a text change report as invalidated.
        let all_ids = book.clause_ids();
        for id in &all_ids {
            if let Some(ct) = book.get(id) {
                store.set_current_text_sha(id, &dcgo_harness::exam::verdict::sha256_hex(&ct.text));
            }
        }
        store.save(store_path)?;
        println!(
            "exam: verdict store {} -- {}",
            store_path.display(),
            store.summary(&all_ids).describe()
        );
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

    if verdicts_refused > 0 {
        println!(
            "exam: {verdicts_refused} verdict(s) refused (clause id outside the \
             clause-text denominator) -- failing the run."
        );
    }

    Ok(if failed == 0 && verdicts_refused == 0 {
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
    emit_job: Option<&Path>,
    all_diffs: bool,
    lowered: &mut u32,
    ran: &mut u32,
    diffed: &mut u32,
) -> Result<(ExamOutcome, Option<VerdictEvent>), String> {
    use dcgo_harness::exam::adapter::ScenarioAdapter;
    use dcgo_harness::exam::differ::diff_paired;
    use dcgo_harness::exam::projection::{
        align_to_scenario_origin, pair_by_wire_rows, parse_sidecar, StateProjection,
    };
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
    println!("  lowered {} step(s): {:?}", s.steps.len(), adapter.lowered_steps());
    // Captured BEFORE the adapter moves into the replay session: this is how
    // the differ pairs our per-STEP rows against DCGO's per-DECISION rows.
    let wire_rows_per_step = adapter.dcgo_wire_rows_per_step();

    // Sim-only path (checked in run_exam): the line lowered cleanly, so emit
    // the DCGO scripted job that runs the same line against the oracle.
    if let Some(dir) = emit_job {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("unnamed scenario {}", path.display()))?;
        let e0 = book.resolve(&s.decks.p0.rest)?;
        let e1 = book.resolve(&s.decks.p1.rest)?;
        let job = build_exam_job(stem, &s, e0, e1, adapter.lowered_steps())?;
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("creating {}: {e}", dir.display()))?;
        let out = dir.join(format!("{}.json", job.job_id));
        let mut text = serde_json::to_string_pretty(&job)
            .map_err(|e| format!("serializing job for {}: {e}", path.display()))?;
        text.push('\n');
        std::fs::write(&out, text).map_err(|e| format!("writing {}: {e}", out.display()))?;
        println!("  emitted job {}", out.display());
    }

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
        return Ok((
            fail(format!(
                "the line did not run to completion: {} of {} steps",
                session.current_step(),
                session.total_steps()
            )),
            None,
        ));
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
        return Ok((
            if failures.is_empty() {
                ExamOutcome::Passed
            } else {
                ExamOutcome::CheckFailed
            },
            // Sim-only cannot confirm; it never produces a verdict event.
            None,
        ));
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
    // StateDumper writes one before each decision and stops. Dropping our
    // trailing entry aligns the two ends; keeping it made the differ report
    // TRUNCATED (correctly -- it refuses to call an unequal comparison clean)
    // on a run that had no divergence at all.
    //
    // `projections` itself keeps the trailing state, because an `assert:` block
    // legitimately wants to talk about the position AFTER the final step.
    let ours_for_diff: Vec<_> = projections
        .iter()
        .take(s.steps.len())
        .cloned()
        .collect();
    // ...and pair the two by LOWERED STEP rather than 1:1, because a step can
    // consume one, two, or ZERO DCGO decision rows (an OptionalSkill+pick fold
    // writes two; a sim-only phase exit writes none). A positional pairing
    // slides apart from the first multi-row step onward and manufactures
    // divergences out of the offset -- measured on EX12-011#effect#0, where our
    // post-battle row was compared against DCGO's pre-battle mid-fold row.
    let pairing = pair_by_wire_rows(&wire_rows_per_step, dcgo.len());
    let report = diff_paired(&ours_for_diff, &dcgo, &pairing);
    *diffed += 1;
    if all_diffs {
        for line in report.render_verbose().lines() {
            println!("  {line}");
        }
    } else {
        println!("  {report}");
    }

    // What this run established about the clause, for the verdict store: a
    // CLEAN oracle diff confirms, anything else (divergence or truncation)
    // is a finding to triage.
    let event = if report.is_clean() {
        VerdictEvent {
            clause_id: s.clause.clone(),
            verdict: dcgo_harness::exam::verdict::Verdict::Confirmed,
            reason: None,
            scenario_path: path.display().to_string(),
        }
    } else {
        VerdictEvent {
            clause_id: s.clause.clone(),
            verdict: dcgo_harness::exam::verdict::Verdict::Diverged,
            reason: Some(
                format!("{report}")
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_string(),
            ),
            scenario_path: path.display().to_string(),
        }
    };

    Ok((
        if report.is_clean() {
            ExamOutcome::Passed
        } else {
            ExamOutcome::CheckFailed
        },
        Some(event),
    ))
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

/// One named deck, main and eggs kept apart. `ordered_deck` flattens them for
/// our engine (`Game::new_inner` re-splits by card kind), while `--emit-job`
/// needs the boundary: a DCGO job's flat list is main-then-eggs by
/// convention, and a job silently emitted with no eggs would run a different
/// game than the scenario claims.
#[derive(Debug, Clone)]
struct DeckEntry {
    main: Vec<String>,
    eggs: Vec<String>,
}

/// Resolves a scenario's `rest:` name to a card list.
///
/// Two sources, both already in the repo: the harness deck-pool JSON that
/// `submit --decks` consumes, and `data/starter_decks.json`. Nothing here
/// invents a deck — an unknown name is an error naming what IS available,
/// because a silently-substituted deck would change the line under test.
struct DeckBook {
    /// lowercased name -> deck entry.
    by_name: BTreeMap<String, DeckEntry>,
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
        let mut by_name: BTreeMap<String, DeckEntry> = BTreeMap::new();

        if let Some(path) = decks {
            let pool = pool::load_pool(path)?;
            for d in pool.decks {
                by_name.insert(
                    d.name.to_lowercase(),
                    DeckEntry {
                        main: d.cards.clone(),
                        eggs: d.eggs.clone(),
                    },
                );
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
            let entry = DeckEntry {
                main: d.main_deck.clone(),
                eggs: d.egg_deck.clone(),
            };
            // Registered under every name a scenario might reasonably use.
            for alias in [&d.id, &d.name, &d.set] {
                if !alias.is_empty() {
                    by_name.insert(alias.to_lowercase(), entry.clone());
                }
            }
        }
        Ok(DeckBook {
            by_name,
            source: path.display().to_string(),
        })
    }

    fn resolve(&self, rest: &str) -> Result<&DeckEntry, String> {
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
    let entry = book.resolve(&seat.rest)?;
    let mut remainder = entry.main.clone();
    remainder.extend(entry.eggs.iter().cloned());
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

// --- scripted-job emission (`--emit-job`) -----------------------------------

/// A DCGO scripted harness job, field-for-field the shape the modded client's
/// scripted driver reads (see `qa/dcgo-harness/golden-scripted-job.json`).
/// Same core fields as `dcgo_harness::job::JobSpec` plus the scripted-only
/// `deck_order` / `inputs`; phase-1 readers tolerate the extras.
#[derive(Debug, Serialize)]
struct ExamJobSpec {
    job_id: String,
    policy: String,
    decks: dcgo_harness::job::JobDecks,
    deck_order: ExamDeckOrder,
    inputs: Vec<ScriptedInput>,
    first_player: u8,
    seed: u64,
    limits: JobLimits,
}

/// The scenario's stack per seat, in DCGO's TOP-FIRST convention — the order
/// the author wrote it in.
#[derive(Debug, Serialize)]
struct ExamDeckOrder {
    p0: Vec<String>,
    p1: Vec<String>,
}

/// One scripted answer: which seat, which lowered action id, and which prompt
/// it expects to be answering (asserted BEFORE answering — see the exam doc).
/// Our prompt vocabulary maps 1:1 onto DCGO's, so the name passes through.
///
/// A `select:` step carries no action id — it answers a selection RPC, not a
/// main-phase/breeding prompt — and instead fills the `select_*` fields, whose
/// names are the C# `HarnessJobStep` contract verbatim (`select_card_ids` /
/// `select_value` / `select_has_bool` + `select_bool` / `select_cancel`; the
/// C# side treats an absent `select_value` as `int.MinValue` via its field
/// initializer, so absence is expressed by OMITTING the key). The values are
/// the scenario's SYMBOLIC identities (card ids, counts, bools), never engine
/// action ids: DCGO resolves them against its own candidate lists.
#[derive(Debug, Default, Serialize)]
struct ScriptedInput {
    actor: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    action_id: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expect_prompt: Option<String>,
    /// Expected number of picks. The C# initializer is `-1` = "do not
    /// assert", so absence is expressed by OMITTING the key.
    #[serde(skip_serializing_if = "Option::is_none")]
    expect_count: Option<u16>,
    /// Expected candidate card ids, order-insensitive. Empty = "do not
    /// assert". On a `MultipleSkills` row these are the stacked triggers'
    /// SOURCE-CARD ids, which is the cheapest way to catch "DCGO did not stack
    /// what we stacked".
    #[serde(skip_serializing_if = "Vec::is_empty")]
    expect_candidates: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    select_card_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    select_value: Option<i32>,
    /// Which of the SAME identity's candidates to take, 0-based — DCGO's
    /// `select_ordinal`. Deliberately NOT `select_value` reused: that field
    /// stays the raw DCGO-index fallback, and one field meaning "an index into
    /// DCGO's list" in one step and "an index within one card's own triggers"
    /// in the next is the value-space confusion the payload exists to end.
    /// Same `int.MinValue` absent sentinel, so absence OMITS the key.
    #[serde(skip_serializing_if = "Option::is_none")]
    select_ordinal: Option<i32>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    select_has_bool: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    select_bool: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    select_cancel: bool,
}

/// One seat's flat job deck in DCGO's top-first convention: full main deck
/// (stack first, then the remainder), with the egg deck appended.
///
/// This inverts `decks_from_recording`'s reversal: our engine's deck vector is
/// draw-from-back (`ordered_deck` builds `remainder + stack.rev()`), while a
/// DCGO job lists the deck top-first. Reversing the main portion gives
/// `stack + remainder.rev()` — `stack[0]` on top, exactly what `deck_order`
/// then re-imposes after DCGO's own seeded shuffle.
fn job_deck_top_first(
    seat: &dcgo_harness::exam::scenario::ScenarioSeat,
    entry: &DeckEntry,
    deck_name: &str,
) -> Result<Vec<String>, String> {
    let mut remainder = entry.main.clone();
    for id in &seat.stack {
        match remainder.iter().position(|c| c == id) {
            Some(i) => {
                remainder.remove(i);
            }
            None => {
                return Err(format!(
                    "stacked card {id} is not in the main deck of `{deck_name}` -- \
                     the job would claim a deck the scenario does not use"
                ))
            }
        }
    }
    let mut out: Vec<String> = seat.stack.clone();
    out.extend(remainder.into_iter().rev());
    out.extend(entry.eggs.iter().cloned());
    Ok(out)
}

/// Build the DCGO scripted job for a lowered scenario.
///
/// Refuses a seat whose deck resolves with no egg cards: `Game::new` would
/// play on regardless, but DCGO's breeding phase would be answering over a
/// different game than the one the line was lowered against — the job must
/// say so instead of silently emitting.
fn build_exam_job(
    stem: &str,
    s: &dcgo_harness::exam::scenario::Scenario,
    entry_p0: &DeckEntry,
    entry_p1: &DeckEntry,
    lowered: &[dcgo_harness::exam::adapter::LoweredStep],
) -> Result<ExamJobSpec, String> {
    if lowered.len() != s.steps.len() {
        return Err(format!(
            "lowered {} action id(s) for a {}-step line -- refusing to emit a \
             desynchronized job",
            lowered.len(),
            s.steps.len()
        ));
    }
    for (seat_name, seat, entry) in [
        ("p0", &s.decks.p0, entry_p0),
        ("p1", &s.decks.p1, entry_p1),
    ] {
        if entry.eggs.is_empty() {
            return Err(format!(
                "deck `{}` ({seat_name}) has no egg cards resolvable from the deck \
                 book -- refusing to emit a job with no eggs, which would run a \
                 different game than the scenario lowered against",
                seat.rest
            ));
        }
    }

    let inputs = s
        .steps
        .iter()
        .zip(lowered)
        .flat_map(|(step, l)| {
            use dcgo_harness::exam::adapter::{EotAttackTarget, LoweredStep};
            let expect_prompt = step.expect.as_ref().and_then(|e| e.prompt.clone());
            // `expect.count` / `expect.candidates` describe the PICK, so on a
            // step the emitter splits they ride the pick row, never the
            // OptionalSkill gate that precedes it.
            let expect_count = step.expect.as_ref().and_then(|e| e.count);
            let expect_candidates = step
                .expect
                .as_ref()
                .map(|e| e.candidates.clone())
                .unwrap_or_default();
            match l {
                LoweredStep::Action(id) => vec![ScriptedInput {
                    actor: step.actor,
                    action_id: Some(*id),
                    expect_prompt,
                    expect_count,
                    expect_candidates,
                    ..ScriptedInput::default()
                }],
                // task_69f10a66 (ruling item 5) — the OptionalSkill+pick
                // FOLD: DCGO gates some optional keyword windows (<Raid>,
                // …) behind an OptionalSkill yes/no BEFORE the pick, while
                // our engine surfaces one declinable pick. A folded row
                // (authored `expect: {prompt: OptionalSkill}` over the live
                // pick) splits on the wire:
                //   picks   -> OptionalSkill(yes) + the pick row
                //   decline -> OptionalSkill(no) only (DCGO never opens the
                //              pick after a declined gate).
                LoweredStep::Select(w) if w.optional_gate_fold => {
                    if w.cancel {
                        vec![ScriptedInput {
                            actor: step.actor,
                            action_id: None,
                            expect_prompt: Some("OptionalSkill".to_string()),
                            select_has_bool: true,
                            ..ScriptedInput::default()
                        }]
                    } else {
                        vec![
                            ScriptedInput {
                                actor: step.actor,
                                action_id: None,
                                expect_prompt: Some("OptionalSkill".to_string()),
                                select_has_bool: true,
                                select_bool: true,
                                ..ScriptedInput::default()
                            },
                            ScriptedInput {
                                actor: step.actor,
                                action_id: None,
                                // The pick's own DCGO prompt class varies
                                // (SelectPermanentEffect / SelectHandEffect /
                                // SelectCardEffect …) — leave it unasserted
                                // rather than guess wrong.
                                expect_prompt: None,
                                expect_count,
                                expect_candidates,
                                select_card_ids: w.card_ids.clone(),
                                select_value: w.value,
                                select_ordinal: w.ordinal,
                                ..ScriptedInput::default()
                            },
                        ]
                    }
                }
                // A DCGO-ONLY row emits exactly like a plain select — DCGO
                // cannot tell the difference, and must not: it resolves the
                // identities against its own candidate list as usual. The
                // asymmetry is entirely on our side, where the step consumes
                // no live prompt.
                LoweredStep::Select(w) | LoweredStep::DcgoOnlySelect(w) => vec![ScriptedInput {
                    actor: step.actor,
                    action_id: None,
                    expect_prompt,
                    expect_count,
                    expect_candidates,
                    select_card_ids: w.card_ids.clone(),
                    select_value: w.value,
                    select_ordinal: w.ordinal,
                    select_has_bool: w.bool_answer.is_some(),
                    select_bool: w.bool_answer.unwrap_or(false),
                    select_cancel: w.cancel,
                }],
                // task_69f10a66 Family 1 surface mapping: our EndOfTurnAction
                // phase park (the §16-37-3 "may attack at end of turn" for
                // printed/granted <Execute>, <Engage>, Vortex, MayAttack) is
                // DCGO's OptionalSkill gate (+ SelectAttackEffect on yes).
                //   PASS   -> one row: OptionalSkill answered "no".
                //   attack -> two rows: OptionalSkill "yes", then the
                //             SelectAttackEffect target pick (permanent
                //             targets by top-card id, the player as
                //             select_value -1 — the SelectAttackEffect.cs
                //             harness contract).
                // A sim-side-only action (e.g. the PASS that exits our
                // EndOfTurnAction park after the last gate was spent) —
                // DCGO has no prompt for it, so it contributes NO wire row.
                LoweredStep::SimOnlyAction(_) => Vec::new(),
                // A follow-on material pick: ours only. The FIRST pick's row
                // already carries every declared id for DCGO.
                LoweredStep::SimOnlySelect => vec![],
                LoweredStep::EndOfTurnGate { attack, .. } => {
                    let gate_prompt =
                        Some(expect_prompt.unwrap_or_else(|| "OptionalSkill".to_string()));
                    match attack {
                        None => vec![ScriptedInput {
                            actor: step.actor,
                            action_id: None,
                            expect_prompt: gate_prompt,
                            select_has_bool: true,
                            ..ScriptedInput::default()
                        }],
                        Some(target) => {
                            let (ids, value) = match target {
                                EotAttackTarget::Player => (Vec::new(), Some(-1)),
                                EotAttackTarget::Permanent { top_card_id } => {
                                    (vec![top_card_id.clone()], None)
                                }
                            };
                            vec![
                                ScriptedInput {
                                    actor: step.actor,
                                    action_id: None,
                                    expect_prompt: gate_prompt,
                                    select_has_bool: true,
                                    select_bool: true,
                                    ..ScriptedInput::default()
                                },
                                ScriptedInput {
                                    actor: step.actor,
                                    action_id: None,
                                    expect_prompt: Some("SelectAttackEffect".to_string()),
                                    expect_count,
                                    expect_candidates,
                                    select_card_ids: ids,
                                    select_value: value,
                                    ..ScriptedInput::default()
                                },
                            ]
                        }
                    }
                }
            }
        })
        .collect();

    Ok(ExamJobSpec {
        job_id: format!("exam-{stem}"),
        policy: "scripted".to_string(),
        decks: dcgo_harness::job::JobDecks {
            p0: job_deck_top_first(&s.decks.p0, entry_p0, &s.decks.p0.rest)?,
            p1: job_deck_top_first(&s.decks.p1, entry_p1, &s.decks.p1.rest)?,
        },
        deck_order: ExamDeckOrder {
            p0: s.decks.p0.stack.clone(),
            p1: s.decks.p1.stack.clone(),
        },
        inputs,
        // The adapter lowers every scenario with seat 0 acting first
        // (`SCENARIO_FIRST_PLAYER`), and DCGO honours the job's first_player.
        first_player: 0,
        seed: s.seed,
        limits: JobLimits {
            max_turns: 40,
            timeout_seconds: 180,
        },
    })
}

#[cfg(test)]
mod emit_job_tests {
    use super::*;
    use dcgo_harness::exam::adapter::{EotAttackTarget, LoweredStep, SelectWire};
    use dcgo_harness::exam::scenario::Scenario;

    /// Shorthand: a lowered line of plain action ids.
    fn actions(ids: &[u16]) -> Vec<LoweredStep> {
        ids.iter().map(|id| LoweredStep::Action(*id)).collect()
    }

    const LINE: &str = r#"
card: ST1-12
clause: ST1-12#effect#0
seed: 424242
decks:
  p0: { stack: [B, D], rest: tiny }
  p1: { stack: [], rest: tiny }
steps:
  - actor: 0
    do: { pass: {} }
    expect: { prompt: breeding_action }
  - actor: 0
    do: { pass: {} }
    expect: { prompt: main_phase }
  - actor: 1
    do: { pass: {} }
"#;

    fn entry() -> DeckEntry {
        DeckEntry {
            main: vec!["A", "B", "C", "D"].into_iter().map(String::from).collect(),
            eggs: vec!["E1".to_string()],
        }
    }

    #[test]
    fn deck_is_reversed_back_to_top_first_with_eggs_appended() {
        // Our draw-from-back vector for this seat would be
        // [A, C, D, B] (remainder [A, C] + stack [B, D] reversed), so the
        // top-first job deck must be [B, D, C, A] -- stack first, remainder
        // reversed -- with the egg deck appended after the main deck.
        let s = Scenario::from_yaml(LINE).unwrap();
        let job = build_exam_job("ST1-12", &s, &entry(), &entry(), &actions(&[62, 62, 62])).unwrap();
        assert_eq!(job.decks.p0, vec!["B", "D", "C", "A", "E1"]);
        // No stack: the whole main deck reversed, then eggs.
        assert_eq!(job.decks.p1, vec!["D", "C", "B", "A", "E1"]);
        assert_eq!(job.deck_order.p0, vec!["B", "D"]);
        assert!(job.deck_order.p1.is_empty());
    }

    #[test]
    fn inputs_carry_actor_action_id_and_expect_prompt() {
        let s = Scenario::from_yaml(LINE).unwrap();
        let job = build_exam_job("ST1-12", &s, &entry(), &entry(), &actions(&[62, 63, 64])).unwrap();
        assert_eq!(job.inputs.len(), 3);
        assert_eq!(job.inputs[0].actor, 0);
        assert_eq!(job.inputs[0].action_id, Some(62));
        assert_eq!(job.inputs[0].expect_prompt.as_deref(), Some("breeding_action"));
        assert_eq!(job.inputs[1].expect_prompt.as_deref(), Some("main_phase"));
        assert_eq!(job.inputs[2].actor, 1);
        assert_eq!(job.inputs[2].expect_prompt, None);
    }

    #[test]
    fn job_identity_fields_come_from_the_scenario() {
        let s = Scenario::from_yaml(LINE).unwrap();
        let job = build_exam_job("ST1-12", &s, &entry(), &entry(), &actions(&[62, 62, 62])).unwrap();
        assert_eq!(job.job_id, "exam-ST1-12");
        assert_eq!(job.policy, "scripted");
        assert_eq!(job.seed, 424242);
        assert_eq!(job.first_player, 0);
    }

    #[test]
    fn a_deck_with_no_resolvable_eggs_is_refused() {
        let s = Scenario::from_yaml(LINE).unwrap();
        let eggless = DeckEntry {
            main: entry().main,
            eggs: vec![],
        };
        let err = build_exam_job("ST1-12", &s, &eggless, &entry(), &actions(&[62, 62, 62])).unwrap_err();
        assert!(err.contains("no egg cards"), "got: {err}");
        assert!(err.contains("tiny"), "must name the deck: {err}");
    }

    #[test]
    fn a_desynchronized_lowering_is_refused() {
        // 3 steps but only 2 lowered ids: emitting would hand DCGO a line that
        // answers the wrong prompts from the first mismatch onward.
        let s = Scenario::from_yaml(LINE).unwrap();
        let err = build_exam_job("ST1-12", &s, &entry(), &entry(), &actions(&[62, 62])).unwrap_err();
        assert!(err.contains("desynchronized"), "got: {err}");
    }

    #[test]
    fn serialized_job_matches_the_golden_field_names() {
        // The DCGO reader is the consumer; these exact key names are the
        // contract (see qa/dcgo-harness/golden-scripted-job.json).
        let s = Scenario::from_yaml(LINE).unwrap();
        let job = build_exam_job("ST1-12", &s, &entry(), &entry(), &actions(&[62, 62, 62])).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&job).unwrap()).unwrap();
        for key in [
            "job_id",
            "policy",
            "decks",
            "deck_order",
            "inputs",
            "first_player",
            "seed",
            "limits",
        ] {
            assert!(v.get(key).is_some(), "missing top-level key {key}");
        }
        assert_eq!(v["inputs"][0]["expect_prompt"], "breeding_action");
        assert_eq!(v["inputs"][0]["action_id"], 62);
        assert_eq!(v["limits"]["max_turns"], 40);
        // A step without `expect:` must OMIT the key, not write null -- the
        // driver treats presence as "assert this prompt".
        assert!(v["inputs"][2].get("expect_prompt").is_none());
        // A non-select step must not wear ANY selection field: on the C# side
        // `IsSelection` keys off their presence, and a stray one would turn an
        // action step into a selection answer.
        for key in [
            "select_card_ids",
            "select_value",
            "select_has_bool",
            "select_bool",
            "select_cancel",
        ] {
            assert!(
                v["inputs"][0].get(key).is_none(),
                "action step must omit {key}"
            );
        }
    }

    // ── select steps on the wire ────────────────────────────────────────
    //
    // The field names below are the C# `HarnessJobStep` contract verbatim
    // (read from `Assets/Scripts/Script/Harness/HarnessJob.cs`, DCGO
    // 9bbc7e5f3): `select_card_ids` / `select_value` (absent = int.MinValue
    // via the field initializer) / `select_has_bool` + `select_bool` /
    // `select_cancel`. The values are symbolic identities from the scenario,
    // never engine action ids.

    /// LINE with its final pass replaced by a select step carrying `args`.
    fn select_line(args: &str) -> Scenario {
        let text = LINE.replace(
            "  - actor: 1\n    do: { pass: {} }",
            &format!("  - actor: 1\n    do: {{ select: {args} }}"),
        );
        Scenario::from_yaml(&text).expect("select line parses")
    }

    fn job_json(s: &Scenario, lowered: &[LoweredStep]) -> serde_json::Value {
        let job = build_exam_job("ST1-12", s, &entry(), &entry(), lowered).unwrap();
        serde_json::from_str(&serde_json::to_string(&job).unwrap()).unwrap()
    }

    #[test]
    fn select_card_ids_ride_the_wire_and_action_id_is_omitted() {
        let s = select_line("{ cards: [ST1-03, ST1-03] }");
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::Select(SelectWire {
            card_ids: vec!["ST1-03".to_string(), "ST1-03".to_string()],
            ..SelectWire::default()
        }));
        let v = job_json(&s, &lowered);
        assert_eq!(
            v["inputs"][2]["select_card_ids"],
            serde_json::json!(["ST1-03", "ST1-03"])
        );
        // A selection step answers a selection RPC, not a 2192-space prompt;
        // an action id here would be an engine-internal leak.
        assert!(v["inputs"][2].get("action_id").is_none());
        assert_eq!(v["inputs"][2]["actor"], 1);
    }

    #[test]
    fn select_value_and_bool_and_cancel_use_the_csharp_field_names() {
        let s = select_line("{ value: 3 }");
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::Select(SelectWire {
            value: Some(3),
            ..SelectWire::default()
        }));
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"][2]["select_value"], 3);
        assert!(v["inputs"][2].get("select_card_ids").is_none());
        assert!(v["inputs"][2].get("select_has_bool").is_none());

        let s = select_line("{ yes: true }");
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::Select(SelectWire {
            bool_answer: Some(true),
            ..SelectWire::default()
        }));
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"][2]["select_has_bool"], true);
        assert_eq!(v["inputs"][2]["select_bool"], true);
        // Absent select_value must be OMITTED (the C# initializer, not 0, is
        // the absent sentinel -- writing 0 would claim a count answer of 0).
        assert!(v["inputs"][2].get("select_value").is_none());

        let s = select_line("{ decline: true }");
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::Select(SelectWire {
            cancel: true,
            ..SelectWire::default()
        }));
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"][2]["select_cancel"], true);
        assert!(v["inputs"][2].get("select_bool").is_none());
    }

    // ── select_ordinal + the expect_* assertions on the wire ────────────

    #[test]
    fn select_ordinal_rides_the_wire_under_the_csharp_field_name() {
        let s = select_line("{ cards: [EX12-047], ordinal: 1 }");
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::Select(SelectWire {
            card_ids: vec!["EX12-047".to_string()],
            ordinal: Some(1),
            ..SelectWire::default()
        }));
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"][2]["select_card_ids"], serde_json::json!(["EX12-047"]));
        assert_eq!(v["inputs"][2]["select_ordinal"], 1);
        // `select_value` stays the raw DCGO-index fallback and must NOT be
        // written: the C# hook aborts on value+card_ids by design.
        assert!(v["inputs"][2].get("select_value").is_none());
    }

    #[test]
    fn ordinal_zero_is_written_and_an_absent_ordinal_is_omitted() {
        // 0 is a real answer ("the FIRST of that card's triggers"), and the
        // C# absent sentinel is int.MinValue -- so 0 must serialize while
        // absent must omit the key entirely.
        let s = select_line("{ cards: [EX12-047], ordinal: 0 }");
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::Select(SelectWire {
            card_ids: vec!["EX12-047".to_string()],
            ordinal: Some(0),
            ..SelectWire::default()
        }));
        assert_eq!(job_json(&s, &lowered)["inputs"][2]["select_ordinal"], 0);

        let s = select_line("{ cards: [EX12-047] }");
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::Select(SelectWire {
            card_ids: vec!["EX12-047".to_string()],
            ..SelectWire::default()
        }));
        assert!(job_json(&s, &lowered)["inputs"][2]
            .get("select_ordinal")
            .is_none());
    }

    #[test]
    fn an_action_step_wears_no_ordinal() {
        // On the C# side `IsSelection` keys off the selection fields'
        // presence, and `select_ordinal` is one of them -- a stray one would
        // turn an action step into a selection answer.
        let s = Scenario::from_yaml(LINE).unwrap();
        let v: serde_json::Value = serde_json::from_str(
            &serde_json::to_string(
                &build_exam_job("ST1-12", &s, &entry(), &entry(), &actions(&[62, 62, 62])).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(v["inputs"][0].get("select_ordinal").is_none());
    }

    #[test]
    fn expect_count_and_expect_candidates_ride_the_wire() {
        // Previously parsed and then DROPPED. On a MultipleSkills row the
        // candidate set is the stacked TRIGGERS' source-card ids, which is the
        // cheapest available check that DCGO stacked what we stacked.
        let text = LINE.replace(
            "  - actor: 1\n    do: { pass: {} }",
            "  - actor: 1\n    do: { select: { cards: [EX12-047], ordinal: 1 } }\n    expect: { prompt: MultipleSkills, count: 1, candidates: [EX12-047, EX12-047] }",
        );
        let s = Scenario::from_yaml(&text).expect("parses");
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::Select(SelectWire {
            card_ids: vec!["EX12-047".to_string()],
            ordinal: Some(1),
            ..SelectWire::default()
        }));
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"][2]["expect_prompt"], "MultipleSkills");
        assert_eq!(v["inputs"][2]["expect_count"], 1);
        assert_eq!(
            v["inputs"][2]["expect_candidates"],
            serde_json::json!(["EX12-047", "EX12-047"])
        );
    }

    #[test]
    fn absent_expectations_omit_their_keys() {
        // The C# initializers (`expect_count = -1`, `expect_candidates =
        // new string[0]`) ARE the "do not assert" defaults, so writing 0 / []
        // would turn "no opinion" into a live assertion.
        let s = Scenario::from_yaml(LINE).unwrap();
        let job = build_exam_job("ST1-12", &s, &entry(), &entry(), &actions(&[62, 62, 62])).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&job).unwrap()).unwrap();
        assert!(v["inputs"][0].get("expect_count").is_none());
        assert!(v["inputs"][0].get("expect_candidates").is_none());
    }

    #[test]
    fn a_folded_pick_carries_the_expectations_on_the_pick_row_not_the_gate() {
        // `expect.count` / `expect.candidates` describe the PICK. Putting them
        // on the OptionalSkill gate would assert a candidate list against a
        // yes/no prompt that has none.
        let text = LINE.replace(
            "  - actor: 1\n    do: { pass: {} }",
            "  - actor: 1\n    do: { select: { targets: [opp.field.0] } }\n    expect: { prompt: OptionalSkill, count: 1, candidates: [ST1-07] }",
        );
        let s = Scenario::from_yaml(&text).expect("parses");
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::Select(SelectWire {
            card_ids: vec!["ST1-07".to_string()],
            optional_gate_fold: true,
            ..SelectWire::default()
        }));
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"].as_array().unwrap().len(), 4);
        assert_eq!(v["inputs"][2]["expect_prompt"], "OptionalSkill");
        assert!(v["inputs"][2].get("expect_count").is_none(), "gate row must not assert the pick");
        assert!(v["inputs"][2].get("expect_candidates").is_none());
        assert_eq!(v["inputs"][3]["expect_count"], 1);
        assert_eq!(v["inputs"][3]["expect_candidates"], serde_json::json!(["ST1-07"]));
    }

    // ── task_69f10a66 surface mappings on the wire ──────────────────────

    /// The end-of-turn attack-keyword gate (our `EndOfTurnAction` park; DCGO
    /// OptionalSkill + SelectAttackEffect). PASS = one OptionalSkill "no"
    /// row; an attack = OptionalSkill "yes" + the SelectAttackEffect answer
    /// (permanent by top-card id / the player as select_value -1); a
    /// sim-only phase-exit pass = NO wire row at all.
    #[test]
    fn end_of_turn_gate_maps_to_optional_skill_rows() {
        // Decline: one OptionalSkill(no) row.
        let s = select_line("{ cards: [ST1-03] }"); // 3-step line; payloads below drive the shape
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::EndOfTurnGate {
            action_id: 62,
            attack: None,
        });
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"][2]["expect_prompt"], "OptionalSkill");
        assert_eq!(v["inputs"][2]["select_has_bool"], true);
        assert!(v["inputs"][2].get("select_bool").is_none()); // false is skip-serialized
        assert!(v["inputs"][2].get("action_id").is_none());

        // Accept + attack a permanent: OptionalSkill(yes) then the pick.
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::EndOfTurnGate {
            action_id: 100,
            attack: Some(EotAttackTarget::Permanent {
                top_card_id: "ST1-07".to_string(),
            }),
        });
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"].as_array().unwrap().len(), 4, "one step -> two rows");
        assert_eq!(v["inputs"][2]["expect_prompt"], "OptionalSkill");
        assert_eq!(v["inputs"][2]["select_bool"], true);
        assert_eq!(v["inputs"][3]["expect_prompt"], "SelectAttackEffect");
        assert_eq!(v["inputs"][3]["select_card_ids"], serde_json::json!(["ST1-07"]));

        // Accept + attack the player: select_value -1 on the pick row.
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::EndOfTurnGate {
            action_id: 113,
            attack: Some(EotAttackTarget::Player),
        });
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"][3]["select_value"], -1);

        // Sim-only phase exit: contributes NOTHING to the wire.
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::SimOnlyAction(62));
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"].as_array().unwrap().len(), 2, "no third row");
    }

    /// The differ pairs the two traces by LOWERED STEP, using
    /// `LoweredStep::dcgo_wire_rows()` to know how many rows each step
    /// consumes. That number is only trustworthy if it agrees with what this
    /// emitter actually writes -- if they ever drift, the pairing slides and
    /// manufactures divergences out of an offset, which is precisely the bug
    /// the pairing exists to fix. So: pin them against each other, per
    /// variant, over the real emitter.
    #[test]
    fn wire_row_counts_match_dcgo_wire_rows() {
        let s = select_line("{ cards: [ST1-03] }");
        let variants: Vec<LoweredStep> = vec![
            LoweredStep::Action(62),
            LoweredStep::SimOnlyAction(62),
            // The mirror of SimOnlyAction: one wire row, no sim-side row.
            LoweredStep::DcgoOnlySelect(SelectWire {
                card_ids: vec!["ST1-03".to_string()],
                ordinal: Some(1),
                ..SelectWire::default()
            }),
            LoweredStep::Select(SelectWire {
                card_ids: vec!["ST1-03".to_string()],
                ..SelectWire::default()
            }),
            LoweredStep::Select(SelectWire {
                value: Some(3),
                ..SelectWire::default()
            }),
            LoweredStep::Select(SelectWire {
                bool_answer: Some(true),
                ..SelectWire::default()
            }),
            LoweredStep::Select(SelectWire {
                cancel: true,
                ..SelectWire::default()
            }),
            LoweredStep::Select(SelectWire {
                card_ids: vec!["ST1-03".to_string()],
                optional_gate_fold: true,
                ..SelectWire::default()
            }),
            LoweredStep::Select(SelectWire {
                cancel: true,
                optional_gate_fold: true,
                ..SelectWire::default()
            }),
            LoweredStep::EndOfTurnGate {
                action_id: 62,
                attack: None,
            },
            LoweredStep::EndOfTurnGate {
                action_id: 100,
                attack: Some(EotAttackTarget::Player),
            },
            LoweredStep::EndOfTurnGate {
                action_id: 100,
                attack: Some(EotAttackTarget::Permanent {
                    top_card_id: "ST1-07".to_string(),
                }),
            },
        ];
        for v in variants {
            // Two known-1-row steps plus the variant under test, so the
            // baseline is fixed and the delta is the variant's own count.
            let mut lowered = actions(&[62, 62]);
            lowered.push(v.clone());
            let job = build_exam_job("ST1-12", &s, &entry(), &entry(), &lowered).unwrap();
            assert_eq!(
                job.inputs.len() - 2,
                v.dcgo_wire_rows(),
                "{v:?} claims {} wire row(s) but the emitter wrote {}",
                v.dcgo_wire_rows(),
                job.inputs.len() - 2
            );
        }
    }

    /// The OptionalSkill+pick FOLD (ruling item 5, `<Raid>`-family): a
    /// folded pick splits into OptionalSkill(yes) + the pick row; a folded
    /// decline emits ONLY OptionalSkill(no).
    #[test]
    fn optional_gate_fold_splits_the_wire_rows() {
        let s = select_line("{ targets: [opp.field.0] }");
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::Select(SelectWire {
            card_ids: vec!["ST1-07".to_string()],
            optional_gate_fold: true,
            ..SelectWire::default()
        }));
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"].as_array().unwrap().len(), 4, "one step -> two rows");
        assert_eq!(v["inputs"][2]["expect_prompt"], "OptionalSkill");
        assert_eq!(v["inputs"][2]["select_bool"], true);
        assert!(v["inputs"][3].get("expect_prompt").is_none());
        assert_eq!(v["inputs"][3]["select_card_ids"], serde_json::json!(["ST1-07"]));

        let s = select_line("{ decline: true }");
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::Select(SelectWire {
            cancel: true,
            optional_gate_fold: true,
            ..SelectWire::default()
        }));
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"].as_array().unwrap().len(), 3, "decline folds to one row");
        assert_eq!(v["inputs"][2]["expect_prompt"], "OptionalSkill");
        assert_eq!(v["inputs"][2]["select_has_bool"], true);
        assert!(v["inputs"][2].get("select_bool").is_none()); // "no" skip-serializes
        assert!(v["inputs"][2].get("select_cancel").is_none());
    }
}
