"""Tests for anchored evaluation — seat-balanced play of a candidate model
against fixed anchors (greedy + frozen champions).

Pure pieces (record arithmetic, seat-balanced seed parity) are unit-tested.
A slow integration test guards against the passive-opponent trap: a mirror
matchup must land away from 0%/100% (i.e. the opponent actually contests).
Part of add-model-evaluation-harness.
"""
from pathlib import Path

import pytest

from digimon_gym.agents.anchored_eval import (
    MatchRecord, _seat_balanced_seed, play_matchup,
)

ROOT = Path(__file__).resolve().parents[3]
V22 = ROOT / "cloud_downloads" / "v022-hf4zm2hl82qk48" / "models" / "pilot_ppo_starter_decks_generalist_v1.zip"
SNAPSHOT = ROOT / "cloud_downloads" / "v022-hf4zm2hl82qk48" / "models" / "pilot_ppo_20260531_073321" / "deck_pool_snapshot.json"


# ---- pure: record arithmetic ----

def test_match_record_games_and_winrate():
    r = MatchRecord(wins=6, losses=3, draws=1)
    assert r.games == 10
    assert r.win_rate == pytest.approx(0.6)


def test_match_record_empty_winrate_is_zero():
    assert MatchRecord().win_rate == 0.0


def test_match_record_addition():
    total = MatchRecord(7, 2, 1) + MatchRecord(4, 5, 1)
    assert (total.wins, total.losses, total.draws) == (11, 7, 2)
    assert total.win_rate == pytest.approx(11 / 20)


# ---- pure: seat-balancing seed parity (first player must alternate) ----

def test_seat_balanced_seed_alternates_parity():
    seeds = [_seat_balanced_seed(777, i) for i in range(6)]
    parities = [s % 2 for s in seeds]
    assert parities == [0, 1, 0, 1, 0, 1]


# ---- integration: a mirror matchup must NOT be degenerate ----
# Catches the passive-opponent bug (where the first player always wins, making
# a model-vs-itself land at exactly 0% or 100% instead of ~50%).

@pytest.mark.slow
def test_anchored_eval_discriminates_greedy_from_mirror():
    """A trained model should clearly beat greedy while ~tying an identical
    copy of itself. The passive-opponent bug makes BOTH ~50% (the opponent
    never contests, so first-player decides everything) — so asserting
    greedy >> mirror is what actually catches it."""
    if not (V22.exists() and SNAPSHOT.exists()):
        pytest.skip("champion model / deck-pool snapshot not present locally")
    from digimon_gym.agents.pilot_training import MaskableRecurrentPPO, make_agent_opponent_fn
    from digimon_gym.agents.gauntlet import GeneralistDeckPool
    from digimon_gym.digimon_gym import greedy_policy
    from digimon_engine import load_implemented_card_ids

    model = MaskableRecurrentPPO.load(str(V22), device="cpu")
    pool = GeneralistDeckPool.from_snapshot(SNAPSHOT, implemented_card_ids=load_implemented_card_ids())

    # Deterministic (fixed seed, deterministic policies, CPU==GPU verified).
    vs_greedy = play_matchup(model, greedy_policy, pool, n=12, base_seed=777,
                             tensor_profile="standard_lite_deck_v2")
    vs_self = play_matchup(model, make_agent_opponent_fn(str(V22), algorithm="lstm"),
                           pool, n=12, base_seed=777, tensor_profile="standard_lite_deck_v2")

    assert vs_greedy.games == 12 and vs_self.games == 12
    # Mirror must be contested, not a 0%/100% blowout (passive opponent).
    assert 0 < vs_self.win_rate < 1, f"mirror degenerate: {vs_self}"
    # The decisive discriminator vs the passive-opponent bug: beating greedy is
    # clearly easier than beating an identical copy of yourself. The bug pins
    # BOTH at ~50% (first player decides), so this strict inequality fails.
    assert vs_greedy.win_rate > vs_self.win_rate, (
        f"opponent not contesting? vs_greedy={vs_greedy} vs_self={vs_self}")
