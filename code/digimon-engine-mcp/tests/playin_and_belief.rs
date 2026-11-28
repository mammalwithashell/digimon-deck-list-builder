//! End-to-end integration tests for Track A next-wave deliverables:
//! 1. `compile_play_in` (Turn-1 legal play-in compilation, line remapping, and symbolic DCGO YAML export)
//! 2. `verify_belief_puzzle` on `qa/puzzles/ex13-belief-hand-trap-discipline.json` and `qa/search-exams/ex13-belief-hand-trap-discipline.json`
//! 3. `forced_selection.npy` and `chain_anchor_idx.npy` sidecars in `SelfPlayDriver`

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use digimon_engine::action::build_action_mask;
use digimon_engine::card_data::CardData;
use digimon_engine::game::Game;
use digimon_engine::rules::Rules;
use digimon_engine::search::{decision_player, DeterminizedConfig, UniformEvaluator};
use digimon_engine::selfplay::{run_self_play, SelfPlayConfig};
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

fn cards_json_path() -> PathBuf {
    repo_root().join("data").join("cards.json")
}

fn bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_digimon-engine-mcp"))
}

fn run_server(requests: &[Value]) -> Vec<Value> {
    let cards = cards_json_path();
    let mut child = Command::new(bin_path())
        .current_dir(repo_root())
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
        .expect("spawning digimon-engine-mcp");
    {
        let stdin = child.stdin.as_mut().unwrap();
        for req in requests {
            writeln!(stdin, "{}", serde_json::to_string(req).unwrap())
                .expect("writing JSON-RPC frame");
        }
    }
    let out = child.wait_with_output().expect("waiting on mcp server");
    assert!(
        out.status.success(),
        "mcp exited non-zero: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<Value>(l).expect("valid JSON-RPC line"))
        .collect()
}

fn unwrap_tool_json(resp: &Value) -> Value {
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("unexpected MCP response: {resp}"));
    serde_json::from_str(text).expect("tool response text is JSON")
}

fn unique_temp_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "digimon_track_a_{tag}_{}_{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn parse_npy_payload(path: &Path) -> (String, usize, Vec<u8>) {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    assert_eq!(&bytes[..6], b"\x93NUMPY", "invalid .npy magic");
    let header_len = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
    let header = std::str::from_utf8(&bytes[10..10 + header_len]).unwrap();
    let descr_pos = header.find("'descr': '").unwrap() + "'descr': '".len();
    let descr_end = header[descr_pos..].find('\'').unwrap() + descr_pos;
    let descr = header[descr_pos..descr_end].to_string();

    let shape_pos = header.find("'shape': (").unwrap() + "'shape': (".len();
    let shape_end = header[shape_pos..].find(')').unwrap() + shape_pos;
    let first_dim_str = header[shape_pos..shape_end]
        .split(',')
        .next()
        .unwrap()
        .trim();
    let rows: usize = first_dim_str.parse().unwrap();

    (descr, rows, bytes[10 + header_len..].to_vec())
}

#[test]
fn compile_play_in_upgrades_staged_puzzle_and_exports_symbolic_dcgo_yaml() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(run_compile_play_in_test)
        .unwrap()
        .join()
        .unwrap();
}

