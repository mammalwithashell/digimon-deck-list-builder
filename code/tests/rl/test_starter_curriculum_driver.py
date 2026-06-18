"""Unit tests for the starter-deck curriculum driver (tools/train_starter_curriculum).

These assert the two phase command-lines and, critically, the phase-1 -> phase-2
handoff (warm-start checkpoint + reused deck-pool snapshot). They do NOT launch
training, so they run without the engine wheel or a GPU.
"""
from __future__ import annotations

from typing import List, Optional

import pytest

from tools.train_starter_curriculum import (
    STARTER_ARCHETYPES,
    CurriculumSpec,
    build_phase1_argv,
    build_phase2_argv,
)


def _val(argv: List[str], flag: str) -> Optional[str]:
    """Return the token following ``flag`` (None if the flag is absent)."""
    return argv[argv.index(flag) + 1] if flag in argv else None


def _sets(argv: List[str]) -> dict:
    """Collect all ``--set key=value`` overrides into a dict."""
    out = {}
    for i, tok in enumerate(argv):
        if tok == "--set":
            k, v = argv[i + 1].split("=", 1)
            out[k] = v
    return out


@pytest.fixture
def spec() -> CurriculumSpec:
    return CurriculumSpec(pool_manifest="pool_starters.json")


def test_phase1_is_fresh_greedy_over_six_starters(spec):
    argv = build_phase1_argv(spec)
    assert "--generalist" in argv
    assert _val(argv, "--opponent") == "greedy"
    assert _val(argv, "--timesteps") == "1000000"
    # All six starters present, comma-joined, no extras.
    archetypes = _val(argv, "--archetypes").split(",")
    assert archetypes == STARTER_ARCHETYPES
    assert len(STARTER_ARCHETYPES) == 6
    # A fresh floor run must NOT warm-start or pin a prior pool.
    assert "--init-from" not in argv
    assert "--curriculum-pool" not in argv
    assert "--opponent-pool-manifest" not in argv


def test_phase2_swaps_to_pool_and_warm_starts(spec):
    argv = build_phase2_argv(spec)
    assert _val(argv, "--opponent") == "pool"
    assert _val(argv, "--opponent-pool-manifest") == "pool_starters.json"
    assert _val(argv, "--timesteps") == "5000000"
    # In curriculum-pool mode the snapshot defines the decks; no --archetypes.
    assert "--archetypes" not in argv


def test_phase2_handoff_matches_phase1_outputs(spec):
    """init-from + curriculum-pool must point at phase 1's deterministic outputs."""
    p2 = build_phase2_argv(spec)
    assert _val(p2, "--init-from") == str(spec.floor_final())
    assert _val(p2, "--curriculum-pool") == str(spec.floor_snapshot())
    # And those are the pinned run_name locations under save_dir.
    assert spec.floor_final().as_posix().endswith("models/starter_floor_v1/final.zip")
    assert spec.floor_snapshot().as_posix().endswith(
        "models/starter_floor_v1/deck_pool_snapshot.json"
    )


def test_both_phases_share_profile_reward_and_seeds(spec):
    """The two phases must agree on tensor profile, reward, and curriculum seeds,
    or the warm-start contract / deck curriculum diverges."""
    p1, p2 = build_phase1_argv(spec), build_phase2_argv(spec)
    s1, s2 = _sets(p1), _sets(p2)
    assert s1["tensor_profile"] == s2["tensor_profile"] == "standard_lite_deck_v2"
    assert s1["reward_profile_override"] == s2["reward_profile_override"] == "starter1_6_flat"
    assert s1["run_name"] == "starter_floor_v1"
    assert s2["run_name"] == "starter_pool_v1"
    assert _val(p1, "--curriculum-seed") == _val(p2, "--curriculum-seed") == "123"
    assert _val(p1, "--eval-seed") == _val(p2, "--eval-seed") == "999"
    assert _val(p1, "--match-format") == _val(p2, "--match-format") == "bo3"


def test_distinct_learning_rates_and_log_dirs(spec):
    """Phase 2 drops LR (harder opponent) and logs to its own TB dir."""
    p1, p2 = build_phase1_argv(spec), build_phase2_argv(spec)
    assert float(_val(p1, "--lr")) == 3e-4
    assert float(_val(p2, "--lr")) == 1e-4
    assert _val(p1, "--log-dir") == "runs/starter_floor_v1"
    assert _val(p2, "--log-dir") == "runs/starter_pool_v1"


def test_eval_games_recorded_by_default_both_phases(spec):
    """Future runs must log eval games (with action sequences) for investigation."""
    for argv in (build_phase1_argv(spec), build_phase2_argv(spec)):
        assert _val(argv, "--record-games") == "eval"
        assert int(_val(argv, "--record-games-max")) >= 100


def test_floor_envs_default_adds_no_parallelism(spec):
    """Default (floor_envs=1) must not inject n_envs/backend into either phase."""
    assert "n_envs" not in _sets(build_phase1_argv(spec))
    assert "n_envs" not in _sets(build_phase2_argv(spec))


def test_floor_envs_parallelizes_floor_phase_only():
    """floor_envs>1 parallelizes the greedy floor; the pool phase stays single-env
    (the engine rejects n_envs>1 for pool opponents)."""
    spec = CurriculumSpec(pool_manifest="pool_starters.json", floor_envs=8)
    s1 = _sets(build_phase1_argv(spec))
    s2 = _sets(build_phase2_argv(spec))
    assert s1["n_envs"] == "8"
    assert s1["vec_env_backend"] == "subproc"
    assert "n_envs" not in s2  # pool phase must NOT be parallelized
