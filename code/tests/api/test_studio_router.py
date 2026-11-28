"""Tests for Board-State Studio backend router and PyO3 bindings (Tasks 2.4 & 3.7)."""

from __future__ import annotations

import json
from pathlib import Path
from unittest.mock import patch

import pytest
from fastapi import FastAPI
from fastapi.testclient import TestClient

import digimon_engine
from digimon_engine import RustDebugGame
from server.routers import debug_games, studio


FILLER_DECK = ["ST1-01"] * 4 + ["ST1-03"] * 50


def _make_client() -> TestClient:
    app = FastAPI()
    app.include_router(debug_games.router)
    app.include_router(studio.router)
    return TestClient(app)


def test_rust_debug_game_extensions_round_trip():
    game = RustDebugGame(FILLER_DECK, FILLER_DECK, 42)
    game.skip_mulligan()
    game.clear_zone(1, "battle_area")
    game.clear_zone(1, "egg_deck")
    game.clear_zone(1, "security")

    game.inject_card(1, "ST1-01", "egg_deck_top")
    game.inject_card(1, "ST1-03", "security_top_face_up")
    game.place_on_field(1, ["ST1-03", "ST1-07"], True, 2, ["ST1-11"])
    game.set_memory(3)
    game.set_turn(4)
    game.set_phase("Main")
    game.validate()

    snap = game.to_scenario()
    assert snap["zones"]["1"]["egg_deck"] == ["ST1-01"]
    assert snap["zones"]["1"]["face_up_security"] == [0]
    assert snap["zones"]["1"]["field"][0]["linked_cards"] == ["ST1-11"]
    assert isinstance(game.explain_action(62), str)


def test_rust_replay_session_dcgo_jsonl_step_seek_and_to_scenario():
    rec_path = studio.DCGO_RECORDINGS_DIR / "mod_game2.jsonl"
    if not rec_path.exists():
        pytest.skip("qa/dcgo-mod-recordings/mod_game2.jsonl not present")

    client = _make_client()
    r = client.post(
        "/studio/replays/load",
        json={"source": "dcgo_jsonl", "ref": "mod_game2.jsonl", "verify": True},
    )
    assert r.status_code == 200, r.text
    rep = r.json()
    rid = rep["replay_id"]
    assert rep["total_steps"] > 0
    assert rep["current_step"] == 0
    assert "player1" in rep["state"]

    r = client.post(f"/studio/replays/{rid}/step")
    assert r.status_code == 200, r.text
    assert r.json()["current_step"] >= 0

    r = client.post(f"/studio/replays/{rid}/seek", json={"step": 0})
    assert r.status_code == 200, r.text
    assert r.json()["current_step"] == 0

    r = client.post(f"/studio/replays/{rid}/fork")
    assert r.status_code == 200, r.text
    forked = r.json()
    assert forked["source"]["kind"] == "dcgo_jsonl"
    assert "session_id" in forked


def test_studio_session_compose_play_rewind_lines_and_snapshot():
    client = _make_client()

    # 1. Create blank board
    r = client.post("/studio/sessions", json={"blank": True})
    assert r.status_code == 200, r.text
    data = r.json()
    sid = data["session_id"]
    assert data["validation_error"] is None

    # 2. Compose board with stack, linked card, egg deck, face-up security, and hand
    board = data["board"]
    board["memory"] = 4
    board["turn"] = 3
    board["phase"] = "Main"
    board["first_player"] = 1
    board["zones"]["1"] = {
        "hand": ["ST1-03", "ST1-07"],
        "egg_deck": ["ST1-01"],
        "security": [{"card_id": "ST1-03", "face_up": True}],
        "field": [{"stack": ["ST1-03"], "is_suspended": False, "turn_played": 1, "linked_cards": ["ST1-11"]}],
        "trash": [],
        "breeding": [],
    }
    r = client.put(f"/studio/sessions/{sid}/board", json=board)
    assert r.status_code == 200, r.text
    updated = r.json()
    assert updated["internal_state"]["memory"] == 4
    assert updated["internal_state"]["players"][0]["egg_deck"] == ["ST1-01"]
    assert updated["internal_state"]["players"][0]["face_up_security"] == [0]
    assert updated["internal_state"]["players"][0]["battle_area"][0]["linked_cards"] == ["ST1-11"]

    # 3. Play a legal action (Pass Turn = 62) and record it
    r = client.post(f"/studio/sessions/{sid}/step", json={"action_id": 62})
    assert r.status_code == 200, r.text
    stepped = r.json()
    assert len(stepped["timeline"]) == 1
    assert stepped["timeline"][0]["action_id"] == 62

    # 4. Save named line, rewind to 0, then reload named line
    r = client.post(f"/studio/sessions/{sid}/lines", json={"name": "Pass line", "role": "avoid"})
    assert r.status_code == 200
    assert len(r.json()["lines"]) == 1
    line_id = r.json()["lines"][0]["id"]

    r = client.post(f"/studio/sessions/{sid}/rewind", json={"target_step": 0})
    assert r.status_code == 200
    assert len(r.json()["timeline"]) == 0

    r = client.post(f"/studio/sessions/{sid}/lines/{line_id}/load")
    assert r.status_code == 200
    assert len(r.json()["timeline"]) == 1

    # 5. Snapshot current board as new puzzle start
    r = client.post(f"/studio/sessions/{sid}/snapshot-as-start")
    assert r.status_code == 200
    assert len(r.json()["timeline"]) == 0


