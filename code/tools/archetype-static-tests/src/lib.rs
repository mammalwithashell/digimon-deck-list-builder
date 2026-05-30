//! Static archetype-level invariant checks for the digimon engine.
//!
//! Four invariants, each runnable independently of the authoring skill
//! (CI-able) and emitting a structured [`InvariantResult`]:
//!
//! 1. [`check_deck_legality`] — the archetype's meta decklist builds into a
//!    rules-legal deck and the engine constructs a game from it.
//! 2. [`coverage_gate`] — the archetype's unique-card pool is implemented (per
//!    `validated_cards_dsl.json`) at or above a threshold; "unknown"-status
//!    cards are reported, never counted as passing.
//! 3. [`smoke_games`] — N self-play games on the deck run to completion without
//!    a panic / illegal state (a *liveness* gate, NOT a correctness check).
//! 4. [`combo_presence`] — every card named in the model's combos is
//!    implemented, so interaction tests are writable.
//!
//! Results are recorded per-archetype in
//! `qa/qa-reports/archetype_interactions.json` via [`record_run`].
//!
//! This is the harness behind the `/archetype-interaction-test-author` skill's
//! precondition gating, but it stands alone (the CLI in `main.rs` runs all four
//! for an archetype name).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use digimon_engine::card_data::CardData;
use digimon_engine::enums::CardKind;
use digimon_engine::{build_registry, Game, HeadlessRunner, Rules};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Structured outcome of one static invariant check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantResult {
    /// `"deck_legality"` | `"coverage_gate"` | `"smoke_games"` | `"combo_presence"`.
    pub name: &'static str,
    pub passed: bool,
    /// One-line human summary.
    pub detail: String,
    /// Structured per-invariant details (violations, ratios, missing cards…).
    pub data: Value,
}

impl InvariantResult {
    fn pass(name: &'static str, detail: impl Into<String>, data: Value) -> Self {
        Self { name, passed: true, detail: detail.into(), data }
    }
    fn fail(name: &'static str, detail: impl Into<String>, data: Value) -> Self {
        Self { name, passed: false, detail: detail.into(), data }
    }
}

/// A named combo from an archetype-model — a set of card IDs that must all be
/// implemented for the combo's interaction test to be writable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Combo {
    pub name: String,
    pub cards: Vec<String>,
}

/// A resolved archetype: a representative decklist (for legality + smoke) and
/// the deduplicated unique-card pool (for the coverage gate).
#[derive(Debug, Clone)]
pub struct ArchetypePool {
    pub name: String,
    pub decklist: Vec<String>,
    pub unique_cards: Vec<String>,
}

// ── repo / data file resolution ──────────────────────────────────────────────

/// Walk up from `start` until a directory containing `data/cards.json` is
/// found. Lets the CLI run from anywhere in the tree (and tests from the crate
/// dir).
pub fn find_repo_root(start: &Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        if dir.join("data").join("cards.json").is_file() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Best-effort repo root: from `CARGO_MANIFEST_DIR`, else the current dir.
pub fn repo_root() -> Result<PathBuf, String> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if let Some(r) = find_repo_root(&manifest) {
        return Ok(r);
    }
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    find_repo_root(&cwd).ok_or_else(|| "could not locate repo root (no data/cards.json found)".into())
}

pub fn load_json(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("reading {}: {}", path.display(), e))?;
    serde_json::from_str(&text).map_err(|e| format!("parsing {}: {}", path.display(), e))
}

pub fn load_card_data(root: &Path) -> Result<HashMap<String, CardData>, String> {
    let text = std::fs::read_to_string(root.join("data").join("cards.json"))
        .map_err(|e| format!("reading cards.json: {}", e))?;
    CardData::load_from_str(&text)
}

// ── archetype resolution (deck_library.json + archetype_aliases.json) ────────

