//! Integration tests for Phase 1A & 1B `digimon-engine-mcp` tools:
//! - `inspect_stack` (top effect_text, inherited-only under-cards, link-only linked cards, face-down redaction)
//! - `load_puzzle` + `Play`-mode `Perspective` lock + `opponent_driver` (`greedy` & `onnx`)
//! - `save_puzzle` (`studio-puzzle-v1` + `search-exam-v1` export & round-trip)
//! - `verify_puzzle` on `ex12-erlangmon-saneiketsu-invitation-trap` (both schemas)
//! - `load_model`, `list_models`, `inspect_position`, `score_line`, `unload_model`

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{json, Value};

fn repo_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..6 {
        if dir.join("data").join("cards.json").exists() {
            return dir;
        }
        if !dir.pop() {
            break;
        }
    }
    panic!("could not locate repo root containing data/cards.json");
}

fn bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_digimon-engine-mcp"))
}

struct TempDirGuard {
    path: PathBuf,
}

impl TempDirGuard {
    fn new(prefix: &str) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        let uniq = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "digimon_mcp_test_{}_{}_{}",
            prefix,
            std::process::id(),
            uniq
        ));
        std::fs::create_dir_all(&dir).expect("create temp test dir");
        Self { path: dir }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Interactive JSON-RPC stdio client keeping a single `digimon-engine-mcp`
