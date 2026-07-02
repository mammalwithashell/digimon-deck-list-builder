//! `digimon-engine-cli` — interactive REPL, replay viewer, and (stubbed)
//! scenario runner for the Rust source-of-truth engine.
//!
//! Phase 5 of the `add-engine-debug-mcp` change. Sits directly on top of
//! `digimon_engine::live_game::LiveGame`; no Python at runtime.

use std::collections::HashMap;
use std::collections::HashSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use digimon_engine::card_data::CardData;
use digimon_engine::live_game::LiveGame;
use digimon_engine::view::Perspective;
use serde_json::Value;

mod debug_repl;
mod replay_view;
mod winprob;

/// Top-level CLI parser.
#[derive(Parser, Debug)]
#[command(
    name = "digimon-engine-cli",
    version,
    about = "Interactive debugger + replay viewer + scenario runner for the Rust Digimon TCG engine",
    long_about = None,
)]
struct Cli {
    /// Card-pool source. Default = `implemented` (cards registered in the
    /// engine's CardEffectRegistry, same filter pilot_training and the
    /// architect agents use). Use `all` to load every card in
    /// `data/cards.json`. Or pass a JSON path containing a card-id array.
    #[arg(long, global = true, default_value = "implemented")]
    pool: String,

    /// Optional override path for `cards.json`. Defaults to the engine's
    /// embedded `cards.json` location if it can be found in the current
    /// repo. For v1, this flag is REQUIRED — the CLI does not embed the
    /// full card database.
    #[arg(long, global = true)]
    cards_json: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Launch an interactive REPL backed by a fresh or recording-derived
    /// `LiveGame`. Reads one command per line from stdin.
    Debug {
        /// Construct from these two deck files (JSON arrays of card_ids).
        /// Optional — REPL also supports `new decks <path> <path>`.
        #[arg(long)]
        deck1: Option<PathBuf>,
        #[arg(long)]
        deck2: Option<PathBuf>,
        /// Seed for shuffling. Defaults to system entropy.
        #[arg(long)]
        seed: Option<u64>,
    },

    /// Load a `GameRecorder` recording and print a chosen view at a
    /// chosen step. Single-shot — no REPL.
    Replay {
        /// Path to the recording JSON.
        recording: PathBuf,
        /// Step to seek to (0 = initial post-mulligan state). Defaults to 0.
        #[arg(long, default_value_t = 0)]
        step: u32,
        /// Perspective: `god` (default), `player0`, or `player1`.
        #[arg(long, default_value = "god")]
        view: String,
        /// Which view to print: `state` (default), `hand`, `field`,
        /// `security`, `pending`, `queue`, `events`, `actions`.
        #[arg(long, default_value = "state")]
        show: String,
        /// Player whose zone the `hand`/`field`/`security`/`actions`
        /// view should target. Defaults to 0.
        #[arg(long, default_value_t = 0u8)]
        player: u8,
        /// Run replay in verify mode (compares replayed state to recorded
        /// values, prints divergences).
        #[arg(long)]
        verify: bool,
    },

    /// Replay a `GameRecorder` recording and evaluate every decision with
    /// a policy+value ONNX export (task-0.2 inline-value graph), emitting a
    /// per-decision value / played-prior sidecar JSON — the review-v0
    /// "win-probability bar" trace (add-determinized-search task 0.5).
    Winprob {
        /// Path to the recording JSON (native GameRecorder format).
        recording: PathBuf,
        /// Path to the policy+value .onnx (must have an inline `value` output).
        #[arg(long)]
        model: PathBuf,
        /// Output sidecar path. Defaults to `<recording stem>.winprob.json`.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Evaluation batch size.
        #[arg(long, default_value_t = 256)]
        batch: usize,
    },

    /// Print the card pool as a sorted JSON array of card IDs and exit.
    /// With the default `--pool implemented`, this is the set of cards the
    /// engine has a registered `CardEffect` for, intersected with
    /// `cards.json` (so test fixtures and token IDs are excluded).
    /// `tools/build_tested_cards.py` consumes this to regenerate the
    /// deck-builder allowlist snapshot.
    Pool,