/// Canonicalize an archetype name through `archetype_aliases.json`
/// (case-insensitive): if `name` matches an alias, return the canonical key;
/// otherwise return `name` unchanged.
pub fn canonicalize(name: &str, aliases: &Value) -> String {
    let lname = name.to_lowercase();
    if let Some(map) = aliases.as_object() {
        for (canonical, alts) in map {
            if canonical == "_comment" {
                continue;
            }
            if canonical.to_lowercase() == lname {
                return canonical.clone();
            }
            if let Some(arr) = alts.as_array() {
                if arr.iter().filter_map(|a| a.as_str()).any(|a| a.to_lowercase() == lname) {
                    return canonical.clone();
                }
            }
        }
    }
    name.to_string()
}

/// Parse a deck_library decklist entry (`decklist` is a JSON-string array).
fn parse_decklist(entry: &Value) -> Option<Vec<String>> {
    let raw = entry.get("decklist")?.as_str()?;
    let arr: Vec<String> = serde_json::from_str(raw).ok()?;
    Some(arr)
}

/// Resolve an archetype name to a representative decklist + unique-card pool.
/// The representative decklist prefers a top-cut / best-placement list; the
/// unique pool is deduped across all the archetype's decklists.
pub fn resolve_pool(name: &str, deck_library: &Value, aliases: &Value) -> Result<ArchetypePool, String> {
    let canonical = canonicalize(name, aliases);
    let archetypes = deck_library
        .get("archetypes")
        .and_then(|v| v.as_object())
        .ok_or("deck_library.json has no `archetypes` object")?;

    // Case-insensitive match on the archetype key.
    let entry = archetypes
        .iter()
        .find(|(k, _)| k.to_lowercase() == canonical.to_lowercase())
        .map(|(_, v)| v)
        .ok_or_else(|| format!("archetype '{}' (canonical '{}') not in deck_library.json", name, canonical))?;

    let decklists = entry
        .get("decklists")
        .and_then(|v| v.as_array())
        .ok_or_else(|| format!("archetype '{}' has no decklists", canonical))?;
    if decklists.is_empty() {
        return Err(format!("archetype '{}' has an empty decklists array", canonical));
    }

    // Representative: prefer a top-cut list, else the first.
    let representative = decklists
        .iter()
        .find(|d| d.get("is_top_cut").and_then(|v| v.as_bool()).unwrap_or(false))
        .or_else(|| decklists.first())
        .and_then(parse_decklist)
        .ok_or_else(|| format!("archetype '{}' representative decklist failed to parse", canonical))?;

    // Unique pool: dedupe across all decklists (preserve first-seen order).
    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    for d in decklists {
        if let Some(cards) = parse_decklist(d) {
            for c in cards {
                if seen.insert(c.clone()) {
                    unique.push(c);
                }
            }
        }
    }

    Ok(ArchetypePool { name: canonical, decklist: representative, unique_cards: unique })
}

// ── invariant 1: deck legality + construction ────────────────────────────────

