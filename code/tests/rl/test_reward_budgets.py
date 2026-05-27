"""Pure-Python tests for the budget engine in
`digimon_gym.agents.reward.budget`. Covers spec scenarios for
"Per-component budget controls" and "Per-profile budget cap and floor".
"""

from __future__ import annotations

import pytest

from digimon_gym.agents.reward.budget import (
    PROFILE_BUDGET_EXEMPT_KINDS,
    ComponentBudget,
    ProfileBudget,
)


# ---- ComponentBudget: max_fires_per_episode ------------------------------


def test_max_fires_per_episode_caps_fire_count():
    # Spec scenario: weight=0.5, max_fires=2 → [+0.5, +0.5, 0.0]
    b = ComponentBudget(max_fires_per_episode=2)
    state: dict = {}
    series = [b.apply(0.5, "prof", "c", state) for _ in range(3)]
    assert series == [0.5, 0.5, 0.0]


def test_max_fires_does_not_consume_budget_when_raw_is_zero():
    # A zero raw fire shouldn't count as one of the budgeted fires.
    b = ComponentBudget(max_fires_per_episode=2)
    state: dict = {}
    out = [
        b.apply(0.5, "prof", "c", state),  # fire 1
        b.apply(0.0, "prof", "c", state),  # no-op
        b.apply(0.5, "prof", "c", state),  # fire 2
        b.apply(0.5, "prof", "c", state),  # blocked
    ]
    assert out == [0.5, 0.0, 0.5, 0.0]


# ---- ComponentBudget: diminishing_returns_factor -------------------------


def test_diminishing_returns_factor_reduces_successive_fires():
    # Spec scenario: weight=1.0, factor=0.5 → [+1.0, +0.5, +0.25, +0.125]
    b = ComponentBudget(diminishing_returns_factor=0.5)
    state: dict = {}
    series = [b.apply(1.0, "prof", "c", state) for _ in range(4)]
    assert series == [1.0, 0.5, 0.25, 0.125]


def test_diminishing_returns_factor_1_0_is_a_passthrough():
    b = ComponentBudget(diminishing_returns_factor=1.0)
    state: dict = {}
    series = [b.apply(1.0, "prof", "c", state) for _ in range(3)]
    assert series == [1.0, 1.0, 1.0]


# ---- ComponentBudget: max_total_per_episode ------------------------------


def test_max_total_per_episode_clamps_positive_weight():
    # Spec scenario: weight=1.0, max_total=1.5 → [+1.0, +0.5, 0.0]
    b = ComponentBudget(max_total_per_episode=1.5)
    state: dict = {}
    series = [b.apply(1.0, "prof", "c", state) for _ in range(3)]
    assert series == [1.0, 0.5, 0.0]


def test_max_total_per_episode_floors_negative_weight():
    # Spec scenario: weight=-1.5, max_total=-1.5 → [-1.5, 0.0]
    b = ComponentBudget(max_total_per_episode=-1.5)
    state: dict = {}
    series = [b.apply(-1.5, "prof", "c", state) for _ in range(2)]
    assert series == [-1.5, 0.0]


def test_max_total_combined_with_diminishing_returns():
    # Diminishing applies first; then max_total clamp.
    # weight=1.0, factor=0.5, max_total=1.2.
    # Raw series before clamp: 1.0, 0.5, 0.25, 0.125.
    # Cumulative: 1.0, 1.5 → 1.0+0.2=1.2 (clamp), 0.0, 0.0.
    # (Tolerate float drift on the 0.2 clamp residue.)
    b = ComponentBudget(max_total_per_episode=1.2, diminishing_returns_factor=0.5)
    state: dict = {}
    series = [b.apply(1.0, "prof", "c", state) for _ in range(4)]
    assert series[0] == pytest.approx(1.0)
    assert series[1] == pytest.approx(0.2)
    assert series[2] == 0.0
    assert series[3] == 0.0


# ---- ComponentBudget: state isolation per (profile, component) ----------


def test_component_budget_state_isolated_per_profile():
    b = ComponentBudget(max_fires_per_episode=1)
    state: dict = {}
    assert b.apply(1.0, "profA", "c", state) == 1.0
    assert b.apply(1.0, "profB", "c", state) == 1.0   # different profile, fresh budget
    assert b.apply(1.0, "profA", "c", state) == 0.0   # profA is now exhausted


def test_component_budget_state_isolated_per_component_key():
    b = ComponentBudget(max_fires_per_episode=1)
    state: dict = {}
    assert b.apply(1.0, "prof", "cA", state) == 1.0
    assert b.apply(1.0, "prof", "cB", state) == 1.0   # different key, fresh budget