fn run_compile_play_in_test() {
    let tmp = unique_temp_dir("playin");
    let out_puzzle = tmp.join("upgraded_puzzle.json");
    let out_yaml = tmp.join("exported_exam.yaml");

    let mut d1 = vec!["ST1-01"; 4];
    d1.extend(std::iter::repeat_n("ST1-03", 20));
    d1.extend(std::iter::repeat_n("ST1-05", 15));
    d1.extend(std::iter::repeat_n("BT2-091", 15));

    let mut d2 = vec!["ST1-01"; 4];
    d2.extend(std::iter::repeat_n("ST1-02", 25));
    d2.extend(std::iter::repeat_n("ST1-03", 25));

    let staged_puzzle = json!({
        "schema": "studio-puzzle-v1",
        "id": "mcp-playin-birdramon-volcanic-flare",
        "title": "Raise Birdramon stack and clear Biyomon with Volcanic Flare",
        "seat": 1,
        "seed": 0,
        "decks": { "1": d1, "2": d2 },
        "setup": {
            "kind": "staged",
            "state": {
                "memory": 3,
                "phase": "Main",
                "turn": 3,
                "first_player": 1
            },
            "zones": {
                "1": {
                    "hand": ["BT2-091"],
                    "security": ["ST1-03", "ST1-03", "ST1-03", "ST1-03", "ST1-03"],
                    "trash": [],
                    "field": [
                        { "stack": ["ST1-01", "ST1-03", "ST1-05"], "is_suspended": false, "turn_played": 1 }
                    ],
                    "breeding": null
                },
                "2": {
                    "hand": ["ST1-03"],
                    "security": ["ST1-03", "ST1-03", "ST1-03", "ST1-03", "ST1-03"],
                    "trash": [],
                    "field": [
                        { "stack": ["ST1-02"], "is_suspended": false, "turn_played": 2 }
                    ],
                    "breeding": null
                }
            }
        },
        "rl_goal": {
            "win_assertions": [
                { "kind": "zone_count", "player": 2, "zone": "field", "value": 0 },
                { "kind": "zone_count", "player": 2, "zone": "security", "value": 4 }
            ]
        },
        "lines": [
            {
                "id": "line-target",
                "name": "Birdramon security swing + Gaia Force clear",
                "role": "target",
                "actions": [114, 0, 100]
            },
            {
                "id": "line-avoid",
                "name": "Pass turn passively",
                "role": "avoid",
                "actions": [62]
            }
        ],
        "expectations": [
            { "kind": "choose", "plan": [114] },
            { "kind": "avoid", "plan": [62] }
        ]
    });

    let resps = run_server(&[json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "compile_play_in",
            "arguments": {
                "puzzle_json": staged_puzzle,
                "beam_width": 32,
                "max_turns": 5,
                "strict_zones": false,
                "out_puzzle_path": out_puzzle.to_string_lossy(),
                "out_dcgo_yaml_path": out_yaml.to_string_lossy()
            }
        }
    })]);
    let res = unwrap_tool_json(&resps[0]);
    assert_eq!(res["ok"], true, "compile_play_in failed: {res}");
    assert_eq!(res["matched"], true, "expected play-in match: {res}");
    assert_eq!(res["strict_zones"], false);
    assert_eq!(res["distance_score"].as_f64().unwrap(), 0.0);

    let prefix_actions: Vec<u16> = serde_json::from_value(res["prefix_actions"].clone()).unwrap();
    assert!(prefix_actions.starts_with(&[0, 0]));

    // Verify upgraded `replay_prefix` puzzle written to `out_puzzle_path` passes `verify_puzzle`!
    assert!(out_puzzle.exists(), "out_puzzle_path was not written");
    let upgraded_doc: Value =
        serde_json::from_str(&std::fs::read_to_string(&out_puzzle).unwrap()).unwrap();
    assert_eq!(upgraded_doc["setup"]["kind"], "replay_prefix");
    assert!(upgraded_doc["setup"].get("zones").is_none());

    let verify_resps = run_server(&[json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "verify_puzzle",
            "arguments": {
                "puzzle_path": out_puzzle.to_string_lossy(),
                "seeds": 2
            }
        }
    })]);
    let v_body = unwrap_tool_json(&verify_resps[0]);
    assert_eq!(v_body["ok"], true, "verify_puzzle failed: {v_body}");
    assert_eq!(
        v_body["verified"], true,
        "upgraded replay_prefix puzzle did not pass verify_puzzle: {v_body}"
    );

    // Also verify Turn-1 replay legality directly on `digimon_engine::Game::new`.
    let card_map = CardData::load_from_file(&cards_json_path()).unwrap();
    let sd1: Vec<String> = serde_json::from_value(res["stacked_decks"]["1"].clone()).unwrap();
    let sd2: Vec<String> = serde_json::from_value(res["stacked_decks"]["2"].clone()).unwrap();
    let mut game = Game::new(&[sd1, sd2], &card_map, Rules::standard(), Some(0)).unwrap();
    for &aid in &prefix_actions {
        let pid = decision_player(&game);
        let mask = build_action_mask(&game, pid);
        assert!(mask[aid as usize] > 0.5, "illegal prefix action {aid}");
        game.decode_action(aid, pid);
    }
    assert_eq!(game.memory, 3);
    assert_eq!(game.players[0].battle_area.len(), 1);
    assert_eq!(game.players[1].battle_area.len(), 1);

    // Verify symbolic DCGO YAML written to `out_dcgo_yaml_path`.
    assert!(out_yaml.exists(), "out_dcgo_yaml_path was not written");
    let yaml_text = std::fs::read_to_string(&out_yaml).unwrap();
    assert_eq!(Some(yaml_text.as_str()), res["dcgo_yaml"].as_str());
    assert!(yaml_text.contains("decks:\n  p0:\n    stack:\n"));
    assert!(yaml_text.contains("  p1:\n    stack:\n"));
    assert!(yaml_text.contains("steps:\n"));
    assert!(yaml_text.contains("hatch:"));
    assert!(yaml_text.contains("digivolve:"));
    assert!(yaml_text.contains("move:"));
    assert!(yaml_text.contains("attack:"));
    assert!(yaml_text.contains("play:"));

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn verify_belief_puzzle_passes_ex13_hand_trap_discipline_and_reports_information_gap() {
    let root = repo_root();
    let puzzle_path = root.join("qa/puzzles/ex13-belief-hand-trap-discipline.json");
    let exam_path = root.join("qa/search-exams/ex13-belief-hand-trap-discipline.json");
    assert!(puzzle_path.exists(), "missing {}", puzzle_path.display());
    assert!(exam_path.exists(), "missing {}", exam_path.display());

    let resps = run_server(&[
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "verify_puzzle",
                "arguments": {
                    "puzzle_path": puzzle_path.to_string_lossy(),
                    "seeds": 2
                }
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "verify_belief_puzzle",
                "arguments": {
                    "puzzle_path": puzzle_path.to_string_lossy(),
                    "simulations": 256,
                    "worlds": 16,
                    "seeds": 2
                }
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "verify_belief_puzzle",
                "arguments": {
                    "puzzle_path": exam_path.to_string_lossy(),
                    "simulations": 256,
                    "worlds": 16,
                    "seeds": 2
                }
            }
        }),
    ]);

    let vp = unwrap_tool_json(&resps[0]);
    assert_eq!(vp["ok"], true, "verify_puzzle error: {vp}");
    assert_eq!(vp["verified"], true, "verify_puzzle failed: {vp}");

    for (label, vb) in [
        ("studio-puzzle-v1", unwrap_tool_json(&resps[1])),
        ("search-exam-v1", unwrap_tool_json(&resps[2])),
    ] {
        assert_eq!(vb["ok"], true, "{label} verify_belief_puzzle error: {vb}");
        assert_eq!(
            vb["passed"], true,
            "{label} verify_belief_puzzle expectations failed: {vb}"
        );
        let closed_mean = vb["closed_value_mean"].as_f64().unwrap();
        let open_mean = vb["open_value_mean"].as_f64().unwrap();
        let gap = vb["information_gap"].as_f64().unwrap();
        assert!(
            gap > 0.05 && open_mean > closed_mean,
            "{label} expected positive information_gap (open={open_mean}, closed={closed_mean}, gap={gap})"
        );
        let shares = vb["closed_visit_shares"].as_array().unwrap();
        assert!(!shares.is_empty());
        assert_eq!(
            shares[0]["plan"],
            json!([0]),
            "{label} top closed-list plan should bounce the Red Lv.5 Blast host first: {shares:?}"
        );
        let exps = vb["expectation_results"].as_array().unwrap();
        assert!(exps.len() >= 2);
        assert!(exps.iter().all(|e| e["passed"] == json!(true)));
    }
}