/// Validate a decklist against the standard deck-construction rules (50-card
/// main deck, 0–5 digi-egg deck, ≤4 copies of any card number, all cards in the
/// pool) and confirm the engine constructs a game from it. Reports the specific
/// violation on failure.
pub fn check_deck_legality(decklist: &[String], card_data: &HashMap<String, CardData>) -> InvariantResult {
    const NAME: &str = "deck_legality";
    let mut violations: Vec<String> = Vec::new();

    // Unknown cards (not in the pool).
    let unknown: Vec<String> = decklist
        .iter()
        .filter(|id| !card_data.contains_key(*id))
        .cloned()
        .collect();
    if !unknown.is_empty() {
        let mut uniq: Vec<String> = unknown.iter().cloned().collect::<HashSet<_>>().into_iter().collect();
        uniq.sort();
        violations.push(format!("cards not in pool: {:?}", uniq));
    }

    // Split by kind (only for cards we know).
    let mut main_count = 0usize;
    let mut egg_count = 0usize;
    for id in decklist {
        if let Some(cd) = card_data.get(id) {
            if cd.card_kind == CardKind::DigiEgg {
                egg_count += 1;
            } else {
                main_count += 1;
            }
        }
    }
    if main_count != 50 {
        violations.push(format!("main deck must be 50 cards, found {}", main_count));
    }
    if egg_count > 5 {
        violations.push(format!("digi-egg deck must be 0–5 cards, found {}", egg_count));
    }

    // Copy limit (≤4 of any single card number).
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for id in decklist {
        *counts.entry(id.as_str()).or_insert(0) += 1;
    }
    let over: Vec<String> = counts
        .iter()
        .filter(|(_, n)| **n > 4)
        .map(|(id, n)| format!("{}×{}", id, n))
        .collect();
    if !over.is_empty() {
        violations.push(format!("over the 4-copy limit: {:?}", over));
    }

    // Engine construction (mirror match — both players use the deck).
    let decks = vec![decklist.to_vec(), decklist.to_vec()];
    let construct_err = match Game::new(&decks, card_data, Rules::standard(), Some(0)) {
        Ok(_) => None,
        Err(e) => Some(e),
    };
    if let Some(e) = &construct_err {
        violations.push(format!("engine construction failed: {}", e));
    }

    let data = json!({
        "main_count": main_count,
        "egg_count": egg_count,
        "unknown_cards": unknown,
        "over_copy_limit": over,
        "constructs": construct_err.is_none(),
        "violations": violations,
    });

    if violations.is_empty() {
        InvariantResult::pass(NAME, "deck is rules-legal and constructs a game", data)
    } else {
        InvariantResult::fail(NAME, format!("{} violation(s)", violations.len()), data)
    }
}

// ── invariant 2: coverage gate ───────────────────────────────────────────────

/// Statuses in `validated_cards_dsl.json` that count as a passing per-card
/// implementation.
fn is_passing_status(status: &str) -> bool {
    matches!(status, "IMPLEMENTED" | "PASS")
}

/// Cross-reference the unique-card pool against `validated_cards_dsl.json`.
/// Implemented = status IMPLEMENTED/PASS; absent from the tracker = "unknown"
/// (reported, never counted as passing); any other status = "failing". Passes
/// iff the implemented ratio ≥ `threshold` and nothing is unknown/failing
/// below it.
pub fn coverage_gate(unique_cards: &[String], validated: &Value, threshold: f64) -> InvariantResult {
    const NAME: &str = "coverage_gate";
    let cards = validated.get("cards").and_then(|v| v.as_object());

    let mut implemented = Vec::new();
    let mut failing = Vec::new();
    let mut unknown = Vec::new();
    for id in unique_cards {
        let status = cards
            .and_then(|c| c.get(id))
            .and_then(|e| e.get("status"))
            .and_then(|s| s.as_str());
        match status {
            Some(s) if is_passing_status(s) => implemented.push(id.clone()),
            Some(_) => failing.push(id.clone()),
            None => unknown.push(id.clone()),
        }
    }

    let total = unique_cards.len().max(1);
    let ratio = implemented.len() as f64 / total as f64;
    // Unknown / failing cards are simply absent from `implemented`, so they
    // drag the ratio down — they are never *counted as passing*. The pass
    // condition is the configurable threshold on that ratio (default 1.0 =
    // every card implemented). Failing/unknown are reported in `data`.
    let passed = ratio >= threshold;

    let data = json!({
        "total": unique_cards.len(),
        "implemented": implemented.len(),
        "ratio": ratio,
        "threshold": threshold,
        "failing_cards": failing,
        "unknown_cards": unknown,
    });
    let detail = format!(
        "coverage {}/{} = {:.0}% (threshold {:.0}%); {} failing, {} unknown",
        implemented.len(),
        unique_cards.len(),
        ratio * 100.0,
        threshold * 100.0,
        failing.len(),
        unknown.len()
    );
    if passed {
        InvariantResult::pass(NAME, detail, data)
    } else {
        InvariantResult::fail(NAME, detail, data)
    }
}

// ── invariant 3: per-archetype smoke games ───────────────────────────────────

