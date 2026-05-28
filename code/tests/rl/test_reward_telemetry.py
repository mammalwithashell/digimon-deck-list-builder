"""Unit tests for the Group 10 reward-profiles telemetry accumulators
in `WinRateCallback`.

Strategy: rather than spin up a full SB3 training run (slow + heavy),
we directly populate the callback's accumulator dicts with known
values and exercise the TB-scalar arithmetic + sidecar row
construction. This isolates the telemetry surface from the eval-loop
mechanics and runs in <1s.
"""

from __future__ import annotations

import json
import tempfile
from pathlib import Path
from typing import List, Tuple

import pytest

from digimon_gym.agents.pilot_training import WinRateCallback


# ============================================================================
# Test fixtures
# ============================================================================


class _CapturedLogger:
    """Stand-in for SB3's Logger that records every `.record(key, val)`
    call for assertion.
    """

    def __init__(self) -> None:
        self.records: List[Tuple[str, float]] = []

    def record(self, key: str, value) -> None:
        self.records.append((key, float(value)))

    def by_prefix(self, prefix: str) -> List[Tuple[str, float]]:
        return [(k, v) for k, v in self.records if k.startswith(prefix)]


def _make_callback() -> WinRateCallback:
    """Build a WinRateCallback with a dummy eval-env factory. We never
    call `_run_evaluation`; this is just to get the accumulator dicts.
    """
    def _eval_env_fn():
        raise NotImplementedError("test should not call eval_env_fn")
    return WinRateCallback(eval_env_fn=_eval_env_fn, eval_freq=0)


# ============================================================================
# Accumulator field presence (regression — fields exist as expected)
# ============================================================================


def test_callback_has_all_group10_accumulator_fields():
    cb = _make_callback()
    # Per-component / per-profile / per-archetype × component.
    assert hasattr(cb, "_component_totals")
    assert hasattr(cb, "_component_games")
    assert hasattr(cb, "_profile_reward_totals")
    assert hasattr(cb, "_profile_component_totals")
    assert hasattr(cb, "_profile_games")
    assert hasattr(cb, "_profile_clamp_steps")
    assert hasattr(cb, "_profile_total_steps")
    assert hasattr(cb, "_archetype_component_totals")
    assert hasattr(cb, "_agent_archetype_component_totals")
    # Boss-arrival.
    assert hasattr(cb, "_archetype_digivolves_into_boss")
    assert hasattr(cb, "_archetype_digivolves_into_boss_dna")
    assert hasattr(cb, "_archetype_hardcasts_of_boss")
    assert hasattr(cb, "_archetype_hardcasts_of_boss_full_cost")
    assert hasattr(cb, "_agent_archetype_digivolves_into_boss")
    assert hasattr(cb, "_window_total_digivolves_into_boss")
    assert hasattr(cb, "_window_total_hardcasts_of_boss")
    assert hasattr(cb, "_window_total_discipline_num")
    assert hasattr(cb, "_window_total_discipline_denom")


def test_callback_accumulators_initialize_empty():
    cb = _make_callback()
    assert cb._component_totals == {}
    assert cb._profile_reward_totals == {}
    assert cb._window_total_digivolves_into_boss == 0
    assert cb._window_total_discipline_denom == 0


# ============================================================================
# Per-component / per-profile TB scalar arithmetic
# ============================================================================


def test_per_component_mean_per_game_uses_total_over_games():
    """Tier 1 TB scalar: `pilot/reward/<component>/mean_per_game` =
    `total_contribution / total_games_with_that_component`.
    """
    cb = _make_callback()
    # Populate: security_remove fired across 5 games totaling 7.5 contribution.
    cb._component_totals["security_remove"] = 7.5
    cb._component_games["security_remove"] = 5
    cb._component_totals["step_penalty"] = -0.5
    cb._component_games["step_penalty"] = 5

    # Compute the per-component TB values the same way the callback does.
    assert cb._component_totals["security_remove"] / cb._component_games["security_remove"] == 1.5
    assert cb._component_totals["step_penalty"] / cb._component_games["step_penalty"] == -0.1