def test_studio_single_step_model_playout_greedy_and_mock_onnx(tmp_path: Path):
    client = _make_client()
    r = client.post("/studio/sessions", json={"blank": True})
    sid = r.json()["session_id"]

    # Assign P1=greedy, P2=greedy and step exactly ONE decision
    r = client.post(
        f"/studio/sessions/{sid}/seats",
        json={
            "seats": {
                "1": {"type": "greedy", "simulations": 0, "worlds": 1, "regime": "open_list", "seed": 0},
                "2": {"type": "greedy", "simulations": 0, "worlds": 1, "regime": "open_list", "seed": 0},
            }
        },
    )
    assert r.status_code == 200
    r = client.post(f"/studio/sessions/{sid}/model-step")
    assert r.status_code == 200, r.text
    assert len(r.json()["timeline"]) == 1
    assert r.json()["timeline"][0]["controller"] == "greedy"

    # Mock ONNX model inspection & model-step
    fake_onnx = tmp_path / "dummy.onnx"
    fake_onnx.write_bytes(b"fake")
    mock_eval = {
        "mode": "search",
        "regime": "open_list",
        "viewer": 1,
        "legal_count": 1,
        "policy_value": 0.25,
        "policy_actions": [{"action_id": 62, "label": "Pass", "prob": 1.0}],
        "search_value": 0.30,
        "simulations_run": 16,
        "root_plans": [{"actions": [62], "labels": ["Pass"], "visits": 16, "share": 1.0, "q_value": 0.3, "prior_prob": 1.0}],
        "root_q": [{"action_id": 62, "label": "Pass", "q_value": 0.3}],
        "chosen_action_id": 62,
        "chosen_plan": [62],
        "chosen_label": "Pass",
        "expectation_results": [],
    }
    with patch.object(studio, "_models_dir", return_value=tmp_path), patch.object(
        digimon_engine, "inspect_position", return_value=mock_eval, create=True
    ):
        r = client.get("/studio/models")
        assert r.status_code == 200
        assert [m["id"] for m in r.json()["models"]] == ["dummy.onnx"]

        r = client.post(
            f"/studio/games/{sid}/inspect-agent",
            json={"model_id": "dummy.onnx", "simulations": 16, "worlds": 1, "regime": "open_list", "seed": 0},
        )
        assert r.status_code == 200
        assert r.json()["search_value"] == 0.30