# ---- ProfileBudget: per_episode_cap -------------------------------------


def test_per_episode_cap_clamps_last_fire():
    # Spec scenario: per_episode_cap=5.0, single component dna_digivolve
    # weight=4.0. Three fires → [+4.0, +1.0 (clamped from +4.0), 0.0]
    # with reward_breakdown_clamped on the second step = {"dna": 3.0}.
    pb = ProfileBudget(per_episode_cap=5.0)
    state: dict = {}

    # Step 1: 4.0 emits → cap remaining = 1.0 after.
    out, clamped = pb.apply(
        [("dna_digivolve", "dna", 4.0)], "prof", state,
    )
    assert out == [("dna_digivolve", "dna", 4.0)]
    assert clamped == {}

    # Step 2: 4.0 emits but cap leaves only 1.0 → emit 1.0, clamp 3.0.
    out, clamped = pb.apply(
        [("dna_digivolve", "dna", 4.0)], "prof", state,
    )
    assert out == [("dna_digivolve", "dna", 1.0)]
    assert clamped == {"dna": 3.0}

    # Step 3: cap exhausted → emit 0.0, clamp entire 4.0.
    out, clamped = pb.apply(
        [("dna_digivolve", "dna", 4.0)], "prof", state,
    )
    assert out == [("dna_digivolve", "dna", 0.0)]
    assert clamped == {"dna": 4.0}


def test_per_episode_floor_clamps_negative_contributions():
    pb = ProfileBudget(per_episode_floor=-2.0)
    state: dict = {}

    # Step 1: -1.5 emits → floor remaining = -0.5.
    out, clamped = pb.apply([("opp_deletion", "x", -1.5)], "prof", state)
    assert out == [("opp_deletion", "x", -1.5)]
    assert clamped == {}

    # Step 2: -1.5 would overshoot; clamp to -0.5.
    out, clamped = pb.apply([("opp_deletion", "x", -1.5)], "prof", state)
    assert out == [("opp_deletion", "x", -0.5)]
    assert clamped == {"x": -1.0}  # clamped away 1.0 magnitude

    # Step 3: floor exhausted.
    out, clamped = pb.apply([("opp_deletion", "x", -1.5)], "prof", state)
    assert out == [("opp_deletion", "x", 0.0)]
    assert clamped == {"x": -1.5}


def test_terminal_outcome_and_step_penalty_exempt_from_profile_cap():
    # Spec scenario: per_episode_cap=1.0, components include terminal,
    # step_penalty, and dna. Only `dna` is clamped at 1.0.
    pb = ProfileBudget(per_episode_cap=1.0)
    state: dict = {}

    contributions = [
        ("terminal_outcome", "terminal", 10.0),
        ("step_penalty", "step", -0.1),
        ("dna_digivolve", "dna", 4.0),
    ]
    out, clamped = pb.apply(contributions, "prof", state)

    # terminal_outcome and step_penalty passed through unmodified.
    assert ("terminal_outcome", "terminal", 10.0) in out
    assert ("step_penalty", "step", -0.1) in out
    # dna clamped to per_episode_cap = 1.0.
    assert ("dna_digivolve", "dna", 1.0) in out
    assert clamped == {"dna": 3.0}


def test_profile_budget_without_cap_or_floor_is_passthrough():
    pb = ProfileBudget()  # both None — no clamping
    state: dict = {}
    contributions = [("dna_digivolve", "dna", 4.0), ("opp_deletion", "x", -2.5)]
    out, clamped = pb.apply(contributions, "prof", state)
    assert out == contributions
    assert clamped == {}


def test_profile_budget_state_isolated_per_profile():
    pb = ProfileBudget(per_episode_cap=2.0)
    state: dict = {}
    out1, _ = pb.apply([("dna_digivolve", "x", 4.0)], "profA", state)
    out2, _ = pb.apply([("dna_digivolve", "x", 4.0)], "profB", state)
    # Each profile has its own fresh budget.
    assert out1 == [("dna_digivolve", "x", 2.0)]
    assert out2 == [("dna_digivolve", "x", 2.0)]


def test_zero_value_contribution_passes_through_unchanged():
    pb = ProfileBudget(per_episode_cap=1.0)
    state: dict = {}
    out, clamped = pb.apply([("dna_digivolve", "dna", 0.0)], "prof", state)
    assert out == [("dna_digivolve", "dna", 0.0)]
    assert clamped == {}


# ---- Spec invariant: exempt set ----------------------------------------


def test_exempt_kinds_includes_terminal_and_step_penalty():
    assert "terminal_outcome" in PROFILE_BUDGET_EXEMPT_KINDS
    assert "step_penalty" in PROFILE_BUDGET_EXEMPT_KINDS
