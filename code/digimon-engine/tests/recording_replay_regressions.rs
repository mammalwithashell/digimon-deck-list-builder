//! Replay-based regression tests for engine panics that were caught in
//! production training runs and captured as recordings under
//! `tests/recordings/`. Each fixture's filename matches the panic-family
//! identifier in `qa/archetype-qa/panic-families.json`.
//!
//! The pattern: load the captured `recording` JSON (the inner object the
//! recorder produces, without the outer `outcome` / `run` wrapper), seek to
//! the end with the full `cards.json` database, and assert the replay
//! completes without panicking. Pre-fix, this would surface the original
//! `expect(...)` panic; post-fix, the soft-fail (or the appropriate
//! invariant repair) carries the replay through.
//!
//! Adding a fixture:
//!   1. Capture the inner recording: `python -c "import json,pathlib; d=json.load(open(SRC)); pathlib.Path('code/digimon-engine/tests/recordings/<family>.json').write_text(json.dumps(d['recording']))"`
//!   2. Add a `#[test]` here that loads the fixture and asserts replay-to-end is Ok.
//!   3. Cross-link in the panic-families.json entry's `regression_test` field
//!      (when that schema field lands).

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use digimon_engine::card_data::CardData;
use digimon_engine::runners::replay::ReplayRunner;

/// Load the full repo-root `data/cards.json`. Pattern mirrors
/// `tests/dsl/real_cards_json.rs`: `CARGO_MANIFEST_DIR` is `code/digimon-engine`,
/// so `cards.json` lives two levels up under `data/`.
fn load_full_card_data() -> HashMap<String, CardData> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("data/cards.json");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("must be able to read {}: {}", path.display(), e));
    CardData::load_from_str(&text)
        .unwrap_or_else(|e| panic!("must be able to parse cards.json: {}", e))
}

/// Load a recording fixture by file name under `tests/recordings/`.
fn load_recording(name: &str) -> serde_json::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("recordings")
        .join(name);
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("must be able to read {}: {}", path.display(), e));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("must be able to parse {}: {}", name, e))
}

/// Regression for `G-DSL-TRASH-SOURCES-STALE-HANDLE`
/// (`fix-trash-card-source-stale-handle`).
///
/// Captured from production training: `generalist_1m_v2`, game 9728, recorder
/// action 87 — TS Olympos (Magneticdramon-Mineral/Rock) vs Yellow-Tamer-heavy.
/// Turn 10 main with parallel EX10-033 + EX10-032 [WD] triggers; the
/// SourceMulti picker for EX10-032 Clause 2 snapshotted a candidate from a
/// stack that was emptied by EX10-033 Clause B's earlier trash, then the
/// player's submit drove `trash_card_source` into its `expect("card not in
/// this permanent's stack")` panic.
///
/// Post-fix expectation: the picker's submit callback re-validates the
/// pick against the live `card_sources`, refuses the stale ref, and
/// re-installs with refreshed (now empty) candidates; the empty-candidates
/// + `picked.len() >= min` branch final-callbacks with no picks; the DSL
/// `TrashSelectedSources` loop's soft-fail path no-ops. The replay seeks
/// past the original panic step without erroring.
#[test]
fn replay_g_dsl_trash_sources_stale_handle_does_not_panic() {
    let recording = load_recording("g_dsl_trash_sources_stale_handle.json");
    let db = load_full_card_data();

    let mut replay = ReplayRunner::new(recording, &db, /*verify=*/ false)
        .expect("constructs from g_dsl_trash_sources_stale_handle fixture");

    let total = replay.total_steps();
    assert!(
        total > 0,
        "fixture must contain replayable steps (recorder captured 87 actions; 2 mulligan filter out, ~85 expected)"
    );

    // Pre-fix: this panics inside `trash_card_source` somewhere mid-seek.
    // Post-fix: clean Ok all the way to the end.
    replay
        .seek(total)
        .expect("replay must complete without panic after the soft-fail fix");
}
