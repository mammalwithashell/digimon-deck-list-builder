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