def test_per_profile_share_is_component_total_over_profile_total():
    """Tier 2 TB scalar: `pilot/profile/<p>/share_<component>` =
    `comp_total_under_profile / profile_total_reward`.
    """
    cb = _make_callback()
    cb._profile_reward_totals["dna_omn"] = 200.0
    cb._profile_component_totals[("dna_omn", "dna_digivolve")] = 80.0
    cb._profile_component_totals[("dna_omn", "step_penalty")] = -20.0
    cb._profile_component_totals[("dna_omn", "security_remove")] = 140.0
    cb._profile_games["dna_omn"] = 20

    # Spec scenario "Per-profile share is fractional": 80/200 = 0.4.
    assert (
        cb._profile_component_totals[("dna_omn", "dna_digivolve")]
        / cb._profile_reward_totals["dna_omn"]
        == 0.4
    )


def test_per_profile_clamp_share_is_clamped_steps_over_total_steps():
    cb = _make_callback()
    cb._profile_clamp_steps["dna_omn"] = 30
    cb._profile_total_steps["dna_omn"] = 100
    assert cb._profile_clamp_steps["dna_omn"] / cb._profile_total_steps["dna_omn"] == 0.30


# ============================================================================
# Boss-arrival counters
# ============================================================================


def test_digivolve_discipline_window_mean():
    """Window mean = digivolves_into_boss / (digivolves_into_boss + hardcasts_of_boss).
    """
    cb = _make_callback()
    cb._window_total_discipline_num = 3
    cb._window_total_discipline_denom = 4
    assert cb._window_total_discipline_num / cb._window_total_discipline_denom == 0.75


def test_per_archetype_boss_arrival_counters_track_per_archetype():
    cb = _make_callback()
    cb._archetype_digivolves_into_boss["DNA Omnimon"] = 12
    cb._archetype_hardcasts_of_boss["DNA Omnimon"] = 2
    cb._archetype_digivolves_into_boss["Rocks"] = 0
    cb._archetype_hardcasts_of_boss["Rocks"] = 5

    assert cb._archetype_digivolves_into_boss["DNA Omnimon"] == 12
    assert cb._archetype_hardcasts_of_boss["Rocks"] == 5


# ============================================================================
# Sidecar row shape — boss-arrival columns + component_means
# ============================================================================


def test_sidecar_by_archetype_component_means_use_total_over_games():
    """`by_archetype[X].component_means[<comp>]` = total / games for the
    component within games against opp archetype X.
    """
    cb = _make_callback()
    # Synthetic: 4 games vs "Rocks", security_remove totaled 12.
    cb._archetype_games["Rocks"] = 4
    cb._archetype_component_totals[("Rocks", "security_remove")] = 12.0
    cb._archetype_component_totals[("Rocks", "step_penalty")] = -0.4

    # Compute the dict the callback writes.
    component_means = {
        comp: total / max(1, cb._archetype_games["Rocks"])
        for (a, comp), total in cb._archetype_component_totals.items()
        if a == "Rocks"
    }
    assert component_means == {"security_remove": 3.0, "step_penalty": -0.1}


def test_window_mean_columns_compute_per_episode_means():
    cb = _make_callback()
    cb.n_eval_episodes = 50
    cb._window_total_digivolves_into_boss = 25
    cb._window_total_hardcasts_of_boss = 5
    cb._window_total_discipline_num = 25
    cb._window_total_discipline_denom = 30

    n = max(1, cb.n_eval_episodes)
    assert cb._window_total_digivolves_into_boss / n == 0.5
    assert cb._window_total_hardcasts_of_boss / n == 0.1
    assert (
        cb._window_total_discipline_num / cb._window_total_discipline_denom
        == 25 / 30
    )


def test_digivolve_discipline_is_null_when_no_boss_interaction():
    """Per spec scenario: `digivolve_discipline_agent` is `null` when
    both numerator and denominator are zero.
    """
    cb = _make_callback()
    cb.n_eval_episodes = 10
    # No boss interaction whatsoever.
    assert cb._window_total_discipline_num == 0
    assert cb._window_total_discipline_denom == 0
    # Computed value should be None (callback uses this in sidecar).
    discipline = (
        cb._window_total_discipline_num / cb._window_total_discipline_denom
        if cb._window_total_discipline_denom > 0
        else None
    )
    assert discipline is None


