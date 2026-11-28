"""Tests for RL Studio Puzzle loader and PuzzleResetWrapper (Task 4.3)."""

from __future__ import annotations

import json
from pathlib import Path

import numpy as np
import pytest

from digimon_gym.agents.studio_puzzles import (
    PuzzleResetWrapper,
    load_studio_puzzle,
    load_studio_puzzles_dir,
)
from digimon_gym.digimon_gym import DigimonEnv

FILLER_DECK = ["ST1-01"] * 4 + ["ST1-03"] * 50


def _write_puzzle(path: Path, puzzle_id: str, folder: str, tags: list[str], rl_goal: dict) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    doc = {
        "schema": "studio-puzzle-v1",
        "id": puzzle_id,
        "title": f"Puzzle {puzzle_id}",
        "description": "test",
        "folder": folder,
        "tags": tags,
        "source": {"kind": "composed"},
        "decks": {"1": FILLER_DECK, "2": FILLER_DECK},
        "seed": 11,
        "seat": 1,
        "setup": {
            "kind": "staged",
            "state": {"memory": 3, "phase": "Main", "turn": 3, "first_player": 1},
            "zones": {
                "1": {"hand": ["ST1-03"], "field": [], "security": ["ST1-03"]},
                "2": {"hand": ["ST1-03"], "field": [], "security": ["ST1-03"]},
            },
            "prefix_actions": [],
        },
        "lines": [],
        "expectations": [],
        "assertions": {"engine": [], "ui": []},
        "rl_goal": rl_goal,
    }
    path.write_text(json.dumps(doc), encoding="utf-8")
    return path


def test_recursive_puzzle_loading_with_folder_and_tag_filters(tmp_path: Path):
    _write_puzzle(
        tmp_path / "examon" / "lethal" / "p1.json",
        "p1",
        "examon/lethal",
        ["lethal", "examon"],
        {"mode": "playout_to_win"},
    )
    _write_puzzle(
        tmp_path / "control" / "p2.json",
        "p2",
        "control",
        ["defense"],
        {"mode": "playout_to_win"},
    )
    (tmp_path / "_library.json").write_text(json.dumps({"version": 1, "folders": [], "items": {}}))

    all_p = load_studio_puzzles_dir(tmp_path)
    assert [p.id for p in all_p] == ["p2", "p1"] or {p.id for p in all_p} == {"p1", "p2"}

    ex_only = load_studio_puzzles_dir(tmp_path, folder="examon")
    assert [p.id for p in ex_only] == ["p1"]

    tag_only = load_studio_puzzles_dir(tmp_path, tags=["defense"])
    assert [p.id for p in tag_only] == ["p2"]


def test_puzzle_reset_wrapper_playout_and_bounded_horizon(tmp_path: Path):
    p_playout = load_studio_puzzle(
        _write_puzzle(
            tmp_path / "playout.json",
            "playout-1",
            "",
            ["test"],
            {"mode": "playout_to_win"},
        )
    )
    p_bounded = load_studio_puzzle(
        _write_puzzle(
            tmp_path / "bounded.json",
            "bounded-1",
            "",
            ["test"],
            {
                "mode": "bounded_horizon",
                "max_actions": 1,
                "max_turns": None,
                "win_assertions": [{"kind": "zone_count", "player": 1, "zone": "field", "value": 99}],
            },
        )
    )

    env = DigimonEnv()
    if not hasattr(env.unwrapped, "game") and True:
        pass
    wrapped = PuzzleResetWrapper(env, [p_playout], puzzle_prob=1.0, seed=0)
    obs, info = wrapped.reset(seed=0)
    assert obs.shape == env.observation_space.shape
    assert info["studio_puzzle_id"] == "playout-1"
    mask = wrapped.action_mask()
    assert mask.shape == (env.action_space.n,)
    assert int(mask[62]) == 1  # PASS is legal in Main phase

    # Bounded horizon: unsatisfied win_assertions expires at max_actions=1 -> -1.0
    wrapped_b = PuzzleResetWrapper(DigimonEnv(), [p_bounded], puzzle_prob=1.0, seed=0)
    obs_b, info_b = wrapped_b.reset(seed=0)
    assert info_b["studio_puzzle_goal_mode"] == "bounded_horizon"
    _, reward, terminated, truncated, step_info = wrapped_b.step(62)
    assert terminated is True
    assert truncated is False
    assert reward == pytest.approx(-1.0)
    assert step_info.get("studio_puzzle_horizon_expired") is True