    /// Run a YAML scenario file (or a directory of them). v1 deliberately
    /// stubs this — the Python `ScenarioRunner` shape is rich enough that
    /// porting it is out of v1 scope; the stub exits with a clear message
    /// and a non-zero code so CI surfaces it.
    Scenario {
        /// Path to a single scenario YAML file or a directory.
        path: PathBuf,
        /// Glob pattern for batch mode. Default matches all YAML.
        #[arg(long, default_value = "*.yaml")]
        pattern: String,
    },
}

fn main() -> ExitCode {
    // Run the real work on a worker thread with a large stack. Building the
    // engine's `CardEffectRegistry` (via `LiveGame::default_pool()`, reached
    // by `Pool` and the game-constructing subcommands) recurses deeply enough
    // to overflow the OS-default main-thread stack on Windows (~1 MB),
    // aborting with STATUS_STACK_OVERFLOW. `RUST_MIN_STACK` only governs
    // spawned threads, not `main`, so the fix is to spawn one explicitly.
    let stack_size = std::env::var("RUST_MIN_STACK")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(256 * 1024 * 1024);
    std::thread::Builder::new()
        .stack_size(stack_size)
        .spawn(run)
        .expect("failed to spawn worker thread")
        .join()
        .expect("worker thread panicked")
}

fn run() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Debug { deck1, deck2, seed } => {
            let card_data = match load_card_data(&cli.cards_json, &cli.pool) {
                Ok(cd) => cd,
                Err(e) => {
                    eprintln!("error loading card data: {}", e);
                    return ExitCode::from(2);
                }
            };
            debug_repl::run(deck1, deck2, seed, card_data)
        }
        Command::Replay {
            recording,
            step,
            view,
            show,
            player,
            verify,
        } => {
            let card_data = match load_card_data(&cli.cards_json, &cli.pool) {
                Ok(cd) => cd,
                Err(e) => {
                    eprintln!("error loading card data: {}", e);
                    return ExitCode::from(2);
                }
            };
            replay_view::run(recording, step, &view, &show, player, verify, card_data)
        }
        Command::Winprob {
            recording,
            model,
            out,
            batch,
        } => {
            let card_data = match load_card_data(&cli.cards_json, &cli.pool) {
                Ok(cd) => cd,
                Err(e) => {
                    eprintln!("error loading card data: {}", e);
                    return ExitCode::from(2);
                }
            };
            winprob::run(recording, model, out, batch, card_data)
        }
        Command::Pool => {
            let card_data = match load_card_data(&cli.cards_json, &cli.pool) {
                Ok(cd) => cd,
                Err(e) => {
                    eprintln!("error loading card data: {}", e);
                    return ExitCode::from(2);
                }
            };
            let mut ids: Vec<&String> = card_data.keys().collect();
            ids.sort();
            match serde_json::to_string_pretty(&ids) {
                Ok(json) => {
                    println!("{}", json);
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error serializing pool: {}", e);
                    ExitCode::from(2)
                }
            }
        }
        Command::Scenario { path, pattern } => {
            let _ = (path, pattern);
            eprintln!(
                "scenario subcommand is not yet implemented in v1. \
                 The Python ScenarioRunner shape (deck1_archetype, fuzzy `find:` \
                 actions, archetype-derived setup) is out of scope for v1. \
                 Track in the next change after `add-engine-debug-mcp` lands."
            );
            ExitCode::from(64) // EX_USAGE
        }
    }
}