/// child alive across sequential tool calls.
struct InteractiveMcp {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl InteractiveMcp {
    fn spawn() -> Self {
        let root = repo_root();
        let cards = root.join("data").join("cards.json");
        let mut child = Command::new(bin_path())
            .current_dir(&root)
            .args([
                "--cards-json",
                &cards.to_string_lossy(),
                "--pool",
                "all",
                "--max-games",
                "8",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn digimon-engine-mcp");
        let stdin = child.stdin.take().expect("child stdin");
        let stdout = BufReader::new(child.stdout.take().expect("child stdout"));
        let mut s = Self {
            child,
            stdin,
            stdout,
            next_id: 0,
        };
        let init = s.rpc("initialize", json!({}));
        assert_eq!(init["result"]["serverInfo"]["name"], "digimon-engine-mcp");
        s
    }

    fn rpc(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let req = json!({
            "jsonrpc": "2.0",
            "id": self.next_id,
            "method": method,
            "params": params,
        });
        let line = serde_json::to_string(&req).expect("serialize rpc request");
        writeln!(self.stdin, "{line}").expect("write rpc line");
        self.stdin.flush().expect("flush rpc stdin");

        let mut out = String::new();
        loop {
            out.clear();
            let n = self.stdout.read_line(&mut out).expect("read rpc line");
            assert!(n > 0, "unexpected EOF from digimon-engine-mcp");
            if !out.trim().is_empty() {
                break;
            }
        }
        serde_json::from_str(&out).expect("parse rpc response JSON")
    }

    fn call_tool(&mut self, name: &str, arguments: Value) -> Value {
        let resp = self.rpc(
            "tools/call",
            json!({
                "name": name,
                "arguments": arguments,
            }),
        );
        assert!(
            resp.get("error").is_none(),
            "unexpected JSON-RPC protocol error calling {name}: {resp}"
        );
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .expect("tool content text");
        serde_json::from_str(text).expect("tool content text is valid JSON")
    }
}

impl Drop for InteractiveMcp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Append a `ModelProto.metadata_props` entry (`field 14`, wire type `2` = `0x72`)
/// to an existing `.onnx` binary so we can test `digimon_value_semantics="terminal_outcome"`
/// without external Python dependencies.
fn append_onnx_metadata_prop(raw_onnx: &[u8], key: &str, value: &str) -> Vec<u8> {
    fn push_varint(buf: &mut Vec<u8>, mut n: usize) {
        while n >= 0x80 {
            buf.push(((n as u8) & 0x7f) | 0x80);
            n >>= 7;
        }
        buf.push(n as u8);
    }

    let kb = key.as_bytes();
    let vb = value.as_bytes();
    let mut entry = Vec::new();
    entry.push(0x0a);
    push_varint(&mut entry, kb.len());
    entry.extend_from_slice(kb);
    entry.push(0x12);
    push_varint(&mut entry, vb.len());
    entry.extend_from_slice(vb);

    let mut out = Vec::with_capacity(raw_onnx.len() + entry.len() + 8);
    out.extend_from_slice(raw_onnx);
    out.push(0x72);
    push_varint(&mut out, entry.len());
    out.extend_from_slice(&entry);
    out
}

/// Build a legal 54-card deck (4 `EX13-005` Digi-Eggs + 50 main cards) containing
/// `required_main` cards plus `ST1-03` filler, along with exact conserved `deck_top`.
fn build_conserved_deck(
    eggs_used: &[&str],
    main_used: &[&str],
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut full_eggs = vec!["EX13-005".to_string(); 4];
    let mut counts: HashMap<String, usize> = HashMap::new();
    for &c in main_used {
        *counts.entry(c.to_string()).or_insert(0) += 1;
    }
    let mut full_main: Vec<String> = Vec::with_capacity(50);
    for (cid, cnt) in &counts {
        for _ in 0..*cnt {
            full_main.push(cid.clone());
        }
    }
    while full_main.len() < 50 {
        full_main.push("ST1-03".to_string());
    }

    let rem_eggs = 4usize.saturating_sub(eggs_used.len());
    let egg_deck = vec!["EX13-005".to_string(); rem_eggs];

    let mut deck_top = full_main.clone();
    for &used in main_used {
        if let Some(pos) = deck_top.iter().position(|x| x == used) {
            deck_top.remove(pos);
        }
    }

    full_eggs.extend(full_main);
    (full_eggs, egg_deck, deck_top)
}

#[test]
fn inspect_stack_exposes_top_effect_inherited_only_sources_linked_cards_and_face_down_redaction() {
    let mut mcp = InteractiveMcp::spawn();

    // Seat 1 has:
    // - Battle slot 0: ["EX13-005", "EX13-018", "BT20-042"] + linked ["BT21-047"]
    // - Hand: ["EX13-071"] (Richard Sampson, which places deck top face-down under itself)
    // - Deck top[0]: "EX13-018" (Coredramon, which has inherited "+2000 DP" when face-up)
    let p1_main_used = [
        "EX13-018", // under BT20-042
        "BT20-042", // top of slot 0
        "BT21-047", // linked to slot 0
        "EX13-071", // in hand
        "EX13-018", // on top of deck to be tucked face-down under EX13-071
        "ST1-03",   // security
    ];
    let (deck1, p1_eggs, mut p1_deck_top) = build_conserved_deck(&["EX13-005"], &p1_main_used);
    // Put EX13-018 on top of P1's deck so EX13-071 tucks it face-down.
    p1_deck_top.insert(0, "EX13-018".to_string());
    // Remove one ST1-03? Wait: EX13-018 was already removed twice in build_conserved_deck, so add back!
    // Wait: build_conserved_deck removed ALL of p1_main_used from deck_top; EX13-018 #2 belongs IN deck_top!
    // So inserting EX13-018 at index 0 restores exact 54-card conservation!

    let (deck2, p2_eggs, p2_deck_top) = build_conserved_deck(&[], &["ST1-03"]);

    let puzzle = json!({
        "schema": "studio-puzzle-v1",
        "id": "inspect-stack-fixture",
        "title": "Inspect stack test",
        "decks": { "1": deck1, "2": deck2 },
        "seed": 0,
        "seat": 1,
        "setup": {
            "kind": "staged",
            "state": {
                "memory": 6,
                "phase": "Main",
                "turn": 3,
                "first_player": 1
            },
            "zones": {
                "1": {
                    "breeding": null,
                    "deck_top": p1_deck_top,
                    "egg_deck": p1_eggs,
                    "field": [
                        {
                            "stack": ["EX13-005", "EX13-018", "BT20-042"],
                            "is_suspended": true,
                            "turn_played": 2,
                            "linked_cards": ["BT21-047"]
                        }
                    ],
                    "hand": ["EX13-071"],
                    "security": ["ST1-03"],
                    "trash": []
                },
                "2": {
                    "breeding": null,
                    "deck_top": p2_deck_top,
                    "egg_deck": p2_eggs,
                    "field": [],
                    "hand": [],
                    "security": ["ST1-03"],
                    "trash": []
                }
            },
            "prefix_actions": []
        }
    });

    let loaded = mcp.call_tool("load_puzzle", json!({ "puzzle_json": puzzle, "mode": "author" }));
    assert_eq!(loaded["ok"], true, "load_puzzle failed: {loaded}");
    let gid = loaded["game_id"].as_str().unwrap().to_string();

    let stack = mcp.call_tool(
        "inspect_stack",
        json!({ "game_id": gid, "player": 0, "zone": "battle", "slot": 0 }),
    );
    assert_eq!(stack["ok"], true, "inspect_stack failed: {stack}");
    assert_eq!(stack["is_suspended"], true);
    assert_eq!(stack["turn_played"], 2);

    let top = &stack["top_card"];
    assert_eq!(top["card_id"], "BT20-042");
    assert_eq!(top["card_name"], "Groundramon");
    assert_eq!(top["level"], 5);
    assert_eq!(top["dp_printed"], 7000);
    let dp_eff = top["dp_effective"].as_i64().expect("dp_effective integer");
    assert!(
        dp_eff >= 9000,
        "expected effective DP >= 9000 (7000 printed + EX13-018 +2000 inherited + link bonus), got {dp_eff}"
    );
    assert!(
        top["effect_text"]
            .as_str()
            .unwrap_or("")
            .contains("Suspend 1 of your opponent's Digimon"),
        "top_card must expose main effect_text: {top}"
    );

    // Under-cards must expose inherited_text ONLY (never effect_text).
    let sources = stack["inherited_sources"].as_array().expect("inherited_sources");
    assert_eq!(sources.len(), 2);
    assert_eq!(sources[0]["card_id"], "EX13-005");
    assert_eq!(sources[1]["card_id"], "EX13-018");
    for src in sources {
        assert!(
            src.get("effect_text").is_none(),
            "under-card must NOT include main effect_text: {src}"
        );
        assert_eq!(src["face_down"], false);
    }
    assert!(
        sources[1]["inherited_text"]
            .as_str()
            .unwrap_or("")
            .contains("+2000 DP"),
        "EX13-018 under-card must expose inherited_text: {}",
        sources[1]
    );

    // Linked cards must expose link_text + dp_bonus ONLY (never effect_text or inherited_text).
    let linked = stack["linked_cards"].as_array().expect("linked_cards");
    assert_eq!(linked.len(), 1);
    assert_eq!(linked[0]["card_id"], "BT21-047");
    assert!(
        linked[0]["link_text"]
            .as_str()
            .unwrap_or("")
            .contains("Piercing"),
        "linked card must expose link_text: {}",
        linked[0]
    );
    assert!(linked[0].get("effect_text").is_none());
    assert!(linked[0].get("inherited_text").is_none());
    assert!(linked[0].get("dp_bonus").is_some());

    // Now play EX13-071 Richard Sampson (action 0) and accept its [On Play] to tuck
    // top of deck ("EX13-018") face-down under EX13-071 at battle slot 1!
    let step_play = mcp.call_tool("step", json!({ "game_id": gid, "player": 0, "action_id": 0 }));
    assert!(step_play.get("ok").unwrap_or(&json!(true)).as_bool().unwrap());

    // Accept the optional [On Play] face-down tuck if prompted.
    let ps = mcp.call_tool("pending_selection", json!({ "game_id": gid }));
    if ps.is_object() && ps.get("selecting_player").is_some() {
        let la = mcp.call_tool("legal_actions", json!({ "game_id": gid, "player": 0 }));
        let accept_id = la
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["action_id"].as_u64().unwrap())
            .find(|&aid| aid != 62)
            .expect("accept action for EX13-071 On Play");
        mcp.call_tool(
            "step",
            json!({ "game_id": gid, "player": 0, "action_id": accept_id }),
        );
    }

    // God view sees card_id == "EX13-018", face_down == true, and empty inherited_text.
    let tamer_god = mcp.call_tool(
        "inspect_stack",
        json!({ "game_id": gid, "player": 0, "zone": "battle", "slot": 1, "view": "god" }),
    );
    assert_eq!(tamer_god["ok"], true, "inspect_stack on EX13-071 failed: {tamer_god}");
    assert_eq!(tamer_god["top_card"]["card_id"], "EX13-071");
    let fd_god = &tamer_god["inherited_sources"][0];
    assert_eq!(fd_god["face_down"], true);
    assert_eq!(fd_god["card_id"], "EX13-018");
    assert_eq!(
        fd_god["inherited_text"], "",
        "face-down source must have empty inherited_text even in God view"
    );

    // Player view redacts face-down source identity to "<hidden>".
    let tamer_p0 = mcp.call_tool(
        "inspect_stack",
        json!({ "game_id": gid, "player": 0, "zone": "battle", "slot": 1, "view": "player0" }),
    );
    assert_eq!(tamer_p0["ok"], true);
    let fd_p0 = &tamer_p0["inherited_sources"][0];
    assert_eq!(fd_p0["face_down"], true);
    assert_eq!(fd_p0["card_id"], "<hidden>");
    assert_eq!(fd_p0["card_name"], "<hidden>");
    assert_eq!(fd_p0["inherited_text"], "");
}

#[test]
fn play_mode_locks_perspective_redacts_hidden_zones_blocks_mutators_and_drives_opponent() {
    let mut mcp = InteractiveMcp::spawn();
    let puzzle_path = repo_root()
        .join("qa/puzzles/ex12-erlangmon-saneiketsu-invitation-trap.json")
        .to_string_lossy()
        .to_string();

    let loaded = mcp.call_tool(
        "load_puzzle",
        json!({
            "puzzle_path": puzzle_path,
            "mode": "play",
            "opponent_driver": "greedy"
        }),
    );
    assert_eq!(loaded["ok"], true, "load_puzzle in play mode failed: {loaded}");
    assert_eq!(loaded["seat"], 0);
    assert_eq!(loaded["mode"], "play");
    let gid = loaded["game_id"].as_str().unwrap().to_string();

    // 1. view="god" and view="player1" are rejected across state/hand/field/security/inspect_stack.
    for (tool_name, extra_args) in [
        ("state", json!({ "game_id": gid, "view": "god" })),
        ("state", json!({ "game_id": gid, "view": "player1" })),
        ("hand", json!({ "game_id": gid, "player": 1, "view": "god" })),
        ("field", json!({ "game_id": gid, "player": 1, "view": "god" })),
        ("security", json!({ "game_id": gid, "player": 0, "view": "god" })),
        (
            "inspect_stack",
            json!({ "game_id": gid, "player": 1, "zone": "battle", "slot": 0, "view": "god" }),
        ),
    ] {
        let res = mcp.call_tool(tool_name, extra_args);
        assert_eq!(
            res["ok"], false,
            "expected {tool_name} with non-seat view to be blocked in play mode, got: {res}"
        );
        assert!(res["error"].as_str().unwrap_or("").contains("blocked in play mode"));
    }

    // 2. Omitted view clamps to Perspective::Player(0): own hand visible, opponent hand & face-down security redacted.
    let my_hand = mcp.call_tool("hand", json!({ "game_id": gid, "player": 0 }));
    assert_eq!(my_hand["count"], 2);
    assert!(my_hand["cards"].is_array(), "seat 0 should see own hand cards: {my_hand}");

    let opp_hand = mcp.call_tool("hand", json!({ "game_id": gid, "player": 1 }));
    assert_eq!(opp_hand["count"], 3);
    assert!(
        opp_hand["cards"].is_null(),
        "opponent hand cards must be redacted in play mode: {opp_hand}"
    );

    let my_sec = mcp.call_tool("security", json!({ "game_id": gid, "player": 0 }));
    assert_eq!(my_sec["count"], 3);
    assert!(
        my_sec["card_ids"].is_null(),
        "face-down security card_ids must be redacted in play mode: {my_sec}"
    );

    // 3. Author-only tools (save_puzzle, score_line, deck_cards) are blocked in play mode.
    let tmp = TempDirGuard::new("blocked_save");
    let blocked_save = mcp.call_tool(
        "save_puzzle",
        json!({
            "game_id": gid,
            "puzzle_path": tmp.path().join("nope.json").to_string_lossy(),
            "id": "nope",
            "title": "Nope"
        }),
    );
    assert_eq!(blocked_save["ok"], false);

    let blocked_score = mcp.call_tool(
        "score_line",
        json!({ "game_id": gid, "actions": [62] }),
    );
    assert_eq!(blocked_score["ok"], false);

    let blocked_decks = mcp.call_tool("deck_cards", json!({ "game_id": gid }));
    assert_eq!(blocked_decks["ok"], false);

    let blocked_perfect = mcp.call_tool(
        "inspect_position",
        json!({ "game_id": gid, "regime": "perfect_info", "simulations": 4 }),
    );
    assert_eq!(blocked_perfect["ok"], false);

    // 4. Passing turn (step 62) triggers the bound greedy opponent_driver to play Player 1's turn
    //    and bundles `opponent_events` (with opponent CardDrawn card_ids redacted!).
    let pass_res = mcp.call_tool("pass_turn", json!({ "game_id": gid, "player": 0 }));
    assert_eq!(pass_res["ok"], true, "pass_turn in play mode failed: {pass_res}");
    let opp_events = pass_res["opponent_events"]
        .as_array()
        .expect("opponent_driver should bundle opponent_events after pass_turn");
    assert!(
        !opp_events.is_empty(),
        "expected non-empty opponent_events when greedy opponent takes Turn 5"
    );
    for ev in opp_events {
        if ev.get("type") == Some(&json!("CardDrawn")) && ev.get("player") == Some(&json!(1)) {
            assert_eq!(
                ev["card_id"], "<hidden>",
                "opponent CardDrawn card_id must be redacted in play mode: {ev}"
            );
        }
    }
}

#[test]
fn save_puzzle_writes_canonical_studio_puzzle_and_search_exam_and_round_trips() {
    let mut mcp = InteractiveMcp::spawn();
    let tmp = TempDirGuard::new("save_roundtrip");

    let src_path = repo_root()
        .join("qa/puzzles/ex12-erlangmon-saneiketsu-invitation-trap.json")
        .to_string_lossy()
        .to_string();
    let loaded = mcp.call_tool(
        "load_puzzle",
        json!({ "puzzle_path": src_path, "mode": "author" }),
    );
    assert_eq!(loaded["ok"], true);
    let gid = loaded["game_id"].as_str().unwrap().to_string();

    let out_puzzle = tmp.path().join("saved-puzzle.json");
    let out_exam = tmp.path().join("saved-exam.json");
    let saved = mcp.call_tool(
        "save_puzzle",
        json!({
            "game_id": gid,
            "puzzle_path": out_puzzle.to_string_lossy(),
            "export_search_exam_path": out_exam.to_string_lossy(),
            "id": "saved-roundtrip-check",
            "title": "Saved Roundtrip Check",
            "description": "Round-trip export test",
            "folder": "unit",
            "tags": ["Ex12", "RoundTrip"],
            "seat": 1,
            "lines": [
                {
                    "id": "pass_line",
                    "name": "Pass",
                    "role": "avoid",
                    "actions": [62]
                }
            ],
            "expectations": [
                { "kind": "choose", "plan": [415], "label": "Choose Groundramon" },
                { "kind": "visits", "plan": [415], "min_share": 0.4, "label": "Visit Groundramon" },
                { "kind": "avoid", "plan": [0], "label": "Avoid Oblivion Bird" }
            ],
            "refutation_triggers": [
                {
                    "source_card_id": "EX12-034",
                    "phase": null,
                    "accept_optional": true,
                    "prefer_label_contains": "Sources"
                }
            ]
        }),
    );
    assert_eq!(saved["ok"], true, "save_puzzle failed: {saved}");
    assert!(out_puzzle.exists(), "studio-puzzle-v1 file was not created");
    assert!(out_exam.exists(), "search-exam-v1 file was not created");

    // Path traversal '..' must be rejected cleanly.
    let bad_path = mcp.call_tool(
        "save_puzzle",
        json!({
            "game_id": gid,
            "puzzle_path": "../escape.json",
            "id": "bad",
            "title": "Bad"
        }),
    );
    assert_eq!(bad_path["ok"], false);

    let p_doc: Value =
        serde_json::from_str(&std::fs::read_to_string(&out_puzzle).unwrap()).unwrap();
    assert_eq!(p_doc["schema"], "studio-puzzle-v1");
    assert_eq!(p_doc["id"], "saved-roundtrip-check");
    assert_eq!(p_doc["seat"], 1);
    assert_eq!(p_doc["tags"], json!(["ex12", "roundtrip"]));
    assert_eq!(p_doc["refutation_triggers"].as_array().unwrap().len(), 1);

    let e_doc: Value =
        serde_json::from_str(&std::fs::read_to_string(&out_exam).unwrap()).unwrap();
    assert_eq!(e_doc["schema"], "search-exam-v1");
    assert_eq!(e_doc["search"]["viewer"], 0);
    assert_eq!(e_doc["expectations"][0]["kind"], "chosen");
    assert_eq!(e_doc["expectations"][1]["kind"], "visit_share_at_least");
    assert_eq!(e_doc["expectations"][2]["kind"], "not_chosen");
    assert_eq!(e_doc["lines"]["Pass"], json!([62]));

    // Round-trip both written files back through load_puzzle.
    for path in [&out_puzzle, &out_exam] {
        let rt = mcp.call_tool(
            "load_puzzle",
            json!({ "puzzle_path": path.to_string_lossy(), "mode": "author" }),
        );
        assert_eq!(rt["ok"], true, "round-trip load_puzzle failed for {:?}: {rt}", path);
        let rt_gid = rt["game_id"].as_str().unwrap();
        let st = mcp.call_tool("state", json!({ "game_id": rt_gid }));
        assert_eq!(st["phase"], "Main");
        assert_eq!(st["memory"], 5);
        assert_eq!(st["turn_player"], 0);
    }
}

#[test]
fn verify_puzzle_validates_ex12_erlangmon_saneiketsu_invitation_trap_both_schemas() {
    let mut mcp = InteractiveMcp::spawn();
    let root = repo_root();

    for rel in [
        "qa/puzzles/ex12-erlangmon-saneiketsu-invitation-trap.json",
        "qa/search-exams/ex12-erlangmon-saneiketsu-invitation-trap.json",
    ] {
        let full = root.join(rel).to_string_lossy().to_string();
        let res = mcp.call_tool(
            "verify_puzzle",
            json!({
                "puzzle_path": full,
                "adversary_sims": 16,
                "seeds": 2
            }),
        );
        assert_eq!(res["ok"], true, "verify_puzzle ok=false on {rel}: {res}");
        assert_eq!(
            res["verified"], true,
            "verify_puzzle verified=false on {rel}: {res}"
        );

        let targets = res["target_results"].as_array().expect("target_results");
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0]["passed"], true);
        assert_eq!(targets[0]["final_winner"], 0);