/// Play `n` self-play games on `decklist` (mirror match), each with a distinct
/// seed and a random-legal policy, asserting none panic or hit an illegal
/// state. Liveness gate only. Reports the seed (and panic message) of the first
/// failing game.
pub fn smoke_games(
    decklist: &[String],
    n: u32,
    max_steps: u32,
    card_data: &HashMap<String, CardData>,
) -> InvariantResult {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    use std::panic::{catch_unwind, AssertUnwindSafe};

    const NAME: &str = "smoke_games";

    // Silence panic backtraces during the catch_unwind sweep so a smoke panic
    // doesn't spam the harness output; restore afterward.
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    let mut completed = 0u32;
    let mut failure: Option<(u64, String)> = None;
    for seed in 0..n as u64 {
        let result = catch_unwind(AssertUnwindSafe(|| -> Result<(), String> {
            let mut runner =
                HeadlessRunner::new(decklist.to_vec(), decklist.to_vec(), card_data, false, false, false, Some(seed))?;
            let mut rng = StdRng::seed_from_u64(seed ^ 0x9E37_79B9_7F4A_7C15);
            runner.run_until_conclusion(
                max_steps,
                Some(|_g: &Game, mask: &[f32]| -> u16 {
                    let legal: Vec<u16> = mask
                        .iter()
                        .enumerate()
                        .filter(|(_, b)| **b > 0.5)
                        .map(|(i, _)| i as u16)
                        .collect();
                    if legal.is_empty() {
                        0
                    } else {
                        legal[rng.gen_range(0..legal.len())]
                    }
                }),
            );
            Ok(())
        }));
        match result {
            Ok(Ok(())) => completed += 1,
            Ok(Err(e)) => {
                failure = Some((seed, format!("construction error: {}", e)));
                break;
            }
            Err(panic) => {
                let msg = panic
                    .downcast_ref::<&str>()
                    .map(|s| s.to_string())
                    .or_else(|| panic.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "unknown panic".to_string());
                failure = Some((seed, format!("panic: {}", msg)));
                break;
            }
        }
    }

    std::panic::set_hook(prev_hook);

    match failure {
        None => InvariantResult::pass(
            NAME,
            format!("{}/{} smoke games completed cleanly", completed, n),
            json!({ "games": n, "completed": completed }),
        ),
        Some((seed, msg)) => InvariantResult::fail(
            NAME,
            format!("smoke game seed {} failed: {}", seed, msg),
            json!({ "games": n, "completed": completed, "failed_seed": seed, "error": msg }),
        ),
    }
}

// ── invariant 4: combo presence ──────────────────────────────────────────────

/// The set of card IDs the engine considers implemented for combo-presence:
/// any card whose status in `validated_cards_dsl.json` is IMPLEMENTED/PASS,
/// unioned with the engine's registered-effect card IDs (so a card with a
/// registered `CardEffect` but no tracker entry still counts).
pub fn implemented_set(validated: &Value) -> HashSet<String> {
    let mut set: HashSet<String> = build_registry().registered_card_ids().into_iter().collect();
    if let Some(cards) = validated.get("cards").and_then(|v| v.as_object()) {
        for (id, entry) in cards {
            if entry.get("status").and_then(|s| s.as_str()).map(is_passing_status).unwrap_or(false) {
                set.insert(id.clone());
            }
        }
    }
    set
}

/// Assert every card named in each combo is implemented. A combo with a missing
/// piece is reported as blocked on the specific missing card(s).
pub fn combo_presence(combos: &[Combo], implemented: &HashSet<String>) -> InvariantResult {
    const NAME: &str = "combo_presence";
    let mut blocked = Vec::new();
    for combo in combos {
        let missing: Vec<String> = combo
            .cards
            .iter()
            .filter(|c| !implemented.contains(*c))
            .cloned()
            .collect();
        if !missing.is_empty() {
            blocked.push(json!({ "combo": combo.name, "missing": missing }));
        }
    }

    let data = json!({ "combos_total": combos.len(), "blocked": blocked });
    if blocked.is_empty() {
        InvariantResult::pass(
            NAME,
            format!("all {} combo(s) have every piece implemented", combos.len()),
            data,
        )
    } else {
        InvariantResult::fail(
            NAME,
            format!("{}/{} combo(s) blocked on a missing card", blocked.len(), combos.len()),
            data,
        )
    }
}

