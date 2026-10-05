//! `dcgo-harness` — submit DCGO harness jobs, report queue status, triage the
//! resulting corpus.
//!
//! Exit codes:
//!   0 — command succeeded.
//!   1 — command ran but reported failures (e.g. triage found divergences).
//!   2 — argument or I/O error.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use dcgo_harness::job::{JobLimits, DIR_CLAIMED, DIR_DONE, DIR_FAILED, DIR_JOBS};

/// Marker file whose presence tells DCGO the harness is on. Must match
/// HarnessConfig.EnabledMarkerPath on the C# side.
const MARKER_FILE: &str = "harness.enabled";
use dcgo_harness::pool;

#[derive(Parser, Debug)]
#[command(about = "Drive unattended DCGO games from a filesystem job queue.")]
struct Args {
    /// Harness root: the directory holding jobs/ claimed/ done/ failed/.
    ///
    /// OPTIONAL, because not every subcommand drives DCGO. `exam --sim-only`
    /// replays scenarios in our engine alone -- no jobs, no queue, no Unity --
    /// so demanding a root there is a pure obstacle. It was a required arg
    /// until now, which is why `.github/workflows/dcgo-exam-sim.yml` had never
    /// once run: clap rejected the invocation before a single scenario was
    /// read, and the failure looked like a config problem rather than a dead
    /// gate. Subcommands that DO need a root resolve it through
    /// [`Args::require_root`], which names the flag in its error.
    #[arg(long)]
    root: Option<PathBuf>,

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
            default_missing_value = "qa/qa-reports/exam-verdicts"
        )]
        verdicts: Option<PathBuf>,
        /// `clause_coverage extract` output supplying each clause's label and
        /// text (hashed into the verdict as its drift fingerprint). A verdict
        /// is REFUSED for any clause id absent from this file.
        #[arg(long)]
        clause_text_json: Option<PathBuf>,
        /// The interaction denominator (`python -m tools.card_loop interactions
        /// build`). Read only when --verdicts records an INTERACTION scenario;
        /// its verdict is REFUSED for an id absent from it, or when it was
        /// never generated.
        #[arg(long, default_value = dcgo_harness::exam::verdict::DEFAULT_INTERACTION_DENOMINATOR)]
        interaction_denominator: PathBuf,
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
        /// Print, per scenario, which card raised each select step's prompt
        /// (and our engine's prompt kind/text) -- the card whose DCGO script
        /// decides the prompt's shape on the oracle side.
        #[arg(long)]
        explain_selects: bool,
        /// On a CLEAN oracle diff, write the confirmed state into the scenario's
        /// `assert:` block (rows marked `_backfilled`). Oracle mode only.
        #[arg(long)]
        backfill: bool,
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
    /// Split a single-blob verdict store into per-card files (one-time).
    MigrateVerdicts {
        /// The existing single-file store.
        #[arg(long, default_value = "qa/qa-reports/dcgo_exam_verdicts.json")]
        from: PathBuf,
        /// Destination directory for per-card files.
        #[arg(long, default_value = "qa/qa-reports/exam-verdicts")]
        to: PathBuf,
    },
    /// Classify a diverged exam verdict: whose bug is it?
    VerdictTriage {
        #[arg(long)]
        clause: String,
        /// ours_wrong | dcgo_quirk | undetermined
        #[arg(long)]
        triage: String,
        #[arg(long)]
        citation: Option<String>,
        #[arg(long, default_value = "qa/qa-reports/exam-verdicts")]
        verdicts: PathBuf,
    },
    /// Serve the exam's agent surface over stdio (MCP, JSON-RPC 2.0).
    Mcp,
    /// Bring this machine up as an oracle node: preflight, then launch.
    Node {
        #[command(subcommand)]
        action: NodeAction,
    },
}