        let avoids = res["avoid_results"].as_array().expect("avoid_results");
        assert_eq!(avoids.len(), 2);
        for av in avoids {
            assert_eq!(av["refuted"], true, "avoid line not refuted on {rel}: {av}");
            assert_eq!(av["passed"], true);
            assert_ne!(av["final_winner"], json!(0));
        }
        let bird = avoids
            .iter()
            .find(|a| {
                a["id"]
                    .as_str()
                    .unwrap_or("")
                    .contains("oblivion_bird")
            })
            .expect("oblivion_bird avoid line");
        assert!(
            bird["auto_adversary_steps"].as_u64().unwrap_or(0) >= 8,
            "expected >= 8 adversary interrupt steps on Oblivion Bird trap, got: {bird}"
        );
    }
}

#[test]
fn onnx_model_registry_inspect_position_and_score_line_end_to_end() {
    let mut mcp = InteractiveMcp::spawn();
    let root = repo_root();
    let tmp = TempDirGuard::new("onnx_models");

    let lite_fixture = root.join("code/digimon-engine/tests/fixtures/mlp_lite_value.onnx");
    let deck_fixture = root.join("code/digimon-engine/tests/fixtures/mlp_tiny_value.onnx");

    // Create a terminal_outcome-stamped copy in temp dir as well.
    let raw_bytes = std::fs::read(&lite_fixture).expect("read mlp_lite_value.onnx");
    let stamped_bytes =
        append_onnx_metadata_prop(&raw_bytes, "digimon_value_semantics", "terminal_outcome");
    let terminal_onnx = tmp.path().join("mlp_lite_terminal.onnx");
    std::fs::write(&terminal_onnx, stamped_bytes).expect("write mlp_lite_terminal.onnx");

    // 1. Load both shaped_return and terminal_outcome MLP models + deck-profile model.
    let m1 = mcp.call_tool(
        "load_model",
        json!({
            "model_id": "lite_shaped",
            "onnx_path": lite_fixture.to_string_lossy()
        }),
    );
    assert_eq!(m1["ok"], true, "load_model lite_shaped failed: {m1}");
    assert_eq!(m1["kind"], "mlp");
    assert_eq!(m1["profile"], "standard_lite_v2");
    assert_eq!(m1["value_semantics"], "shaped_return");

    let m2 = mcp.call_tool(
        "load_model",
        json!({
            "model_id": "lite_term",
            "onnx_path": terminal_onnx.to_string_lossy()
        }),
    );
    assert_eq!(m2["ok"], true, "load_model lite_term failed: {m2}");
    assert_eq!(m2["kind"], "mlp");
    assert_eq!(m2["profile"], "standard_lite_v2");
    assert_eq!(m2["value_semantics"], "terminal_outcome");

    let m3 = mcp.call_tool(
        "load_model",
        json!({
            "model_id": "deck_v2",
            "onnx_path": deck_fixture.to_string_lossy(),
            "profile": "standard_lite_deck_v2"
        }),
    );
    assert_eq!(m3["ok"], true, "load_model deck_v2 failed: {m3}");
    assert_eq!(m3["profile"], "standard_lite_deck_v2");

    let listed = mcp.call_tool("list_models", json!({}));
    assert_eq!(listed["ok"], true);
    assert_eq!(listed["models"].as_array().unwrap().len(), 3);

    // 2. Load the EX12 puzzle in author mode and run inspect_position (policy-only + search).
    let puzzle_path = root
        .join("qa/puzzles/ex12-erlangmon-saneiketsu-invitation-trap.json")
        .to_string_lossy()
        .to_string();
    let loaded = mcp.call_tool(
        "load_puzzle",
        json!({ "puzzle_path": puzzle_path, "mode": "author" }),
    );
    let gid = loaded["game_id"].as_str().unwrap().to_string();

    let ip_policy = mcp.call_tool(
        "inspect_position",
        json!({
            "game_id": gid,
            "model_id": "lite_term",
            "simulations": 0,
            "top_k": 6
        }),
    );
    assert_eq!(ip_policy["ok"], true, "inspect_position policy failed: {ip_policy}");
    assert_eq!(ip_policy["mode"], "policy_only");
    assert_eq!(ip_policy["viewer"], 1);
    assert!(ip_policy["policy_value"].is_number());
    assert!(ip_policy["policy_entropy"].as_f64().unwrap() >= 0.0);
    assert!(!ip_policy["policy_actions"].as_array().unwrap().is_empty());

    let ip_search = mcp.call_tool(
        "inspect_position",
        json!({
            "game_id": gid,
            "model_id": "deck_v2",
            "simulations": 12,
            "worlds": 2,
            "seed": 7,
            "expectations": [
                { "kind": "avoid", "plan": [114], "label": "Avoid swinging Coredramon directly" }
            ]
        }),
    );
    assert_eq!(ip_search["ok"], true, "inspect_position search failed: {ip_search}");
    assert_eq!(ip_search["mode"], "search");
    assert_eq!(ip_search["simulations_run"], 12);
    assert!(ip_search["search_value"].is_number());
    assert!(ip_search["chosen_action_id"].is_number());
    assert!(ip_search["chosen_plan"].is_array());
    assert!(!ip_search["root_plans"].as_array().unwrap().is_empty());
    assert!(!ip_search["root_q"].as_array().unwrap().is_empty());
    assert_eq!(ip_search["expectation_results"].as_array().unwrap().len(), 1);

    // 3. Run score_line with ONNX evaluator + refutation_triggers on the Oblivion Bird trap line
    //    and confirm the live game in GameRegistry is NOT mutated!
    let sl = mcp.call_tool(
        "score_line",
        json!({
            "game_id": gid,
            "model_id": "lite_term",
            "actions": [0, 101],
            "adversary_sims": 8,
            "refutation_triggers": [
                {
                    "source_card_id": "EX12-034",
                    "phase": null,
                    "accept_optional": true,
                    "prefer_label_contains": "Sources"
                },
                {
                    "source_card_id": "EX12-071",
                    "phase": null,
                    "accept_optional": true,
                    "prefer_label_contains": null
                }
            ]
        }),
    );
    assert_eq!(sl["ok"], true, "score_line failed: {sl}");
    assert_eq!(sl["root_mover"], 0);
    let steps = sl["steps"].as_array().expect("score_line steps");
    assert!(
        steps.len() >= 10,
        "expected >= 10 steps including adversary interrupts, got {}",
        steps.len()
    );
    assert_eq!(steps[0]["auto_adversary"], false);
    assert_eq!(steps[1]["auto_adversary"], false);
    assert!(steps.iter().skip(2).any(|s| s["auto_adversary"] == true && s["mover"] == 1));
    for stp in steps {
        assert!(stp["policy_value_for_root_mover"].is_number());
        assert!(stp["prior_prob"].is_number());
        assert!(stp["events_emitted"].is_array());
    }

    // Live game must remain untouched at Turn 4 Main, memory 5, with Coredramon still on field!
    let post_state = mcp.call_tool("state", json!({ "game_id": gid }));
    assert_eq!(post_state["turn_count"], 4);
    assert_eq!(post_state["phase"], "Main");
    assert_eq!(post_state["memory"], 5);
    let p1_field = mcp.call_tool("field", json!({ "game_id": gid, "player": 0 }));
    assert_eq!(p1_field["battle_area"].as_array().unwrap().len(), 1);
    assert_eq!(p1_field["battle_area"][0]["top_card_id"], "EX13-018");

    // 4. Test ONNX opponent_driver in Play mode!
    let play_onnx = mcp.call_tool(
        "load_puzzle",
        json!({
            "puzzle_path": puzzle_path,
            "mode": "play",
            "opponent_driver": "onnx:lite_term"
        }),
    );
    assert_eq!(play_onnx["ok"], true);
    let play_gid = play_onnx["game_id"].as_str().unwrap();
    let pass_onnx = mcp.call_tool("pass_turn", json!({ "game_id": play_gid, "player": 0 }));
    assert_eq!(pass_onnx["ok"], true, "onnx opponent_driver step failed: {pass_onnx}");
    assert!(!pass_onnx["opponent_events"].as_array().unwrap().is_empty());

    // 5. Unload models and confirm list_models updates.
    for mid in ["lite_shaped", "lite_term", "deck_v2"] {
        let u = mcp.call_tool("unload_model", json!({ "model_id": mid }));
        assert_eq!(u["ok"], true);
        assert_eq!(u["removed"], true);
    }
    let u_again = mcp.call_tool("unload_model", json!({ "model_id": "lite_term" }));
    assert_eq!(u_again["removed"], false);
    let empty_models = mcp.call_tool("list_models", json!({}));
    assert!(empty_models["models"].as_array().unwrap().is_empty());
}
