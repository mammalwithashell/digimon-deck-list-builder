//! Rust headless runner for the shared scenario corpus
//! (add-ui-scenario-test-substrate, group 6).
//!
//! Reads every `qa/scenarios/*.json` fixture, stages a `DebugRunner` from
//! it using the REAL card pool (`data/cards.json`), and evaluates the
//! fixture's `engine` assertions. This is the headless half of the
//! two-layer conformance harness — the Playwright half stages the same
//! fixtures over the `/debug` HTTP router and evaluates via the server.
//!
//! Readiness tags:
//! - `expected_pass`  — every engine assertion must pass; a failure fails
//!   the test (a real regression).
//! - `blocked_on_card_impl` — depends on unimplemented card behavior;
//!   reported as PENDING (printed), never fails the suite. If one starts
//!   passing, flip its tag.
//!
//! Note: the DebugRunner test path registers hand-written Rust effects but
//! not the full DSL pack, so resolution-heavy assertions (effect_triggered)
//! belong on `blocked_on_card_impl` fixtures or the server/Playwright path.
//! The `expected_pass` corpus asserts staging-level facts (memory, stack
//! top, base/effective DP, zone counts, action legality).

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::GamePhase;
use serde_json::Value;

const CARDS_JSON: &str = include_str!("../../../data/cards.json");

fn full_card_db() -> HashMap<String, CardData> {
    CardData::load_from_str(CARDS_JSON).expect("parse data/cards.json")
}

fn scenarios_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../qa/scenarios")
}

fn parse_phase(s: &str) -> GamePhase {
    match s {
        "Mulligan" => GamePhase::Mulligan,
        "Unsuspend" => GamePhase::Unsuspend,
        "Draw" => GamePhase::Draw,
        "Breeding" => GamePhase::Breeding,
        "Main" => GamePhase::Main,
        "EndTurn" => GamePhase::EndTurn,
        other => panic!("unstageable phase {other}"),
    }
}

