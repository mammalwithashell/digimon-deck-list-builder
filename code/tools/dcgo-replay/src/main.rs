//! `dcgo-replay` CLI — replay one or many DCGO JSONL recordings through
//! `digimon-engine` and emit a parity report.
//!
//! Single-file mode:
//!   dcgo-replay --input recording.jsonl --cards-json data/cards.json
//!
//! Directory mode (Phase 1 bot-fuzzer loop):
//!   dcgo-replay --input recordings/dcgo/ --cards-json data/cards.json \
//!               --output parity_report.json
//!
//! Exit codes:
//!   0 — every recording passed (or hit a clean PartialPass).
//!   1 — at least one recording reported a parity failure.
//!   2 — argument or I/O error (no recordings processed).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;

use dcgo_replay::{
    aggregate, load_card_data, parse_jsonl, replay_recording, ParityReport, RecordingV1,
    ReplayConfig, ReplayFail, ReplayOutcome,
};

#[derive(Parser, Debug)]
#[command(about = "Replay DCGO recordings through digimon-engine and report parity.")]
struct Args {
    /// Input recording: either a single .jsonl file or a directory of
    /// .jsonl files (directory mode runs every file).
    #[arg(short, long)]
    input: PathBuf,

    /// Path to `data/cards.json`. If omitted, search upward from CWD for
    /// `data/cards.json` (matching the engine CLI's behavior).
    #[arg(long)]
    cards_json: Option<PathBuf>,

    /// Where to write the aggregated JSON parity report. If omitted, print
    /// the report to stdout.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Trace each step to stderr while replaying. Off in batch mode.
    #[arg(long, default_value_t = false)]
    verbose: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();
    // Run the real work on a worker thread with a large stack. Replaying a
    // recording constructs the engine's `CardEffectRegistry`, which recurses
    // deeply enough to overflow the OS-default main-thread stack on Windows
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
        .spawn(move || run(args))
        .expect("failed to spawn worker thread")
        .join()
        .expect("worker thread panicked")
}

fn run(args: Args) -> ExitCode {
    let card_data = match load_card_data(args.cards_json.as_deref()) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(2);
        }
    };

    let recording_paths = match collect_recording_paths(&args.input) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(2);
        }
    };
    if recording_paths.is_empty() {
        eprintln!(
            "error: no .jsonl files found under {}",
            args.input.display()
        );
        return ExitCode::from(2);
    }

    eprintln!("Replaying {} recording(s)...", recording_paths.len());

    let mut recordings: Vec<RecordingV1> = Vec::with_capacity(recording_paths.len());
    let mut outcomes: Vec<ReplayOutcome> = Vec::with_capacity(recording_paths.len());
    let cfg = ReplayConfig {
        verbose: args.verbose,
        ..Default::default()
    };

    for path in &recording_paths {
        let text = match fs::read_to_string(path) {
            // Recordings written before the recorder's UTF8Encoding(false) fix
            // start with a BOM, which serde_json rejects as invalid JSON.
            Ok(t) => t.trim_start_matches('\u{feff}').to_string(),
            Err(e) => {
                eprintln!("warn: skipping {}: read failed: {}", path.display(), e);
                continue;
            }
        };
        let recording = match parse_jsonl(&text) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("warn: skipping {}: parse failed: {}", path.display(), e);
                continue;
            }
        };
        let outcome = replay_recording(&recording, &card_data, &cfg);
        if args.verbose {
            eprintln!("  {} → {}", path.display(), summary(&outcome));
            eprintln!("{}", detail(&outcome));
        }
        recordings.push(recording);
        outcomes.push(outcome);
    }

    let entries: Vec<(&RecordingV1, &ReplayOutcome)> =
        recordings.iter().zip(outcomes.iter()).collect();
    let report = aggregate(&entries);

    write_report(&report, args.output.as_deref());

    eprintln!(
        "\nDone. {} pass, {} partial_pass, {} fail (of {}).",
        report.pass, report.partial_pass, report.fail, report.total_recordings
    );

    if report.fail > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn summary(outcome: &ReplayOutcome) -> &'static str {
    match outcome {
        ReplayOutcome::Pass { .. } => "PASS",
        ReplayOutcome::PartialPass { .. } => "PARTIAL",
        ReplayOutcome::Fail(_) => "FAIL",
    }
}

