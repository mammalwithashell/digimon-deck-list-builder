"""Tests for code/digimon_gym/agents/game_log.py and the WinRateCallback
row-emission integration."""

from __future__ import annotations

import json
import os
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

from digimon_gym.agents.game_log import GameLogWriter


# ---------------------------------------------------------------------------
# GameLogWriter
# ---------------------------------------------------------------------------


def test_writer_appends_not_truncates(tmp_path: Path):
    path = tmp_path / "eval_game_log.jsonl"
    path.write_text('{"existing": 1}\n{"existing": 2}\n', encoding="utf-8")
    writer = GameLogWriter(path)
    writer.append({"new": 3})
    lines = path.read_text(encoding="utf-8").splitlines()
    assert len(lines) == 3
    assert json.loads(lines[0])["existing"] == 1
    assert json.loads(lines[1])["existing"] == 2
    assert json.loads(lines[2])["new"] == 3


def test_writer_flushes_each_row(tmp_path: Path):
    path = tmp_path / "eval_game_log.jsonl"
    writer = GameLogWriter(path)
    writer.append({"row": 1})
    # File handle was closed by `with`, so flush happened. Read raw.
    assert path.read_text(encoding="utf-8") == '{"row": 1}\n'


def test_writer_disables_on_oserror_and_logs_once(tmp_path: Path, capsys):
    path = tmp_path / "eval_game_log.jsonl"
    writer = GameLogWriter(path)

    with patch.object(Path, "open", side_effect=OSError("disk full")):
        writer.append({"row": 1})
    assert writer.enabled is False
    err1 = capsys.readouterr().err
    assert "game_log" in err1
    assert "disk full" in err1

    # Second append must silently no-op (no further log lines).
    writer.append({"row": 2})
    err2 = capsys.readouterr().err
    assert err2 == ""
    # File should not exist or be empty (open failed both times).
    if path.exists():
        assert path.read_text(encoding="utf-8") == ""


def test_writer_creates_parent_dir(tmp_path: Path):
    path = tmp_path / "deep" / "nested" / "eval_game_log.jsonl"
    writer = GameLogWriter(path)
    writer.append({"row": 1})
    assert path.exists()


# ---------------------------------------------------------------------------
# WinRateCallback row emission
# ---------------------------------------------------------------------------


def _make_callback_with_writer(tmp_path: Path, writer: GameLogWriter | None):
    """Construct a WinRateCallback without running its eval loop."""
    from digimon_gym.agents.pilot_training import WinRateCallback

    return WinRateCallback(
        eval_env_fn=lambda: None,
        eval_freq=0,
        n_eval_episodes=0,
        game_log_writer=writer,
    )


def test_callback_accepts_game_log_writer(tmp_path: Path):
    writer = GameLogWriter(tmp_path / "eval_game_log.jsonl")
    cb = _make_callback_with_writer(tmp_path, writer)
    assert cb._game_log_writer is writer
    assert cb._eval_window_idx == 0


def test_callback_writer_defaults_to_none():
    cb = _make_callback_with_writer(Path("."), None)
    assert cb._game_log_writer is None


# The full _run_evaluation flow exercises a real DigimonEnv. Cover those
# in the existing pilot_training integration tests; here we test the row
# build logic by directly invoking the helper.


def test_row_builder_maps_win(tmp_path: Path):
    from digimon_gym.agents.pilot_training import _build_game_log_row

    rec = tmp_path / "rec.json"
    rec.touch()
    row = _build_game_log_row(
        step=100_000,
        eval_window_idx=4,
        game_idx=2,
        agent_archetype="BlueFlare",
        opponent_archetype="Omnimon",
        p1_digi=2,
        p1_dna=1,
        p2_digi=3,
        p2_dna=0,
        winner_id=1,
        terminal_score=1.0,
        steps=18,
        recording_path=rec,
    )
    expected_path = str(rec.resolve())
    assert row == {
        "step": 100_000,
        "eval_window_idx": 4,
        "game_idx": 2,
        "source": "eval",
        "match_format": "single",
        "match_idx": None,
        "game_in_match_idx": None,
        "agent_archetype": "BlueFlare",
        "opponent_archetype": "Omnimon",
        "digivolves_agent": 2,
        "dna_digivolves_agent": 1,
        "digivolves_opponent": 3,
        "dna_digivolves_opponent": 0,
        "result": "win",
        "episode_length": 18,
        "terminal_score": 1.0,
        "recording_path": expected_path,
    }