def test_studio_library_folders_tags_puzzles_and_path_traversal_guards(tmp_path: Path):
    client = _make_client()
    puzzles_dir = tmp_path / "puzzles"
    scenarios_dir = tmp_path / "scenarios"
    exams_dir = tmp_path / "search-exams"
    index_path = puzzles_dir / "_library.json"

    with patch.object(studio, "PUZZLES_DIR", puzzles_dir), patch.object(
        studio, "LIBRARY_INDEX_PATH", index_path
    ), patch.object(studio, "SCENARIOS_DIR", scenarios_dir), patch.object(
        studio, "SEARCH_EXAMS_DIR", exams_dir
    ), patch.object(studio, "REPO_ROOT", tmp_path):
        # Reject path traversal
        r = client.post(
            "/studio/puzzles",
            json={
                "id": "../escape",
                "title": "Bad",
                "decks": {"1": FILLER_DECK, "2": FILLER_DECK},
                "setup": {"kind": "staged", "state": {}, "zones": {}},
            },
        )
        assert r.status_code == 400

        # Create folder + save puzzle inside folder
        r = client.post("/studio/library/folders", json={"folder": "examon/lethal"})
        assert r.status_code == 200
        r = client.post(
            "/studio/puzzles",
            json={
                "id": "ex13-test-puzzle",
                "title": "Examon Test",
                "folder": "examon/lethal",
                "tags": ["lethal", "training"],
                "decks": {"1": FILLER_DECK, "2": FILLER_DECK},
                "seed": 7,
                "seat": 1,
                "setup": {
                    "kind": "staged",
                    "state": {"memory": 2, "phase": "Main", "turn": 3, "first_player": 1},
                    "zones": {"1": {"hand": ["ST1-03"]}},
                },
                "lines": [{"id": "main", "name": "Pass", "role": "target", "actions": [62]}],
                "expectations": [{"kind": "visits", "plan": [62], "min_share": 0.1}],
            },
        )
        assert r.status_code == 200, r.text
        assert (puzzles_dir / "examon" / "lethal" / "ex13-test-puzzle.json").exists()

        # Export to scenario & search-exam
        r = client.post("/studio/puzzles/ex13-test-puzzle/export/scenario")
        assert r.status_code == 200
        assert (scenarios_dir / "ex13-test-puzzle.json").exists()

        r = client.post("/studio/puzzles/ex13-test-puzzle/export/search-exam")
        assert r.status_code == 200
        exam_file = exams_dir / "ex13-test-puzzle.json"
        assert exam_file.exists()
        exam_doc = json.loads(exam_file.read_text(encoding="utf-8"))
        assert exam_doc["schema"] == "search-exam-v1"
        assert "staged" in exam_doc["setup"]

        # Orphan entry in _library.json does not break GET /studio/library
        idx = json.loads(index_path.read_text(encoding="utf-8"))
        idx["items"]["dcgo_jsonl:deleted_file.jsonl"] = {"folder": "examon/lethal", "tags": ["old"]}
        index_path.write_text(json.dumps(idx), encoding="utf-8")

        r = client.get("/studio/library")
        assert r.status_code == 200
        keys = [it["item_key"] for it in r.json()["items"]]
        assert "puzzle:ex13-test-puzzle" in keys
        assert "dcgo_jsonl:deleted_file.jsonl" not in keys