fn str_list(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .map(|x| x.as_str().unwrap().to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// Stage a `DebugRunner` from a fixture's `decks` / `state` / `zones`.
fn stage(fixture: &Value, db: HashMap<String, CardData>) -> DebugRunner {
    let decks = &fixture["decks"];
    let deck1 = str_list(&decks["1"]);
    let deck2 = str_list(&decks["2"]);

    // Build with the real DB. DebugRunner::builder seeds hands/decks; we
    // override zones below, so just seed the decks for registry/draw pile.
    let mut builder = DebugRunner::builder().with_card_data(db);
    builder = builder.deck(0, &deck1.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    builder = builder.deck(1, &deck2.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    let mut r = builder.start();
    r.skip_mulligan();

    // Per-player zones (clear-then-populate, mirroring the HTTP loader).
    if let Some(zones) = fixture["zones"].as_object() {
        for (pid_str, zone) in zones {
            let pid: u8 = pid_str.parse::<u8>().unwrap() - 1; // 1/2 -> 0/1
            if let Some(hand) = zone.get("hand") {
                r.game.stage_clear_zone(pid, "hand").unwrap();
                for cid in str_list(hand) {
                    r.game.stage_inject_card(pid, &cid, "hand").unwrap();
                }
            }
            if let Some(trash) = zone.get("trash") {
                r.game.stage_clear_zone(pid, "trash").unwrap();
                for cid in str_list(trash) {
                    r.game.stage_inject_card(pid, &cid, "trash").unwrap();
                }
            }
            if let Some(field) = zone.get("field").and_then(|f| f.as_array()) {
                r.game.stage_clear_zone(pid, "battle_area").unwrap();
                for fs in field {
                    let stack = str_list(&fs["stack"]);
                    let refs: Vec<&str> = stack.iter().map(|s| s.as_str()).collect();
                    let suspended = fs["is_suspended"].as_bool().unwrap_or(false);
                    let turn_played = fs["turn_played"].as_u64().unwrap_or(0) as u16;
                    r.game
                        .stage_place_field_stack(pid, &refs, suspended, turn_played);
                }
            }
            if let Some(breeding) = zone.get("breeding") {
                let stack = str_list(breeding);
                if !stack.is_empty() {
                    r.game.stage_clear_zone(pid, "breeding").unwrap();
                    let refs: Vec<&str> = stack.iter().map(|s| s.as_str()).collect();
                    r.game.stage_place_in_breeding(pid, &refs);
                }
            }
        }
    }

    // Scalar state.
    let state = &fixture["state"];
    if let Some(fp) = state["first_player"].as_u64() {
        r.set_first_player((fp as u8) - 1);
    }
    if let Some(turn) = state["turn"].as_u64() {
        r.set_turn(turn as u16);
    }
    if let Some(phase) = state["phase"].as_str() {
        r.set_phase(parse_phase(phase));
    }
    if let Some(mem) = state["memory"].as_i64() {
        r.game.set_memory(mem as i16);
    }

    r.validate().expect("staged board must validate");
    r
}

/// Evaluate one engine assertion. Returns (passed, message).
fn evaluate(r: &DebugRunner, a: &Value) -> (bool, String) {
    let kind = a["kind"].as_str().unwrap();
    match kind {
        "memory_equals" => {
            let want = a["value"].as_i64().unwrap() as i16;
            let got = r.memory();
            (got == want, format!("memory expected {want}, got {got}"))
        }
        "stack_top" => {
            let pid = a["player"].as_u64().unwrap() as u8 - 1;
            let fi = a["field_index"].as_u64().unwrap() as usize;
            let want = a["card_id"].as_str().unwrap();
            let ba = &r.game.player(pid).battle_area;
            if fi >= ba.len() {
                return (false, format!("field_index {fi} out of range"));
            }
            let top_data = ba[fi].top_card().data_index;
            let got = &r.game.card_data[top_data].card_id;
            (got == want, format!("stack_top expected {want}, got {got}"))
        }
        "effective_dp" => {
            let pid = a["player"].as_u64().unwrap() as u8 - 1;
            let fi = a["field_index"].as_u64().unwrap() as usize;
            let want = a["value"].as_i64().unwrap() as i32;
            let handle = r.perm_handle(pid, fi);
            let got = r.effective_dp(handle).unwrap_or(i32::MIN);
            (got == want, format!("effective_dp expected {want}, got {got}"))
        }
        "zone_count" => {
            let pid = a["player"].as_u64().unwrap() as u8 - 1;
            let zone = a["zone"].as_str().unwrap();
            let want = a["value"].as_u64().unwrap() as usize;
            let p = r.game.player(pid);
            let got = match zone {
                "hand" => p.hand.len(),
                "deck" => p.deck.len(),
                "security" => p.security.len(),
                "trash" => p.trash.len(),
                other => return (false, format!("unknown zone {other}")),
            };
            (got == want, format!("{zone} count expected {want}, got {got}"))
        }
        "action_legal" => {
            let aid = a["action_id"].as_u64().unwrap() as usize;
            let want = a["expected"].as_bool().unwrap_or(true);
            let mask = digimon_engine::action::mask::build_action_mask(
                &r.game,
                r.game.turn_player(),
            );
            let legal = aid < mask.len() && mask[aid] == 1.0;
            (legal == want, format!("action {aid} legal={legal}, want {want}"))
        }
        // effect_triggered / legal_selection_options need full resolution /
        // DSL effects; not evaluated in the headless test path (those live
        // on blocked fixtures or the server path). Treat as inconclusive.
        other => (false, format!("assertion kind {other} not supported in headless runner")),
    }
}

#[test]
fn scenario_corpus_runs() {
    let dir = scenarios_dir();
    assert!(dir.exists(), "qa/scenarios not found at {dir:?}");

    let mut ran = 0;
    let mut pending = 0;
    let mut failures: Vec<String> = Vec::new();

    for entry in fs::read_dir(&dir).expect("read qa/scenarios") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let raw = fs::read_to_string(&path).unwrap();
        let fixture: Value = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("parse {path:?}: {e}"));
        let id = fixture["id"].as_str().unwrap_or("<no-id>").to_string();
        let readiness = fixture["readiness"].as_str().unwrap_or("expected_pass");

        let r = stage(&fixture, full_card_db());
        let assertions = fixture["assertions"]["engine"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        let mut all_pass = true;
        let mut msgs = Vec::new();
        for a in &assertions {
            let (passed, msg) = evaluate(&r, a);
            if !passed {
                all_pass = false;
                msgs.push(msg);
            }
        }

        if readiness == "blocked_on_card_impl" {
            pending += 1;
            println!("PENDING (blocked): {id}");
            continue;
        }

        ran += 1;
        if all_pass {
            println!("PASS: {id}");
        } else {
            failures.push(format!("{id}: {}", msgs.join("; ")));
        }
    }

    println!("scenario corpus: {ran} expected-pass run, {pending} pending");
    assert!(
        failures.is_empty(),
        "expected-pass scenarios failed:\n{}",
        failures.join("\n")
    );
}