def test_row_builder_maps_loss():
    from digimon_gym.agents.pilot_training import _build_game_log_row

    row = _build_game_log_row(
        step=0, eval_window_idx=0, game_idx=0,
        agent_archetype=None, opponent_archetype=None,
        p1_digi=0, p1_dna=0, p2_digi=0, p2_dna=0,
        winner_id=2, terminal_score=-1.0, steps=5,
        recording_path=None,
    )
    assert row["result"] == "loss"
    assert row["terminal_score"] == -1.0
    assert row["recording_path"] is None


def test_row_builder_maps_draw():
    from digimon_gym.agents.pilot_training import _build_game_log_row

    row = _build_game_log_row(
        step=0, eval_window_idx=0, game_idx=0,
        agent_archetype=None, opponent_archetype=None,
        p1_digi=0, p1_dna=0, p2_digi=0, p2_dna=0,
        winner_id=None, terminal_score=0.0, steps=200,
        recording_path=None,
    )
    assert row["result"] == "draw"
    assert row["terminal_score"] == 0.0


def test_row_builder_bo3_fields():
    """BO3 rows carry match_idx + game_in_match_idx; single mode leaves them null."""
    from digimon_gym.agents.pilot_training import _build_game_log_row

    row_bo3 = _build_game_log_row(
        step=0, eval_window_idx=0, game_idx=5,
        agent_archetype=None, opponent_archetype=None,
        p1_digi=0, p1_dna=0, p2_digi=0, p2_dna=0,
        winner_id=1, terminal_score=1.0, steps=10,
        recording_path=None,
        match_format="bo3", match_idx=2, game_in_match_idx=1,
    )
    assert row_bo3["match_format"] == "bo3"
    assert row_bo3["match_idx"] == 2
    assert row_bo3["game_in_match_idx"] == 1

    row_single = _build_game_log_row(
        step=0, eval_window_idx=0, game_idx=0,
        agent_archetype=None, opponent_archetype=None,
        p1_digi=0, p1_dna=0, p2_digi=0, p2_dna=0,
        winner_id=1, terminal_score=1.0, steps=10,
        recording_path=None,
    )
    assert row_single["match_format"] == "single"
    assert row_single["match_idx"] is None
    assert row_single["game_in_match_idx"] is None


def test_row_builder_absolutizes_recording_path(tmp_path: Path):
    from digimon_gym.agents.pilot_training import _build_game_log_row

    rel = tmp_path / "rec.json"
    rel.touch()
    row = _build_game_log_row(
        step=0, eval_window_idx=0, game_idx=0,
        agent_archetype=None, opponent_archetype=None,
        p1_digi=0, p1_dna=0, p2_digi=0, p2_dna=0,
        winner_id=None, terminal_score=0.0, steps=1,
        recording_path=rel,
    )
    # Should be an absolute path string.
    assert os.path.isabs(row["recording_path"])
    assert Path(row["recording_path"]) == rel.resolve()


# ---------------------------------------------------------------------------
# Recording-path finder
# ---------------------------------------------------------------------------


def test_find_eval_recording_path_returns_none_without_recording_wrapper():
    from digimon_gym.agents.pilot_training import _find_eval_recording_path

    class Plain:
        pass

    assert _find_eval_recording_path(Plain()) is None


def test_find_eval_recording_path_walks_env_chain(tmp_path: Path):
    from digimon_gym.agents.pilot_training import _find_eval_recording_path
    from digimon_gym.agents.training_recording import TrainingRecordingWrapper

    rec_path = tmp_path / "rec.json"
    rec_path.touch()
    # Build a minimal mock with last_recording_path
    rec_wrapper = MagicMock(spec=TrainingRecordingWrapper)
    rec_wrapper.last_recording_path = rec_path

    # Outer wrapper that exposes `.env` (gymnasium.Wrapper protocol)
    outer = MagicMock()
    outer.env = rec_wrapper

    found = _find_eval_recording_path(outer)
    assert found == rec_path