/// Load the `CardData` map from disk, filtered by the pool spec.
///
/// Pool spec semantics:
/// - `implemented` — intersect with `LiveGame::default_pool()`. Default.
/// - `all` — no filter.
/// - any other string — treated as a path to a JSON file containing a
///   `[card_id, ...]` array; intersect with that set.
fn load_card_data(
    cards_json: &Option<PathBuf>,
    pool_spec: &str,
) -> Result<HashMap<String, CardData>, String> {
    let path = cards_json
        .clone()
        .or_else(default_cards_json_path)
        .ok_or_else(|| {
            "no --cards-json path provided and no default cards.json found in current dir or repo root"
                .to_string()
        })?;
    let bytes = std::fs::read(&path).map_err(|e| format!("reading {}: {}", path.display(), e))?;
    let text =
        std::str::from_utf8(&bytes).map_err(|e| format!("cards.json is not valid UTF-8: {}", e))?;
    let all = CardData::load_from_str(text).map_err(|e| format!("parsing cards.json: {}", e))?;

    match pool_spec {
        "all" => Ok(all),
        "implemented" => {
            let pool = LiveGame::default_pool();
            Ok(all
                .into_iter()
                .filter(|(id, _)| pool.contains(id))
                .collect())
        }
        other => {
            // Treat `other` as a path to a JSON array of card_ids.
            let p = PathBuf::from(other);
            let pool_bytes = std::fs::read(&p)
                .map_err(|e| format!("reading pool file {}: {}", p.display(), e))?;
            let pool: Vec<String> = serde_json::from_slice(&pool_bytes)
                .map_err(|e| format!("parsing pool file as [card_id...]: {}", e))?;
            let pool: HashSet<String> = pool.into_iter().collect();
            Ok(all
                .into_iter()
                .filter(|(id, _)| pool.contains(id))
                .collect())
        }
    }
}

/// Walk up from CWD looking for a `data/cards.json`. Returns `None` if
/// none found.
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

/// Parse a perspective string (`god` / `player0` / `player1`) into the
/// engine enum. Falls back to `God` on unknown input.
pub(crate) fn parse_perspective(s: &str) -> Perspective {
    match s {
        "player0" | "p0" | "0" => Perspective::Player(0),
        "player1" | "p1" | "1" => Perspective::Player(1),
        _ => Perspective::God,
    }
}

/// Helper used by both subcommands and tests — load a deck file (a JSON
/// `[card_id, ...]` array).
pub(crate) fn load_deck(path: &std::path::Path) -> Result<Vec<String>, String> {
    let bytes =
        std::fs::read(path).map_err(|e| format!("reading deck {}: {}", path.display(), e))?;
    serde_json::from_slice::<Vec<String>>(&bytes)
        .map_err(|e| format!("parsing deck {}: {}", path.display(), e))
}

/// Pretty-print a `serde_json::Value` to stdout with 2-space indent.
pub(crate) fn print_json(v: &Value) {
    let pretty = serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string());
    println!("{}", pretty);
}

/// Read one line from `reader`, returning `None` on EOF. Trims the
/// trailing newline. Used by the REPL.
pub(crate) fn read_line<R: BufRead>(reader: &mut R) -> Option<String> {
    let mut buf = String::new();
    match reader.read_line(&mut buf) {
        Ok(0) => None,
        Ok(_) => Some(buf.trim_end_matches(['\n', '\r']).to_string()),
        Err(_) => None,
    }
}

/// Read an entire file or stdin (`-`) into a JSON value.
pub(crate) fn read_json_path(path: &std::path::Path) -> Result<Value, String> {
    let bytes = if path.as_os_str() == "-" {
        let mut v = Vec::new();
        std::io::stdin()
            .read_to_end(&mut v)
            .map_err(|e| format!("reading stdin: {}", e))?;
        v
    } else {
        std::fs::read(path).map_err(|e| format!("reading {}: {}", path.display(), e))?
    };
    serde_json::from_slice(&bytes).map_err(|e| format!("parsing JSON: {}", e))
}

/// Tiny io-flush helper used by the REPL prompt.
pub(crate) fn flush_stdout() {
    let _ = std::io::stdout().flush();
}

/// Shared reader factory — separated so tests can pipe scripted input.
pub(crate) fn stdin_reader() -> BufReader<std::io::Stdin> {
    BufReader::new(std::io::stdin())
}