#[test]
fn selfplay_emits_forced_selection_and_chain_anchor_idx_sidecars() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(run_selfplay_sidecar_test)
        .unwrap()
        .join()
        .unwrap();
}

fn run_selfplay_sidecar_test() {
    let cards = CardData::load_from_file(&cards_json_path()).unwrap();

    // Decks rich in mandatory single-target effects (`ST1-16` Gaia Force, `BT2-096` Ray of Victory)
    // and multi-step `PendingSelection` chains (`EX13-024` Slayerdramon) so self-play hits both
    // forced selections and multi-step same-mover selection chains.
    let mut deck = vec!["EX13-002".to_string(); 4];
    deck.extend(std::iter::repeat_n("ST1-03".to_string(), 16));
    deck.extend(std::iter::repeat_n("EX13-021".to_string(), 10));
    deck.extend(std::iter::repeat_n("EX13-024".to_string(), 8));
    deck.extend(std::iter::repeat_n("ST1-16".to_string(), 8));
    deck.extend(std::iter::repeat_n("BT2-096".to_string(), 6));
    deck.extend(std::iter::repeat_n("BT1-027".to_string(), 2));

    let decks = vec![deck];
    let deck_names = vec!["ExamonTest".to_string()];

    let mut base_det = DeterminizedConfig::default();
    base_det.worlds = 1;
    base_det.search.simulations = 8;

    let tmp_off = unique_temp_dir("sp_off");
    let tmp_on = unique_temp_dir("sp_on");

    let cfg_off = SelfPlayConfig {
        games: 4,
        seed: 777,
        decision_cap: 120,
        shard_rows: 10_000,
        include_forced_selection_value: false,
        search: base_det,
        ..Default::default()
    };

    let mut cfg_on = cfg_off.clone();
    cfg_on.include_forced_selection_value = true;

    let mut eval_off = UniformEvaluator;
    let stats_off = run_self_play(
        &cards,
        &decks,
        &deck_names,
        &mut eval_off,
        &cfg_off,
        &tmp_off,
        "uniform",
    )
    .expect("selfplay run (forced=false)");

    let mut eval_on = UniformEvaluator;
    let stats_on = run_self_play(
        &cards,
        &decks,
        &deck_names,
        &mut eval_on,
        &cfg_on,
        &tmp_on,
        "uniform",
    )
    .expect("selfplay run (forced=true)");

    let shard_off = tmp_off.join("shard-000");
    let shard_on = tmp_on.join("shard-000");

    let (fs_descr_off, fs_rows_off, fs_bytes_off) =
        parse_npy_payload(&shard_off.join("forced_selection.npy"));
    let (ca_descr_off, ca_rows_off, ca_bytes_off) =
        parse_npy_payload(&shard_off.join("chain_anchor_idx.npy"));
    assert_eq!(fs_descr_off, "|u1");
    assert_eq!(ca_descr_off, "<i8");
    assert_eq!(fs_rows_off, stats_off.rows);
    assert_eq!(ca_rows_off, stats_off.rows);
    assert!(
        fs_bytes_off.iter().all(|&b| b == 0),
        "include_forced_selection_value=false must emit all-zero forced_selection.npy"
    );

    let (fs_descr_on, fs_rows_on, fs_bytes_on) =
        parse_npy_payload(&shard_on.join("forced_selection.npy"));
    let (ca_descr_on, ca_rows_on, ca_bytes_on) =
        parse_npy_payload(&shard_on.join("chain_anchor_idx.npy"));
    let (pw_descr_on, pw_rows_on, pw_bytes_on) =
        parse_npy_payload(&shard_on.join("policy_weight.npy"));
    assert_eq!(fs_descr_on, "|u1");
    assert_eq!(ca_descr_on, "<i8");
    assert_eq!(pw_descr_on, "<f4");
    assert_eq!(fs_rows_on, stats_on.rows);
    assert_eq!(ca_rows_on, stats_on.rows);
    assert_eq!(pw_rows_on, stats_on.rows);

    // Non-forced rows in `cfg_on` must match `cfg_off` count exactly (temperature clock &
    // trajectory preserved when `include_forced_selection_value` is toggled!).
    let non_forced_count_on = fs_bytes_on.iter().filter(|&&b| b == 0).count();
    assert_eq!(non_forced_count_on, stats_off.rows);
    assert!(
        stats_on.rows >= stats_off.rows,
        "forced=true should have >= rows than forced=false"
    );

    // Verify every `forced_selection == 1` row has `policy_weight == 0.0`,
    // and every `chain_anchor_idx >= 0` is a valid future local row index `i < anchor < rows`.
    let pw_floats: Vec<f32> = pw_bytes_on
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    let anchors_on: Vec<i64> = ca_bytes_on
        .chunks_exact(8)
        .map(|c| i64::from_le_bytes(c.try_into().unwrap()))
        .collect();
    let anchors_off: Vec<i64> = ca_bytes_off
        .chunks_exact(8)
        .map(|c| i64::from_le_bytes(c.try_into().unwrap()))
        .collect();

    for (i, (&fs, &pw)) in fs_bytes_on.iter().zip(pw_floats.iter()).enumerate() {
        if fs == 1 {
            assert_eq!(
                pw, 0.0,
                "forced_selection=1 row {i} must carry policy_weight=0.0"
            );
        } else {
            assert_eq!(fs, 0);
            assert!(pw > 0.0, "normal row {i} should carry positive policy_weight");
        }
    }

    for (rows, anchors) in [(stats_off.rows, &anchors_off), (stats_on.rows, &anchors_on)] {
        for (i, &anc) in anchors.iter().enumerate() {
            assert!(
                anc == -1 || ((anc as usize) > i && (anc as usize) < rows),
                "invalid chain_anchor_idx[{i}] = {anc} for shard of {rows} rows"
            );
        }
    }

    let _ = std::fs::remove_dir_all(&tmp_off);
    let _ = std::fs::remove_dir_all(&tmp_on);
}