def test_studio_library_scans_external_raw_melon_multigame_and_guards_roots(tmp_path: Path, monkeypatch):
    client = _make_client()
    ext_dir = tmp_path / "dcgo_ext"
    ext_dir.mkdir()
    raw_file = ext_dir / "raw-20261125T100000Z.jsonl"
    raw_rows = [
        # Game 1: menu-only (no Snapshot.*) -> should be skipped
        {"method": "TurnStateMachine.Init", "t": "2026-11-25T10:00:00Z", "turn": 0},
        # Game 2: real game with snapshot + DCGO deck_name (should override classifier)
        {"method": "TurnStateMachine.Init", "t": "2026-11-25T10:05:00Z", "turn": 0},
        {
            "method": "Snapshot.Start",
            "t": "2026-11-25T10:05:01Z",
            "turn": 1,
            "you": 0,
            "deck_name": "My Spicy List",
            "players": [
                {"id": 0, "library": ["ST1-03"] * 45, "hand": ["ST1-03"] * 5, "security": [], "egg_library": [], "trash": []},
                {"id": 1, "library": ["ST1-03"] * 45, "hand": ["ST1-03"] * 5, "security": [], "egg_library": [], "trash": []},
            ],
        },
        {"method": "TurnStateMachine.EndGame", "t": "2026-11-25T10:15:00Z", "turn": 6, "args": ["Player", False, ""]},
        # Game 3: conceded game
        {"method": "TurnStateMachine.Init", "t": "2026-11-25T10:20:00Z", "turn": 0},
        {"method": "Snapshot.Start", "t": "2026-11-25T10:20:01Z", "turn": 2},
        {"method": "TurnStateMachine.EndGame", "t": "2026-11-25T10:21:00Z", "turn": 2, "args": ["Player", True, ""]},
        {"method": "TurnStateMachine.Surrender", "t": "2026-11-25T10:21:00Z", "turn": 2, "args": [0]},
    ]
    raw_file.write_text("\n".join(json.dumps(r) for r in raw_rows) + "\n", encoding="utf-8")

    monkeypatch.setenv("DIGIMON_STUDIO_AUTO_SCAN", "0")
    monkeypatch.setenv("DIGIMON_DCGO_RECORDER_DIRS", f"test_live={ext_dir}")
    r = client.get("/studio/library")
    assert r.status_code == 200, r.text
    live_items = [it for it in r.json()["items"] if it.get("source_root") == "test_live"]
    # Only games 2 and 3 (game 1 had no Snapshot.*), sorted newest-first (#g3 then #g2)
    assert [it["ref"] for it in live_items] == [
        "test_live:raw-20261125T100000Z.jsonl#g3",
        "test_live:raw-20261125T100000Z.jsonl#g2",
    ]
    assert "test-live" in live_items[0]["tags"]
    assert live_items[0]["finish_type"] == "concede"
    assert "concede" in live_items[0]["auto_tags"]
    assert live_items[1]["finish_type"] == "natural"
    assert "no-concede" in live_items[1]["auto_tags"]
    assert "My Spicy List" in live_items[1]["title"]
    assert live_items[1]["my_deck"] == "My Spicy List"
    assert "2026-11-25" in live_items[1]["title"]
    assert "T6" in live_items[1]["title"]

    # Path traversal and unknown roots are rejected
    bad1 = client.post("/studio/replays/load", json={"source": "dcgo_jsonl", "ref": "test_live:../escape.jsonl"})
    assert bad1.status_code == 400
    bad2 = client.post("/studio/replays/load", json={"source": "dcgo_jsonl", "ref": "C:\\Windows\\win.ini"})
    assert bad2.status_code == 400


def test_studio_loads_live_dcgo_raw_game_slice_if_present():
    if not studio.DEFAULT_DCGO_LIVE_DIR.exists():
        pytest.skip("Local DCGO live recorder directory not present")
    client = _make_client()
    r = client.get("/studio/library")
    assert r.status_code == 200
    live_items = [it for it in r.json()["items"] if it.get("source_root") == "dcgo_live"]
    if not live_items:
        pytest.skip("No playable raw games in local DCGO live recorder directory")

    newest_ref = live_items[0]["ref"]
    load_r = client.post(
        "/studio/replays/load",
        json={"source": "dcgo_jsonl", "ref": newest_ref, "verify": True},
    )
    assert load_r.status_code == 200, load_r.text
    rep = load_r.json()
    assert rep["item_key"] == f"dcgo_jsonl:{newest_ref}"
    assert rep["divergence_status"] in ("clean", "diverged", "error")
    assert rep["divergence_status"] in rep["auto_tags"]
    assert "player1" in rep["state"]
    assert rep["total_steps"] >= 0


def test_studio_open_search_exam_and_scenario_in_studio():
    client = _make_client()
    exam_file = studio.SEARCH_EXAMS_DIR / "ex13-examon-training-before-wingdramon.json"
    if exam_file.exists():
        r = client.post("/studio/search-exams/ex13-examon-training-before-wingdramon/open")
        assert r.status_code == 200, r.text
        body = r.json()
        assert "session_id" in body["session"]
        assert body["meta"]["id"] == "ex13-examon-training-before-wingdramon"
        assert "player1" in body["session"]["state"]

    scen_file = studio.SCENARIOS_DIR / "dna-paildramon-hand.json"
    if scen_file.exists():
        r = client.post("/studio/scenarios/dna-paildramon-hand/open")
        assert r.status_code == 200, r.text
        body = r.json()
        assert "session_id" in body["session"]
        assert body["meta"]["id"] == "dna-paildramon-hand"
        assert len(body["meta"]["assertions"]) >= 1
