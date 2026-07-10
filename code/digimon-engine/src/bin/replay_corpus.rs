use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use digimon_engine::card_data::CardData;
use digimon_engine::cards::build_registry;
use digimon_engine::recorder::cards_json_content_hash;
use digimon_engine::replay_corpus::{
    bless_replay_corpus_dir, convert_training_recording_artifact, generate_greedy_replay_corpus,
    verify_replay_corpus_dir, write_replay_corpus_entries, ConversionOutcome, ReplayCorpusConfig,
    ReplayCorpusEntry, DEFAULT_REPLAY_CORPUS_BASE_SEED, DEFAULT_REPLAY_CORPUS_MAX_STEPS_PER_GAME,
    DEFAULT_TIER2_REPLAY_CORPUS_GAMES,
};

const DEFAULT_CLI_STACK_SIZE: usize = 268_435_456;

fn main() {
    let stack_size = env::var("RUST_MIN_STACK")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(DEFAULT_CLI_STACK_SIZE);
    let handle = std::thread::Builder::new()
        .name("replay-corpus".to_string())
        .stack_size(stack_size)
        .spawn(run)
        .expect("spawn replay corpus worker thread");

    match handle.join() {
        Ok(Ok(())) => {}
        Ok(Err(err)) => {
            eprintln!("error: {err}");
            eprintln!();
            usage();
            std::process::exit(1);
        }
        Err(_) => {
            eprintln!("error: replay corpus worker thread panicked");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some(command) = args.first().map(String::as_str) else {
        return Err("missing command".to_string());
    };
    if command == "-h" || command == "--help" {
        usage();
        return Ok(());
    }

    match command {
        "generate" => generate(&args[1..]),
        "convert-runs" => convert_runs(&args[1..]),
        "verify" => verify(&args[1..]),
        "bless" => bless(&args[1..]),
        other => Err(format!("unknown command {other:?}")),
    }
}

fn generate(args: &[String]) -> Result<(), String> {
    let deck_library = required_path(args, "--deck-library")?;
    let cards_path = required_path(args, "--cards")?;
    let out = required_path(args, "--out")?;
    let games = optional_usize(args, "--games", DEFAULT_TIER2_REPLAY_CORPUS_GAMES)?;
    let max_steps = optional_u32(
        args,
        "--max-steps",
        DEFAULT_REPLAY_CORPUS_MAX_STEPS_PER_GAME,
    )?;
    let seed = optional_u64(args, "--seed", DEFAULT_REPLAY_CORPUS_BASE_SEED)?;

    let deck_library_json = fs::read_to_string(&deck_library)
        .map_err(|err| format!("read {}: {err}", deck_library.display()))?;
    let cards_json =
        fs::read(&cards_path).map_err(|err| format!("read {}: {err}", cards_path.display()))?;
    let card_data = CardData::load_from_file(&cards_path)?;
    let implemented: HashSet<String> = build_registry().registered_card_ids().into_iter().collect();

    let corpus = generate_greedy_replay_corpus(
        &deck_library_json,
        &cards_json,
        &card_data,
        &implemented,
        ReplayCorpusConfig {
            game_count: games,
            max_steps_per_game: max_steps,
            base_seed: seed,
        },
    )
    .map_err(|err| err.to_string())?;
    let paths =
        write_replay_corpus_entries(&out, &corpus.entries).map_err(|err| err.to_string())?;

    println!(
        "wrote {} generated replay(s) to {}",
        paths.len(),
        out.display()
    );
    if !corpus.skips.is_empty() {
        println!("skipped {} decklist(s)", corpus.skips.len());
    }
    Ok(())
}

fn bless(args: &[String]) -> Result<(), String> {
    let goldens = required_path(args, "--goldens")?;
    let cards_path = required_path(args, "--cards")?;
    let retired = optional_path(args, "--retired").unwrap_or_else(|| goldens.join("retired"));

    let card_data = CardData::load_from_file(&cards_path)?;
    let summary =
        bless_replay_corpus_dir(&goldens, &retired, &card_data).map_err(|err| err.to_string())?;

    println!(
        "blessed {} replay(s) in {}; {} unchanged; retired {} replay(s)",
        summary.updated,
        goldens.display(),
        summary.unchanged,
        summary.retired.len()
    );
    for retired in summary.retired {
        let source = retired.source_file.unwrap_or_else(|| "unknown".to_string());
        println!("retired {source}: {}", retired.reason);
    }
    Ok(())
}

fn verify(args: &[String]) -> Result<(), String> {
    let goldens = required_path(args, "--goldens")?;
    let cards_path = required_path(args, "--cards")?;

    let card_data = CardData::load_from_file(&cards_path)?;
    let summary = verify_replay_corpus_dir(&goldens, &card_data).map_err(|err| err.to_string())?;

    if summary.divergent.is_empty() {
        println!(
            "verified {} replay(s) in {}",
            summary.checked,
            goldens.display()
        );
        return Ok(());
    }

    for report in &summary.divergent {
        if let Some(divergence) = &report.divergence {
            eprintln!(
                "divergence in {} at step {} ({:?}): recorded={} replayed={}",
                divergence.game,
                divergence.step,
                divergence.check_type,
                divergence.recorded,
                divergence.replayed
            );
        }
    }
    Err(format!(
        "{} of {} replay(s) diverged",
        summary.divergent.len(),
        summary.checked
    ))
}

fn convert_runs(args: &[String]) -> Result<(), String> {
    let runs = required_path(args, "--runs")?;
    let cards_path = required_path(args, "--cards")?;
    let out = required_path(args, "--out")?;
    let limit = optional_usize(args, "--limit", usize::MAX)?;

    let cards_json =
        fs::read(&cards_path).map_err(|err| format!("read {}: {err}", cards_path.display()))?;
    let cards_hash = cards_json_content_hash(&cards_json);
    let card_data = CardData::load_from_file(&cards_path)?;

    let mut files = Vec::new();
    collect_json_files(&runs, &mut files)?;
    files.sort();

    let mut entries = Vec::new();
    let mut skipped = 0usize;
    for path in files.into_iter().take(limit) {
        let text =
            fs::read_to_string(&path).map_err(|err| format!("read {}: {err}", path.display()))?;
        let artifact: serde_json::Value = serde_json::from_str(&text)
            .map_err(|err| format!("parse {}: {err}", path.display()))?;
        match convert_training_recording_artifact(&artifact, &cards_hash, &card_data)
            .map_err(|err| err.to_string())?
        {
            ConversionOutcome::Converted(recording) => entries.push(ReplayCorpusEntry {
                name: format!("converted_{}.replay.json", slug(&path)),
                recording,
            }),
            ConversionOutcome::Skipped(_) => skipped += 1,
        }
    }

    let paths = write_replay_corpus_entries(&out, &entries).map_err(|err| err.to_string())?;
    println!(
        "wrote {} converted replay(s) to {}; skipped {} artifact(s)",
        paths.len(),
        out.display(),
        skipped
    );
    Ok(())
}

fn collect_json_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|err| format!("read {}: {err}", dir.display()))? {
        let path = entry
            .map_err(|err| format!("read {} entry: {err}", dir.display()))?
            .path();
        if path.is_dir() {
            collect_json_files(&path, out)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            out.push(path);
        }
    }
    Ok(())
}