#[derive(Subcommand, Debug)]
enum NodeAction {
    /// Preflight and start the oracle.
    Up {
        #[arg(long)]
        build: PathBuf,
    },
    /// Stop the oracle.
    Down,
    /// Report readiness without changing anything.
    Status {
        #[arg(long)]
        build: Option<PathBuf>,
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

impl Args {
    /// The harness root, for the subcommands that actually queue DCGO work.
    fn require_root(&self) -> Result<&std::path::Path, String> {
        self.root.as_deref().ok_or_else(|| {
            "this subcommand drives the DCGO job queue and needs --root <DIR>              (the directory holding jobs/ claimed/ done/ failed/). Only              `exam`, `migrate-verdicts`, and `mcp` run without one."
                .to_string()
        })
    }
}

/// Whether this subcommand touches the job queue at all.
///
/// `exam` does not: it lowers scenarios and replays them in our engine, and
/// with `--sim-only` never involves DCGO. `migrate-verdicts` does not either:
/// it is a pure file-to-file conversion of the verdict store, unrelated to
/// the DCGO job queue. `mcp` does not either: most of its tools (validate,
/// authoring guide, keyword brief, scenario probing) run entirely against
/// files and the engine, not the job queue -- the few that do need a root
/// (e.g. `exam_probe` with `sim_only: false`) resolve it themselves and say
/// so in their error. Creating jobs/ claimed/ done/ failed/ for any of these
/// would litter the CI workspace with empty directories at best, and at worst
/// require a root the run has no use for.
fn needs_root(command: &Command) -> bool {
    !matches!(
        command,
        Command::Exam { .. }
            | Command::MigrateVerdicts { .. }
            | Command::VerdictTriage { .. }
            | Command::Mcp
    )
}

/// Whether `to` looks like the destination of a migration that already ran:
/// a directory that exists and already holds at least one `*.json` file.
///
/// A non-existent or empty directory is fine to migrate into. Anything else
/// -- including a directory that holds unrelated `*.json` files -- is
/// treated as "already migrated" on purpose: `migrate-verdicts` is a
/// one-shot conversion, and the caller (`Command::MigrateVerdicts`) refuses
/// rather than let `save_dir`'s prune step delete files it doesn't
/// recognize.
fn migrate_verdicts_destination_already_populated(to: &Path) -> Result<bool, String> {
    if !to.is_dir() {
        return Ok(false);
    }
    let entries = std::fs::read_dir(to)
        .map_err(|e| format!("failed to read {}: {e}", to.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read {}: {e}", to.display()))?;
        if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
            return Ok(true);
        }
    }
    Ok(false)
}

fn run(args: &Args) -> Result<ExitCode, String> {
    if needs_root(&args.command) {
        let root = args.require_root()?;
        for dir in [DIR_JOBS, DIR_CLAIMED, DIR_DONE, DIR_FAILED] {
            let path = root.join(dir);
            std::fs::create_dir_all(&path)
                .map_err(|e| format!("creating {}: {}", path.display(), e))?;
        }
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
            let jobs_dir = args.require_root()?.join(DIR_JOBS);
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
            let marker = args.require_root()?.join(MARKER_FILE);
            std::fs::write(&marker, "enabled by dcgo-harness
")
                .map_err(|e| format!("writing {}: {}", marker.display(), e))?;
            println!("harness ENABLED ({})", marker.display());
            println!("DCGO will claim jobs on its next Play. Run 'disable' when done.");
            Ok(ExitCode::SUCCESS)
        }
        Command::Disable => {
            let marker = args.require_root()?.join(MARKER_FILE);
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
                    dcgo_harness::queue::sweep_timeouts(args.require_root()?, *timeout_seconds)?;
                if requeued > 0 || quarantined > 0 {
                    println!("swept: requeued={} quarantined={}", requeued, quarantined);
                }
            }
            let status = dcgo_harness::queue::scan(args.require_root()?)?;
            // Surface the enable state. A queue full of pending jobs with the
            // harness switched off looks exactly like a hung DCGO otherwise --
            // the same silent-failure shape the triage denominator guards.
            let enabled = args.require_root()?.join(MARKER_FILE).exists();
            println!("harness: {}", if enabled { "ENABLED" } else { "disabled" });
            // A queue that is not draining looks identical whether DCGO is
            // stopped, hung, or simply switched off. Say which.
            match dcgo_harness::daemon::read_pid(args.require_root()?) {
                Some(pid) if dcgo_harness::daemon::pid_alive(pid) => {
                    let health = dcgo_harness::daemon::classify_heartbeat(
                        dcgo_harness::daemon::heartbeat_age(args.require_root()?),
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

            let status = dcgo_harness::queue::scan(args.require_root()?)?;
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
            interaction_denominator,
            emit_job,
            all_diffs,
            explain_selects,
            backfill,
        } => run_exam(
            scenario,
            *sim_only,
            sidecar.as_deref(),
            cards_json,
            decks.as_deref(),
            verdicts.as_deref(),
            clause_text_json.as_deref(),
            interaction_denominator,
            emit_job.as_deref(),
            *all_diffs,
            *explain_selects,
            *backfill,
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
            println!("{}", dcgo_harness::daemon::up(args.require_root()?, build)?);
            Ok(ExitCode::SUCCESS)
        }
        Command::Down => {
            println!("{}", dcgo_harness::daemon::down(args.require_root()?)?);
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
                args.require_root()?,
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
        Command::MigrateVerdicts { from, to } => {
            // `save_dir` prunes any *.json under `to` that isn't a card the
            // store being written carries. Re-running this migration against
            // a recovered copy of the old blob -- a plausible operator move
            // after a bad merge -- would otherwise silently delete every
            // card file the live ledger has grown since the migration ran,
            // while printing a cheerful success line. Refuse instead.
            if migrate_verdicts_destination_already_populated(to)? {
                return Err(format!(
                    "{} already contains *.json files -- the migration has already \
                     run against this directory. Refusing to re-run it: save_dir \
                     prunes any file not present in the source store, so doing so \
                     here would delete every card file added since the migration.",
                    to.display()
                ));
            }
            let store = dcgo_harness::exam::verdict::VerdictStore::load(from)?;
            let cards: std::collections::BTreeSet<String> =
                store.iter().map(|(_, cv)| cv.card_id.clone()).collect();
            let rows = store.len();
            store.save_dir(to)?;
            println!(
                "migrated {rows} verdicts across {} cards: {} -> {}",
                cards.len(),
                from.display(),
                to.display()
            );
            Ok(ExitCode::SUCCESS)
        }
        Command::VerdictTriage { clause, triage, citation, verdicts } => {
            use dcgo_harness::exam::verdict::{Triage, VerdictStore};
            let class = match triage.as_str() {
                "ours_wrong" => Triage::OursWrong,
                "dcgo_quirk" => Triage::DcgoQuirk,
                "undetermined" => Triage::Undetermined,
                other => return Err(format!(
                    "--triage must be ours_wrong | dcgo_quirk | undetermined, got `{other}`"
                )),
            };
            let mut store = VerdictStore::load_dir(verdicts)?;
            store.set_triage(clause, class, citation.clone())?;
            store.save_dir(verdicts)?;
            println!("verdict-triage: {clause} -> {triage}");
            Ok(ExitCode::SUCCESS)
        }
        Command::Mcp => {
            dcgo_harness::mcp::serve(args.root.clone())?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Node { action } => {
            let root = args.require_root()?;
            match action {
                NodeAction::Up { build } => {
                    println!("{}", dcgo_harness::node::up(root, build)?);
                    Ok(ExitCode::SUCCESS)
                }
                NodeAction::Down => {
                    println!("{}", dcgo_harness::node::down(root)?);
                    Ok(ExitCode::SUCCESS)
                }
                NodeAction::Status { build } => {
                    let h = dcgo_harness::node::health(root, build.as_deref());
                    println!("{}", h.describe());
                    // Scripts and CI gate on this: a NO-GO must exit non-zero
                    // rather than requiring the caller to parse the text.
                    Ok(if h.go { ExitCode::SUCCESS } else { ExitCode::from(1) })
                }
            }
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
    /// Every clause the line covers (`covers:`, default `[clause]`). A clause
    /// scenario's verdict is recorded for each.
    clause_ids: Vec<String>,
    /// Set on an interaction exam: the verdict is filed under this id INSTEAD
    /// of any clause -- a passing negative probe proves a clause did NOT fire,
    /// which must never read as that clause confirmed.
    interaction_id: Option<String>,
    card: String,
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
    interaction_denominator: &Path,
    emit_job: Option<&Path>,
    all_diffs: bool,
    explain_selects: bool,
    backfill: bool,
) -> Result<ExitCode, String> {
    use dcgo_harness::exam::verdict::{ClauseTextBook, InteractionBook, VerdictStore};

    if !sim_only && sidecar.is_none() {
        return Err(
            "exam needs --sidecar <dcgo .state.jsonl> unless --sim-only. Without an \
             oracle trace there is nothing to diff against, and silently degrading \
             to an assertion-only run would report oracle agreement nobody measured."
                .to_string(),
        );
    }
    if backfill && sim_only {
        return Err("--backfill needs an oracle run (--sidecar); sim-only confirms nothing".into());
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
            let store = VerdictStore::load_dir(path)?;
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
            explain_selects,
            backfill,
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
        // Loaded on first use: a clause-only run never needs the interaction
        // denominator, and must not fail for its absence.
        let mut interaction_book: Option<Result<InteractionBook, String>> = None;
        for ev in &events {
            let ids: Vec<String> = match &ev.interaction_id {
                Some(id) => vec![id.clone()],
                None => ev.clause_ids.clone(),
            };
            for id in ids {
                let result = if ev.interaction_id.is_some() {
                    match interaction_book
                        .get_or_insert_with(|| InteractionBook::load(interaction_denominator))
                    {
                        Ok(ib) => dcgo_harness::exam::verdict::record_interaction_verdict(
                            store,
                            ib,
                            &id,
                            std::slice::from_ref(&ev.card),
                            ev.verdict,
                            Some(ev.scenario_path.clone()),
                            ev.reason.clone(),
                            recorded_at.clone(),
                        ),
                        Err(e) => Err(format!("refusing interaction {id}: {e}")),
                    }
                } else {
                    dcgo_harness::exam::verdict::record_scenario_verdict(
                        store,
                        book,
                        &id,
                        ev.verdict,
                        Some(ev.scenario_path.clone()),
                        ev.reason.clone(),
                        recorded_at.clone(),
                    )
                };
                match result {
                    Ok(()) => println!(
                        "exam: verdict {} recorded for {} ({})",
                        ev.verdict, id, ev.scenario_path
                    ),
                    Err(e) => {
                        println!("exam: VERDICT NOT RECORDED: {e}");
                        verdicts_refused += 1;
                    }
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
        store.save_dir(store_path)?;
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
             clause-text denominator, or interaction id outside the interaction \
             denominator) -- failing the run."
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
    explain_selects: bool,
    backfill: bool,
    lowered: &mut u32,
    ran: &mut u32,
    diffed: &mut u32,
) -> Result<(ExamOutcome, Option<VerdictEvent>), String> {
    use dcgo_harness::exam::differ::diff_paired;
    use dcgo_harness::exam::projection::{
        align_to_scenario_origin, pair_by_wire_rows_with_ownership, parse_sidecar,
    };
    use dcgo_harness::exam::run::lower_and_run;
    use dcgo_harness::exam::scenario::Scenario;

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
    // Unity time is spent. Shared with the MCP probe surface
    // (`dcgo_harness::exam::run::lower_and_run`) precisely so this and
    // `exam_probe` can never disagree about what a line lowers to -- see
    // that function's doc comment.
    let run = lower_and_run(&s, deck_p0, deck_p1, card_data)?;
    *lowered += 1;
    println!(
        "  lowered {} step(s): {:?}",
        s.steps.len(),
        run.lowered_steps
    );
    if explain_selects {
        for src in &run.select_sources {
            println!(
                "  select step {}: source {} ({}) -- {}",
                src.step,
                src.card_id.as_deref().unwrap_or("?"),
                src.kind,
                src.prompt
            );
        }
    }

    // Sim-only path (checked in run_exam): the line lowered cleanly, so emit
    // the DCGO scripted job that runs the same line against the oracle.
    if let Some(dir) = emit_job {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("unnamed scenario {}", path.display()))?;
        let e0 = book.resolve(&s.decks.p0.rest)?;
        let e1 = book.resolve(&s.decks.p1.rest)?;
        let job = build_exam_job(stem, &s, e0, e1, &run.lowered_steps, &run.lowered_owners)?;
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("creating {}: {e}", dir.display()))?;
        let out = dir.join(format!("{}.json", job.job_id));
        let mut text = serde_json::to_string_pretty(&job)
            .map_err(|e| format!("serializing job for {}: {e}", path.display()))?;
        text.push('\n');
        std::fs::write(&out, text).map_err(|e| format!("writing {}: {e}", out.display()))?;
        println!("  emitted job {}", out.display());
    }

    *ran += 1;
    if !run.complete {
        return Ok((
            fail(format!(
                "the line did not run to completion: {} of {} steps{}",
                run.steps_run,
                run.steps_total,
                if run.stall_reasons.is_empty() {
                    String::new()
                } else {
                    format!("
    {}", run.stall_reasons.join("
    "))
                }
            )),
            None,
        ));
    }
    // This is how the differ pairs our per-STEP rows against DCGO's
    // per-DECISION rows.
    let wire_rows_per_step = run.wire_rows_per_step;
    let ours_present_per_step = run.ours_present_per_step;
    let projections = run.projections;

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
        // A Q&A exam's third leg: does OUR engine agree with the publisher?
        // Contradicting a ruling fails the run like a failed assert would.
        let ruling_ok = ruling_leg(&s, &projections);
        return Ok((
            if failures.is_empty() && ruling_ok != Some(false) {
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
    let pairing =
        pair_by_wire_rows_with_ownership(&wire_rows_per_step, &ours_present_per_step, dcgo.len());
    let report = diff_paired(&ours_for_diff, &dcgo, &pairing);
    *diffed += 1;
    if all_diffs {
        for line in report.render_verbose().lines() {
            println!("  {line}");
        }
    } else {
        println!("  {report}");
    }

    if backfill && report.is_clean() {
        // Our rows equal DCGO's on every compared field when the diff is clean.
        // Backfill writes exactly the rows the pairing compared: never the
        // trailing post-final-step row, never a sim-only or dcgo-only step.
        //
        // A refusal is reported, never propagated: the oracle verdict below is
        // independent of whether the scenario file could be updated.
        println!(
            "  {}",
            try_backfill(path, &text, &ours_for_diff, &dcgo, &pairing, &report)
        );
    }

    // What this run established about the clause, for the verdict store: a
    // CLEAN oracle diff confirms, anything else (divergence or truncation)
    // is a finding to triage. A Q&A exam additionally needs our engine to
    // agree with the ruling (design D7: ours = DCGO but not the ruling is
    // `ours_wrong`, never `confirmed`).
    let ruling_ok = ruling_leg(&s, &projections);
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
    let event = VerdictEvent {
        clause_ids: s.covered_clauses(),
        interaction_id: s.interaction.as_ref().map(|i| i.id.clone()),
        card: s.card.clone(),
        verdict: if clean {
            dcgo_harness::exam::verdict::Verdict::Confirmed
        } else {
            dcgo_harness::exam::verdict::Verdict::Diverged
        },
        reason,
        scenario_path: path.display().to_string(),
    };

    Ok((
        if clean {
            ExamOutcome::Passed
        } else {
            ExamOutcome::CheckFailed
        },
        Some(event),
    ))
}

/// Evaluate a Q&A scenario's `expect_ruling:` against OUR projected trace and
/// print the leg. `None` when the scenario carries no ruling.
fn ruling_leg(
    s: &dcgo_harness::exam::scenario::Scenario,
    projections: &[dcgo_harness::exam::projection::StateProjection],
) -> Option<bool> {
    let (checked, failures) = dcgo_harness::exam::check_ruling(s, projections)?;
    let q = s.expect_ruling.as_ref().map(|r| r.q_id.as_str()).unwrap_or_default();
    for f in &failures {
        println!("  RULING qa:{q} CONTRADICTED: {f}");
    }
    // Zero checks is a vacuous ruling, which must not read as agreement.
    let agrees = failures.is_empty() && checked > 0;
    println!(
        "  ruling qa:{q}: ours {} ({checked} check(s), {} failed)",
        if agrees { "agrees" } else if checked == 0 { "is UNCHECKED (vacuous ruling)" } else { "CONTRADICTS the ruling" },
        failures.len()
    );
    Some(agrees)
}

/// Write a clean oracle run's confirmed state into the scenario at `path`.
/// Returns the line to print. Never fails the caller: a refused or failed
/// backfill is `backfill skipped: <reason>` and the file is left untouched.
fn try_backfill(
    path: &Path,
    text: &str,
    rows: &[dcgo_harness::exam::projection::StateProjection],
    dcgo: &[dcgo_harness::exam::projection::StateProjection],
    pairing: &dcgo_harness::exam::projection::StepPairing,
    report: &dcgo_harness::exam::differ::DiffReport,
) -> String {
    match dcgo_harness::exam::backfill::backfill_from_diff(text, rows, dcgo, pairing, report) {
        Ok(updated) => match std::fs::write(path, updated) {
            Ok(()) => format!("backfill: wrote confirmed state into {}", path.display()),
            Err(e) => format!(
                "backfill skipped: writing backfilled scenario {}: {e}",
                path.display()
            ),
        },
        Err(reason) => format!("backfill skipped: {reason}"),
    }
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

// `check_assertions`, `DeckEntry`/`DeckBook`/`ordered_deck` moved to
// `dcgo_harness::exam` (2026-08-28, MCP task 6) so the CLI and the MCP's
// `run_scenario` / `exam_probe` share one definition of both -- see
// `exam::assertions` and `exam::deckbook` for the code and the rationale.
use dcgo_harness::exam::{check_assertions, DeckBook, ordered_deck};
use dcgo_harness::exam::job_spec::build_exam_job;

#[cfg(test)]
mod migrate_verdicts_guard_tests {
    use super::*;

    #[test]
    fn missing_destination_is_not_already_migrated() {
        let tmp = std::env::temp_dir().join("migrate_verdicts_guard_missing");
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(!migrate_verdicts_destination_already_populated(&tmp).unwrap());
    }

    #[test]
    fn empty_destination_is_not_already_migrated() {
        let tmp = std::env::temp_dir().join("migrate_verdicts_guard_empty");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        assert!(!migrate_verdicts_destination_already_populated(&tmp).unwrap());
    }

    #[test]
    fn destination_holding_a_json_file_is_already_migrated() {
        let tmp = std::env::temp_dir().join("migrate_verdicts_guard_populated");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("EX12-035.json"), "{}").unwrap();
        assert!(migrate_verdicts_destination_already_populated(&tmp).unwrap());
    }

    #[test]
    fn destination_holding_only_non_json_files_is_not_already_migrated() {
        let tmp = std::env::temp_dir().join("migrate_verdicts_guard_non_json");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("README.md"), "notes").unwrap();
        assert!(!migrate_verdicts_destination_already_populated(&tmp).unwrap());
    }

    #[test]
    fn migrate_verdicts_refuses_when_destination_is_already_populated() {
        // End-to-end: a recovered copy of the old blob, re-migrated against a
        // `--to` that already has per-card files (added since the real
        // migration ran), must be refused rather than pruned.
        let tmp = std::env::temp_dir().join("migrate_verdicts_guard_end_to_end");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let mut blob = dcgo_harness::exam::verdict::VerdictStore::default();
        blob.record(dcgo_harness::exam::verdict::ClauseVerdict {
            clause_id: "EX12-035#effect#0".to_string(),
            card_id: "EX12-035".to_string(),
            verdict: dcgo_harness::exam::verdict::Verdict::Confirmed,
            label: String::new(),
            text_sha256: "sha-ex12-035".to_string(),
            scenario_path: None,
            reason: None,
            dcgo_build: None,
            job_id: None,
            triage: None,
            citation: None,
            recorded_at: "2026-01-01T00:00:00Z".to_string(),
        });
        let from = tmp.join("dcgo_exam_verdicts.json");
        blob.save(&from).unwrap();

        // The live ledger has grown a card since the real migration ran.
        let to = tmp.join("exam-verdicts");
        std::fs::create_dir_all(&to).unwrap();
        std::fs::write(to.join("BT8-084.json"), "{}").unwrap();

        let args = Args {
            root: None,
            command: Command::MigrateVerdicts {
                from: from.clone(),
                to: to.clone(),
            },
        };
        let err = run(&args).expect_err("must refuse a populated destination");
        assert!(err.contains("already contains"), "{err}");
        assert!(err.contains(&to.display().to_string()), "{err}");

        // Refused before ever touching save_dir: the pre-existing file
        // survives untouched.
        assert!(to.join("BT8-084.json").exists());
        assert!(!to.join("EX12-035.json").exists());
    }
}

#[cfg(test)]
mod try_backfill_tests {
    use super::*;
    use dcgo_harness::exam::differ::DiffReport;
    use dcgo_harness::exam::projection::{StateProjection, StepPairing};

    fn pairs(p: &[(usize, usize)]) -> StepPairing {
        StepPairing { pairs: p.to_vec(), ours_unpairable: 0, dcgo_unpairable: 0 }
    }

    const SCENARIO: &str = "card: EX12-035\nclause: EX12-035#effect#0\nseed: 1\ndecks:\n  p0: { stack: [ST1-02], rest: st1 }\n  p1: { stack: [], rest: st1 }\nsteps:\n  - actor: 0\n    do: { pass: {} }\n";

    fn row(step: u32) -> StateProjection {
        StateProjection::from_sidecar_line(&format!(
            r#"{{"step":{step},"turn":1,"phase":"Main","memory":0,
               "p0":{{"security":5,"hand":[],"trash":[],"field":[]}},
               "p1":{{"security":5,"hand":[],"trash":[],"field":[]}}}}"#
        ))
        .unwrap()
    }

    fn report(ours_unpairable: u32) -> DiffReport {
        DiffReport {
            compared_steps: 1,
            ours_steps: 1 + ours_unpairable,
            dcgo_steps: 1,
            ours_unpairable,
            dcgo_unpairable: 0,
            divergences: vec![],
        }
    }

    #[test]
    fn a_refused_backfill_is_reported_not_propagated_and_leaves_the_file_alone() {
        let path = std::env::temp_dir().join("try_backfill_refused.yaml");
        std::fs::write(&path, SCENARIO).unwrap();
        let r = report(0);
        assert!(r.is_clean(), "the verdict inputs are a CLEAN diff");

        // A pairing naming a row the caller never supplied is refused.
        let line = try_backfill(&path, SCENARIO, &[row(0)], &[row(0)], &pairs(&[(3, 0)]), &r);
        assert!(line.starts_with("backfill skipped: "), "got: {line}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), SCENARIO);
        // The clean report is untouched, so the caller still builds Confirmed.
        assert!(r.is_clean());
    }

    #[test]
    fn an_accepted_backfill_writes_the_file() {
        let path = std::env::temp_dir().join("try_backfill_accepted.yaml");
        std::fs::write(&path, SCENARIO).unwrap();
        let line = try_backfill(&path, SCENARIO, &[row(0)], &[row(0)], &pairs(&[(0, 0)]), &report(0));
        assert!(
            line.starts_with("backfill: wrote confirmed state into"),
            "got: {line}"
        );
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.starts_with(SCENARIO) && written.contains("_backfilled"));
    }
}