/// Human-readable failure detail for `--verbose` triage. The aggregated
/// JSON report deliberately stays compact (one key per failing card), so
/// this is where the per-divergence specifics — notably the engine's
/// sampled legal action IDs — surface for a human diagnosing a single
/// recording.
fn detail(outcome: &ReplayOutcome) -> String {
    match outcome {
        ReplayOutcome::Pass {
            steps_consumed,
            winner,
        } => format!("      {} steps, winner=P{}", steps_consumed, winner),
        ReplayOutcome::PartialPass {
            steps_consumed,
            stop_reason,
        } => format!("      {} steps; halted: {}", steps_consumed, stop_reason),
        ReplayOutcome::Fail(fail) => match fail {
            ReplayFail::IllegalAction(ia) => format!(
                "      illegal_action @ step {} (actor P{}, phase {}, source {})\n\
                 \x20       recorded action_id={} ({})\n\
                 \x20       engine legal sample: {:?}",
                ia.step,
                ia.actor,
                ia.phase,
                ia.source,
                ia.action_id,
                describe_action(ia.action_id),
                ia.sample_legal_ids
                    .iter()
                    .map(|id| format!("{} ({})", id, describe_action(*id)))
                    .collect::<Vec<_>>()
            ) + &indent_board(&ia.board),
            ReplayFail::ActorMismatch(am) => format!(
                "      actor_mismatch @ step {}: engine expected P{}, recording said P{} \
                 (action_id={}, phase {})",
                am.step, am.expected_actor, am.recorded_actor, am.action_id, am.phase
            ),
            ReplayFail::WinnerMismatch(wm) => format!(
                "      winner_mismatch after {} steps: recording={}, engine={}",
                wm.steps_consumed, wm.expected_winner, wm.engine_winner
            ),
            ReplayFail::EngineError { step, message } => {
                format!("      engine_error @ step {:?}: {}", step, message)
            }
            ReplayFail::OpaqueRevealError { step, message } => {
                format!("      opaque_reveal_error @ step {:?}: {}", step, message)
            }
        },
    }
}

/// Indent a captured board snapshot to line up under the failure detail.
/// Empty input yields empty output so pre-board recordings print unchanged.
fn indent_board(board: &str) -> String {
    if board.trim().is_empty() {
        return String::new();
    }
    board
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| format!("\n        {}", l))
        .collect()
}

/// Decode an action ID into its action-space range + operands, so a
/// verbose failure line reads as gameplay rather than as a bare integer.
fn describe_action(id: u16) -> String {
    use digimon_engine::action::space as sp;
    match id {
        sp::PASS => "pass".to_string(),
        sp::CONCEDE_GAME => "concede".to_string(),
        id if id < sp::HAND_EFFECT_START => format!("play_hand_{}", id),
        id if id < sp::HAND_EFFECT_START + 30 => {
            format!("hand_effect_{}", id - sp::HAND_EFFECT_START)
        }
        id if (sp::ATTACK_START..sp::ATTACK_END).contains(&id) => {
            let (attacker, target) = sp::decode_attack(id);
            format!("attack_{}_to_{}", attacker, target)
        }
        id if (sp::DIGIVOLVE_START..sp::DIGIVOLVE_END).contains(&id) => {
            let (hand, field) = sp::decode_digivolve(id);
            format!("digivolve_hand_{}_to_field_{}", hand, field)
        }
        id if (sp::FIELD_EFFECT_START..sp::FIELD_EFFECT_END).contains(&id) => {
            let (perm, effect) = sp::decode_field_effect(id);
            format!("field_slot_{}_effect_{}", perm, effect)
        }
        id if (sp::TRASH_EFFECT_START..sp::TRASH_EFFECT_END).contains(&id) => {
            format!("trash_effect_{}", id - sp::TRASH_EFFECT_START)
        }
        id if (sp::SOURCE_SELECT_START..sp::SOURCE_SELECT_END).contains(&id) => {
            let (field, source) = sp::decode_source_select(id);
            format!("source_select_field_{}_src_{}", field, source)
        }
        id => format!("action_{}", id),
    }
}

fn write_report(report: &ParityReport, output: Option<&Path>) {
    let json = serde_json::to_string_pretty(report).expect("serialize report");
    match output {
        Some(p) => match fs::write(p, &json) {
            Ok(_) => eprintln!("Wrote {} bytes to {}", json.len(), p.display()),
            Err(e) => eprintln!("error: failed to write {}: {}", p.display(), e),
        },
        None => println!("{}", json),
    }
}

/// Collect every `.jsonl` file in `path` (recursively, one level deep) or
/// the single file at `path`.
fn collect_recording_paths(path: &Path) -> Result<Vec<PathBuf>, String> {
    if !path.exists() {
        return Err(format!("input path does not exist: {}", path.display()));
    }
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }

    // Directory mode — one level deep. We don't recurse arbitrarily so a
    // user can co-locate misc files next to a `recordings/` subdir without
    // surprises.
    let mut out = Vec::new();
    let entries = fs::read_dir(path).map_err(|e| format!("read_dir {}: {}", path.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("dir entry: {}", e))?;
        let p = entry.path();
        if p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            out.push(p);
        }
    }
    out.sort(); // deterministic processing order
    Ok(out)
}