fn required_path(args: &[String], flag: &str) -> Result<PathBuf, String> {
    value_after(args, flag)
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing {flag}"))
}

fn optional_path(args: &[String], flag: &str) -> Option<PathBuf> {
    value_after(args, flag).map(PathBuf::from)
}

fn optional_usize(args: &[String], flag: &str, default: usize) -> Result<usize, String> {
    value_after(args, flag)
        .map(|raw| raw.parse::<usize>().map_err(|err| format!("{flag}: {err}")))
        .transpose()
        .map(|value| value.unwrap_or(default))
}

fn optional_u32(args: &[String], flag: &str, default: u32) -> Result<u32, String> {
    value_after(args, flag)
        .map(|raw| raw.parse::<u32>().map_err(|err| format!("{flag}: {err}")))
        .transpose()
        .map(|value| value.unwrap_or(default))
}

fn optional_u64(args: &[String], flag: &str, default: u64) -> Result<u64, String> {
    value_after(args, flag)
        .map(|raw| raw.parse::<u64>().map_err(|err| format!("{flag}: {err}")))
        .transpose()
        .map(|value| value.unwrap_or(default))
}

fn value_after<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.windows(2)
        .find_map(|pair| (pair[0] == flag).then_some(pair[1].as_str()))
}

fn slug(path: &Path) -> String {
    let raw = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("recording");
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

fn usage() {
    eprintln!(
        "usage:\n  replay_corpus generate --deck-library data/deck_library.json --cards data/cards.json --out qa/replay-goldens [--games N] [--max-steps N] [--seed N]\n  replay_corpus verify --goldens qa/replay-goldens --cards data/cards.json\n  replay_corpus bless --goldens qa/replay-goldens --cards data/cards.json [--retired qa/replay-goldens/retired]\n  replay_corpus convert-runs --runs runs --cards data/cards.json --out qa/replay-goldens [--limit N]"
    );
}