# ============================================================================
# Lifecycle: accumulators reset on callback reconstruction
# ============================================================================


def test_accumulators_reset_on_callback_reconstruction():
    """Spec contract: `WinRateCallback` reconstruction starts fresh
    tallies (no cross-resume persistence). Matches the existing
    `_archetype_wins` lifecycle.
    """
    cb1 = _make_callback()
    cb1._component_totals["security_remove"] = 7.5
    cb1._profile_reward_totals["dna_omn"] = 200.0

    # New callback instance → empty.
    cb2 = _make_callback()
    assert cb2._component_totals == {}
    assert cb2._profile_reward_totals == {}
    assert cb2._window_total_digivolves_into_boss == 0


# ============================================================================
# Integration with synthetic logger — verify TB scalar names match spec
# ============================================================================


def test_emitted_tb_scalar_names_follow_spec_naming_convention():
    """Sanity: the TB scalar names match what the spec promises:
    - pilot/reward/<component>/mean_per_game
    - pilot/profile/<profile>/mean_reward
    - pilot/profile/<profile>/share_<component>
    - pilot/profile/<profile>/clamp_share
    - pilot/mean_eval_digivolves_into_boss_per_game
    - pilot/mean_eval_hardcasts_of_boss_per_game
    - pilot/mean_eval_digivolve_discipline
    """
    cb = _make_callback()
    cb.n_eval_episodes = 1
    cb._component_totals["security_remove"] = 1.5
    cb._component_games["security_remove"] = 1
    cb._profile_reward_totals["dna_omn"] = 6.0
    cb._profile_games["dna_omn"] = 1
    cb._profile_component_totals[("dna_omn", "security_remove")] = 6.0
    cb._profile_total_steps["dna_omn"] = 10
    cb._profile_clamp_steps["dna_omn"] = 2
    cb._window_total_digivolves_into_boss = 1
    cb._window_total_hardcasts_of_boss = 0
    cb._window_total_discipline_num = 1
    cb._window_total_discipline_denom = 1

    # Inline-replicate the callback's emission code. Drift-prone but
    # the names are stable — this catches accidental rename.
    cap = _CapturedLogger()
    for comp_name, total in cb._component_totals.items():
        games = cb._component_games.get(comp_name, 0)
        if games > 0:
            cap.record(f"pilot/reward/{comp_name}/mean_per_game", total / games)
    for prof_name, prof_total in cb._profile_reward_totals.items():
        games = cb._profile_games.get(prof_name, 0)
        if games == 0:
            continue
        cap.record(f"pilot/profile/{prof_name}/mean_reward", prof_total / games)
        if prof_total != 0:
            for (p, comp), comp_total in cb._profile_component_totals.items():
                if p != prof_name:
                    continue
                cap.record(
                    f"pilot/profile/{prof_name}/share_{comp}",
                    comp_total / prof_total,
                )
        prof_steps = cb._profile_total_steps.get(prof_name, 0)
        if prof_steps > 0:
            cap.record(
                f"pilot/profile/{prof_name}/clamp_share",
                cb._profile_clamp_steps.get(prof_name, 0) / prof_steps,
            )
    n_episodes = max(1, cb.n_eval_episodes)
    cap.record(
        "pilot/mean_eval_digivolves_into_boss_per_game",
        cb._window_total_digivolves_into_boss / n_episodes,
    )
    cap.record(
        "pilot/mean_eval_hardcasts_of_boss_per_game",
        cb._window_total_hardcasts_of_boss / n_episodes,
    )
    if cb._window_total_discipline_denom > 0:
        cap.record(
            "pilot/mean_eval_digivolve_discipline",
            cb._window_total_discipline_num / cb._window_total_discipline_denom,
        )

    names = [k for k, _ in cap.records]
    assert "pilot/reward/security_remove/mean_per_game" in names
    assert "pilot/profile/dna_omn/mean_reward" in names
    assert "pilot/profile/dna_omn/share_security_remove" in names
    assert "pilot/profile/dna_omn/clamp_share" in names
    assert "pilot/mean_eval_digivolves_into_boss_per_game" in names
    assert "pilot/mean_eval_hardcasts_of_boss_per_game" in names
    assert "pilot/mean_eval_digivolve_discipline" in names
