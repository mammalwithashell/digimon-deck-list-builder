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

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use digimon_engine::card_data::CardData;

use dcgo_replay::{
    aggregate, parse_jsonl, replay_recording, ParityReport, RecordingV1, ReplayConfig,
    ReplayOutcome,
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
            Ok(t) => t,
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

fn load_card_data(cards_json: Option<&Path>) -> Result<HashMap<String, CardData>, String> {
    let path = match cards_json {
        Some(p) => p.to_path_buf(),
        None => default_cards_json_path()
            .ok_or_else(|| "no --cards-json provided and no data/cards.json found".to_string())?,
    };
    let bytes = fs::read(&path).map_err(|e| format!("reading {}: {}", path.display(), e))?;
    let text =
        std::str::from_utf8(&bytes).map_err(|e| format!("cards.json is not valid UTF-8: {}", e))?;
    CardData::load_from_str(text).map_err(|e| format!("parsing cards.json: {}", e))
}

/// Walk up from CWD looking for `data/cards.json`. Mirrors the engine CLI's
/// resolver so the harness works without flags in a developer's typical
/// repo-root invocation.
fn default_cards_json_path() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    for _ in 0..6 {
        let candidate = dir.join("data").join("cards.json");
        if candidate.exists() {
            return Some(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}