// ── verdict tracker (qa/qa-reports/archetype_interactions.json) ───────────────

/// Path to the verdict tracker under `root`.
pub fn tracker_path(root: &Path) -> PathBuf {
    root.join("qa").join("qa-reports").join("archetype_interactions.json")
}

/// Record a run's results into the tracker JSON (created if absent), keyed by
/// archetype. `timestamp` is the caller's responsibility (the CLI stamps the
/// current date; tests pass a fixed string). Returns the updated document.
pub fn record_run(
    existing: Option<Value>,
    archetype: &str,
    results: &[InvariantResult],
    combos_tested: &[String],
    findings: &[String],
    timestamp: &str,
) -> Value {
    let mut doc = existing.unwrap_or_else(|| {
        json!({ "version": 1, "last_updated": timestamp, "archetypes": {} })
    });
    if !doc.is_object() {
        doc = json!({ "version": 1, "last_updated": timestamp, "archetypes": {} });
    }
    doc["last_updated"] = json!(timestamp);
    if !doc.get("archetypes").map(|a| a.is_object()).unwrap_or(false) {
        doc["archetypes"] = json!({});
    }

    let static_tests: serde_json::Map<String, Value> = results
        .iter()
        .map(|r| {
            (
                r.name.to_string(),
                json!({ "passed": r.passed, "detail": r.detail, "data": r.data }),
            )
        })
        .collect();

    doc["archetypes"][archetype] = json!({
        "updated": timestamp,
        "static_tests": Value::Object(static_tests),
        "combos_tested": combos_tested,
        "findings": findings,
    });
    doc
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Repo root for the real-data tests, or `None` (skip) if not found.
    fn real_root() -> Option<PathBuf> {
        find_repo_root(&PathBuf::from(env!("CARGO_MANIFEST_DIR")))
    }

    #[test]
    fn deck_legality_real_rocks_deck_passes_and_constructs() {
        let Some(root) = real_root() else { return };
        let card_data = load_card_data(&root).expect("cards.json loads");
        let deck_library = load_json(&root.join("data").join("deck_library.json")).expect("deck_library");
        let aliases = load_json(&root.join("data").join("archetype_aliases.json")).expect("aliases");
        let pool = resolve_pool("Rocks", &deck_library, &aliases).expect("Rocks resolves");
        let r = check_deck_legality(&pool.decklist, &card_data);
        assert!(
            r.passed,
            "a real Rocks meta deck must be rules-legal and construct: {} ({})",
            r.detail, r.data
        );
        assert_eq!(r.data["main_count"], 50);
    }

    #[test]
    fn deck_legality_short_deck_reports_size_violation() {
        let Some(root) = real_root() else { return };
        let card_data = load_card_data(&root).expect("cards.json loads");
        let deck_library = load_json(&root.join("data").join("deck_library.json")).expect("deck_library");
        let aliases = load_json(&root.join("data").join("archetype_aliases.json")).expect("aliases");
        let pool = resolve_pool("Rocks", &deck_library, &aliases).expect("Rocks resolves");
        // Truncate to an illegal 10-card list → main deck != 50.
        let short: Vec<String> = pool.decklist.iter().take(10).cloned().collect();
        let r = check_deck_legality(&short, &card_data);
        assert!(!r.passed, "a 10-card deck must fail deck-legality");
        let violations = r.data["violations"].as_array().unwrap();
        assert!(
            violations.iter().any(|v| v.as_str().unwrap().contains("main deck must be 50")),
            "must name the size violation; got {:?}",
            violations
        );
    }

    fn aliases() -> Value {
        json!({
            "_comment": "test",
            "TS Olympos": ["TS Olymp", "Olympos"],
        })
    }

    fn validated(statuses: &[(&str, &str)]) -> Value {
        let mut cards = serde_json::Map::new();
        for (id, status) in statuses {
            cards.insert(id.to_string(), json!({ "status": status }));
        }
        json!({ "cards": cards })
    }

    #[test]
    fn canonicalize_maps_alias_case_insensitively() {
        assert_eq!(canonicalize("olympos", &aliases()), "TS Olympos");
        assert_eq!(canonicalize("TS Olympos", &aliases()), "TS Olympos");
        // Unknown name passes through unchanged.
        assert_eq!(canonicalize("Rocks", &aliases()), "Rocks");
    }

    #[test]
    fn coverage_gate_full_implementation_passes() {
        let pool = vec!["A".to_string(), "B".to_string()];
        let v = validated(&[("A", "IMPLEMENTED"), ("B", "PASS")]);
        let r = coverage_gate(&pool, &v, 1.0);
        assert!(r.passed, "fully-implemented pool must pass: {}", r.detail);
        assert_eq!(r.data["implemented"], 2);
    }

    #[test]
    fn coverage_gate_sub_threshold_reports_the_gap() {
        let pool = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        // A implemented, B failing, C unknown (absent).
        let v = validated(&[("A", "IMPLEMENTED"), ("B", "QA-FAIL")]);
        let r = coverage_gate(&pool, &v, 1.0);
        assert!(!r.passed, "sub-threshold pool must not pass");
        assert_eq!(r.data["failing_cards"], json!(["B"]));
        assert_eq!(r.data["unknown_cards"], json!(["C"]));
    }

    #[test]
    fn coverage_gate_unknown_card_is_not_counted_as_passing() {
        let pool = vec!["A".to_string()];
        let v = validated(&[]); // A is absent → unknown
        let r = coverage_gate(&pool, &v, 0.5);
        assert!(!r.passed, "an all-unknown pool must not pass");
        assert_eq!(r.data["implemented"], 0);
        assert_eq!(r.data["unknown_cards"], json!(["A"]));
    }

    #[test]
    fn combo_presence_all_present_passes() {
        let mut impl_set = HashSet::new();
        impl_set.insert("X".to_string());
        impl_set.insert("Y".to_string());
        let combos = vec![Combo { name: "c1".into(), cards: vec!["X".into(), "Y".into()] }];
        let r = combo_presence(&combos, &impl_set);
        assert!(r.passed);
    }

    #[test]
    fn combo_presence_missing_piece_blocks_the_combo() {
        let mut impl_set = HashSet::new();
        impl_set.insert("X".to_string());
        let combos = vec![Combo {
            name: "c1".into(),
            cards: vec!["X".into(), "MISSING".into()],
        }];
        let r = combo_presence(&combos, &impl_set);
        assert!(!r.passed);
        assert_eq!(r.data["blocked"][0]["combo"], "c1");
        assert_eq!(r.data["blocked"][0]["missing"], json!(["MISSING"]));
    }

    #[test]
    fn record_run_keys_by_archetype_and_stamps_timestamp() {
        let results = vec![InvariantResult::pass("coverage_gate", "ok", json!({}))];
        let doc = record_run(None, "Rocks", &results, &["combo1".into()], &[], "2026-05-29");
        assert_eq!(doc["last_updated"], "2026-05-29");
        assert_eq!(doc["archetypes"]["Rocks"]["static_tests"]["coverage_gate"]["passed"], true);
        assert_eq!(doc["archetypes"]["Rocks"]["combos_tested"], json!(["combo1"]));

        // A second archetype merges without clobbering the first.
        let doc2 = record_run(Some(doc), "Puppets", &results, &[], &[], "2026-05-30");
        assert!(doc2["archetypes"]["Rocks"].is_object());
        assert!(doc2["archetypes"]["Puppets"].is_object());
        assert_eq!(doc2["last_updated"], "2026-05-30");
    }
}
