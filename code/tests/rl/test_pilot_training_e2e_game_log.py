"""End-to-end integration test for the eval-game-log pipeline.

The unit tests in ``test_game_log.py`` cover the writer, row builder, and
recording-path finder in isolation. This test drives a short generalist
training run with an eval window that actually fires and asserts that
``eval_game_log.jsonl`` appears under the run directory with at least
one row per eval game, each carrying the documented schema. Covers
both ``match_format="single"`` (legacy one-game-per-episode) and
``match_format="bo3"`` (new default — one episode = up to 3 games,
emit one row per inner game).
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest


REQUIRED_FIELDS = (
    "step",
    "eval_window_idx",
    "game_idx",
    "source",
    "match_format",
    "match_idx",
    "game_in_match_idx",
    "agent_archetype",
    "opponent_archetype",
    "digivolves_agent",
    "dna_digivolves_agent",
    "digivolves_opponent",
    "dna_digivolves_opponent",
    "result",
    "episode_length",
    "terminal_score",
    "recording_path",
)


def _run_short_training(tmp_path: Path, *, match_format: str, eval_game_log: str, run_name: str):
    from digimon_gym.agents.gauntlet import load_generalist_deck_pool
    from digimon_gym.agents.pilot_training import train
    from digimon_gym.agents.training_config import TrainingConfig

    pool = load_generalist_deck_pool()
    assert pool.archetype_names, "generalist deck pool is empty; cannot run test"

    cfg = TrainingConfig(
        algorithm="mlp",
        timesteps=2000,
        seed=1,
        n_envs=1,
        n_steps=128,
        batch_size=32,
        n_epochs=2,
        opponent="greedy",
        generalist=True,
        eval_freq=500,
        eval_episodes=2,
        checkpoint_every=0,
        models_dir=str(tmp_path),
        tensorboard_log=str(tmp_path / "tb"),
        run_name=run_name,
        mulligan_log="off",
        eval_game_log=eval_game_log,
        match_format=match_format,
        curriculum_seed=1,
    )

    train(cfg=cfg, generalist_deck_pool=pool, verbose=0)
    return pool, tmp_path / run_name / "eval_game_log.jsonl"


@pytest.mark.slow
def test_train_writes_eval_game_log_single_mode(tmp_path):
    """`match_format='single'` legacy path: one row per game, no
    `match_idx` / `game_in_match_idx`."""
    pool, jsonl_path = _run_short_training(
        tmp_path,
        match_format="single",
        eval_game_log="on",
        run_name="test_single_e2e",
    )
    assert jsonl_path.exists(), f"expected eval game log at {jsonl_path}"

    rows = [
        json.loads(line)
        for line in jsonl_path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    assert len(rows) >= 2, (
        f"expected >=2 rows from 2000 timesteps × eval_freq=500 × "
        f"eval_episodes=2 in single mode; got {len(rows)}"
    )

    expected_archetypes = set(pool.archetype_names)
    for i, row in enumerate(rows):
        missing = [k for k in REQUIRED_FIELDS if k not in row]
        assert not missing, f"row {i} missing fields {missing}: {row!r}"
        assert row["source"] == "eval"
        assert row["match_format"] == "single"
        assert row["match_idx"] is None
        assert row["game_in_match_idx"] is None
        assert row["result"] in {"win", "loss", "draw"}
        assert row["episode_length"] >= 1
        assert row["agent_archetype"] in expected_archetypes


@pytest.mark.slow
def test_train_writes_eval_game_log_bo3_mode(tmp_path):
    """`match_format='bo3'` (new default): one MatchEnv episode contains
    2-3 inner games, each emitting its own row. Rows from the same match
    share ``match_idx`` and have contiguous ``game_in_match_idx`` 0..N-1.
    """
    pool, jsonl_path = _run_short_training(
        tmp_path,
        match_format="bo3",
        eval_game_log="on",
        run_name="test_bo3_e2e",
    )
    assert jsonl_path.exists(), f"expected eval game log at {jsonl_path}"

    rows = [
        json.loads(line)
        for line in jsonl_path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    # eval_episodes=2 matches × at least 2 inner games each = >=4 rows.
    assert len(rows) >= 4, (
        f"expected >=4 inner-game rows in BO3 mode from eval_episodes=2 × "
        f">=2 games per match; got {len(rows)}"
    )

    expected_archetypes = set(pool.archetype_names)
    for i, row in enumerate(rows):
        missing = [k for k in REQUIRED_FIELDS if k not in row]
        assert not missing, f"row {i} missing fields {missing}: {row!r}"
        assert row["match_format"] == "bo3"
        assert row["match_idx"] is not None, (
            f"row {i} BO3 row missing match_idx: {row!r}"
        )
        assert row["game_in_match_idx"] is not None
        assert 0 <= row["game_in_match_idx"] <= 2
        assert row["result"] in {"win", "loss", "draw"}
        assert row["agent_archetype"] in expected_archetypes

    # Group rows by match_idx within each window. Inner games per match
    # must have contiguous game_in_match_idx from 0.
    by_match: dict[tuple, list[int]] = {}
    for row in rows:
        key = (row["step"], row["eval_window_idx"], row["match_idx"])
        by_match.setdefault(key, []).append(row["game_in_match_idx"])
    for key, idxs in by_match.items():
        assert sorted(idxs) == list(range(len(idxs))), (
            f"match {key} has non-contiguous game_in_match_idx: {idxs}"
        )

    # Sanity-check: within one window, game_idx is contiguous across all
    # rows (i.e., it indexes games, not matches).
    by_window: dict[tuple, list[int]] = {}
    for row in rows:
        key = (row["step"], row["eval_window_idx"])
        by_window.setdefault(key, []).append(row["game_idx"])
    for key, game_idxs in by_window.items():
        assert sorted(game_idxs) == list(range(len(game_idxs))), (
            f"window {key} game_idx non-contiguous: {game_idxs}"
        )


@pytest.mark.slow
def test_eval_game_log_off_writes_nothing(tmp_path):
    """`eval_game_log='off'` MUST NOT create the file (both modes)."""
    _, jsonl_path = _run_short_training(
        tmp_path,
        match_format="bo3",
        eval_game_log="off",
        run_name="test_off",
    )
    assert not jsonl_path.exists(), (
        f"expected no eval game log when eval_game_log=off, found {jsonl_path}"
    )
