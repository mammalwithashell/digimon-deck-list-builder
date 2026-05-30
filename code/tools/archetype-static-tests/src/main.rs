//! CLI for the static archetype-test harness.
//!
//! ```bash
//! cargo run -p archetype-static-tests -- "Rocks"
//! cargo run -p archetype-static-tests -- "Rocks" --threshold 0.9 --smoke-games 3 \
//!     --combo "Greymon removal=BT17-102,EX8-046" --json
//! ```
//!
//! Runs all four static invariants for an archetype, prints a per-invariant
//! result, and (unless `--no-write`) records the run in
//! `qa/qa-reports/archetype_interactions.json`.

use std::path::PathBuf;

use archetype_static_tests::{
    check_deck_legality, combo_presence, coverage_gate, implemented_set, load_card_data, load_json,
    record_run, repo_root, resolve_pool, smoke_games, tracker_path, Combo, InvariantResult,
};
use clap::Parser;
use serde_json::json;

#[derive(Parser, Debug)]
#[command(
    name = "archetype-static-tests",
    about = "Run the four static archetype-level invariant checks for an archetype."
)]
struct Args {
    /// Archetype name (resolved via deck_library.json + archetype_aliases.json).
    archetype: String,

    /// Coverage-gate threshold (fraction of the unique pool that must be implemented).
    #[arg(long, default_value_t = 1.0)]
    threshold: f64,

    /// Number of self-play smoke games.
    #[arg(long, default_value_t = 5)]
    smoke_games: u32,

    /// Max steps per smoke game before a forced conclusion.
    #[arg(long, default_value_t = 4000)]
    max_steps: u32,

    /// A combo to check for combo-presence, `name=CARD1,CARD2` (repeatable).
    #[arg(long = "combo")]
    combos: Vec<String>,

    /// JSON file of combos: `[{"name": "...", "cards": ["..."]}]`.
    #[arg(long)]
    combos_file: Option<PathBuf>,

    /// Emit the full structured result as JSON.
    #[arg(long)]
    json: bool,

    /// Do not write the verdict tracker.
    #[arg(long)]
    no_write: bool,
}

fn parse_combo_arg(s: &str) -> Result<Combo, String> {
    let (name, cards) = s
        .split_once('=')
        .ok_or_else(|| format!("combo '{s}' must be name=CARD1,CARD2"))?;
    let cards: Vec<String> = cards
        .split(',')
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .collect();
    if cards.is_empty() {
        return Err(format!("combo '{name}' lists no cards"));
    }
    Ok(Combo { name: name.trim().to_string(), cards })
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse();
    let root = repo_root()?;

    let card_data = load_card_data(&root)?;
    let deck_library = load_json(&root.join("data").join("deck_library.json"))?;
    let aliases = load_json(&root.join("data").join("archetype_aliases.json"))?;
    let validated = load_json(&root.join("qa").join("qa-reports").join("validated_cards_dsl.json"))?;

    let pool = resolve_pool(&args.archetype, &deck_library, &aliases)?;

    // Assemble combos from --combo flags + --combos-file.
    let mut combos: Vec<Combo> = Vec::new();
    for c in &args.combos {
        combos.push(parse_combo_arg(c)?);
    }
    if let Some(path) = &args.combos_file {
        let v = load_json(path)?;
        let arr = v.as_array().ok_or("combos-file must be a JSON array")?;
        for entry in arr {
            let name = entry.get("name").and_then(|n| n.as_str()).unwrap_or("(unnamed)").to_string();
            let cards: Vec<String> = entry
                .get("cards")
                .and_then(|c| c.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            combos.push(Combo { name, cards });
        }
    }

    let results = vec![
        check_deck_legality(&pool.decklist, &card_data),
        coverage_gate(&pool.unique_cards, &validated, args.threshold),
        smoke_games(&pool.decklist, args.smoke_games, args.max_steps, &card_data),
        combo_presence(&combos, &implemented_set(&validated)),
    ];

    let combos_tested: Vec<String> = combos.iter().map(|c| c.name.clone()).collect();
    let timestamp = today_utc();

    if args.json {
        let out = json!({
            "archetype": pool.name,
            "unique_cards": pool.unique_cards.len(),
            "results": results,
            "combos_tested": combos_tested,
        });
        println!("{}", serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?);
    } else {
        println!("archetype: {} ({} unique cards)", pool.name, pool.unique_cards.len());
        for r in &results {
            println!("  [{}] {:<16} {}", mark(r), r.name, r.detail);
        }
    }

    if !args.no_write {
        let path = tracker_path(&root);
        let existing = if path.is_file() { Some(load_json(&path)?) } else { None };
        let doc = record_run(existing, &pool.name, &results, &combos_tested, &[], &timestamp);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?)
            .map_err(|e| format!("writing {}: {}", path.display(), e))?;
        if !args.json {
            println!("recorded → {}", path.display());
        }
    }

    let all_passed = results.iter().all(|r| r.passed);
    if !all_passed {
        std::process::exit(2);
    }
    Ok(())
}

fn mark(r: &InvariantResult) -> &'static str {
    if r.passed {
        "PASS"
    } else {
        "FAIL"
    }
}

/// Current UTC date as `YYYY-MM-DD` (no chrono dependency).
fn today_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, m, d) = civil_from_days((secs / 86_400) as i64);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

/// Howard Hinnant's civil-from-days: days since 1970-01-01 → (year, month, day).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
