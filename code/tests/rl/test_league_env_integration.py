"""Integration test for the `--opponent league` env wiring in pilot_training
(add-deck-specialist-league).

Builds the real make_env league chain (engine-backed) and drives a reset + a few
steps, with the policy loader faked to `greedy_policy` so no model `.zip` is
needed. Verifies the LeaguePoolWrapper is in the chain and couples deck2 from the
sampled entry. Skips cleanly if deck-library data isn't present.
"""
from __future__ import annotations

import numpy as np
import pytest

from digimon_gym.agents import pilot_training as P
from digimon_gym.agents.league_opponent import LeaguePoolWrapper
from digimon_gym.agents.specialist_registry import LeaguePoolEntry

PROFILE = "standard_lite_deck_v2"


def _deck_cards(archetype: str):
    try:
        from digimon_gym.agents.gauntlet import load_generalist_deck_pool
        pool = load_generalist_deck_pool(allowed_archetypes={archetype})
    except FileNotFoundError:
        pytest.skip("deck_library.json not available")
    names = pool.archetype_names
    if not names:
        pytest.skip(f"archetype {archetype!r} resolved to no implemented decks")
    decks = sorted(pool.archetypes[names[0]], key=lambda d: d.deck_id)
    return list(decks[0].card_ids)


def _find_wrapper(env, cls):
    cur = env
    while cur is not None:
        if isinstance(cur, cls):
            return cur
        cur = getattr(cur, "env", None)
    return None


def test_make_env_league_couples_deck_and_policy(monkeypatch):
    deck1 = _deck_cards("ST-1 Gaia Red")
    deck2 = _deck_cards("ST-2 Cocytus Blue")

    # Fake the (policy) loader so no model file is needed — greedy stands in.
    monkeypatch.setattr(P, "_league_policy_loader", lambda wp, algo: P.greedy_policy)
    entries = [
        LeaguePoolEntry(name="st-2@r0", weights_path="unused.zip",
                        algorithm="mlp", deck=deck2),
    ]
    env = P.make_env(
        opponent="league",
        deck1=deck1,
        league_entries=entries,
        match_format="single",
        tensor_profile=PROFILE,
        curriculum_seed=7,
    )
    # The coupled-opponent wrapper is in the chain.
    lw = _find_wrapper(env, LeaguePoolWrapper)
    assert lw is not None, "LeaguePoolWrapper not found in the league env chain"

    obs, info = env.reset(seed=0)
    # The sampled opponent's deck was injected as deck2.
    assert info.get("opponent_deck") == deck2
    assert lw.current_opponent is not None

    # Drive a few legal agent actions; the coupled opponent plays its turns.
    dig = P._unwrap_to_digimon_env(env)
    for _ in range(5):
        mask = dig.action_mask()
        if not np.any(mask):
            break
        action = int(np.argmax(mask))
        obs, reward, terminated, truncated, info = env.step(action)
        if terminated or truncated:
            break


def test_make_env_league_requires_entries():
    with pytest.raises(ValueError, match="requires league_entries"):
        P.make_env(opponent="league", deck1=None, league_entries=None,
                   tensor_profile=PROFILE)


def test_resolve_league_entry_decks_names_to_cards():
    # The orchestrator writes opponent decks as archetype NAMES; pilot_training
    # must resolve them to card IDs before the env sees deck2.
    try:
        from digimon_gym.agents.gauntlet import load_generalist_deck_pool
        load_generalist_deck_pool(allowed_archetypes={"ST-1 Gaia Red"})
    except FileNotFoundError:
        pytest.skip("deck_library.json not available")
    entries = [LeaguePoolEntry(name="st-1@r0", weights_path="x.zip",
                               algorithm="mlp", deck="ST-1 Gaia Red")]
    P._resolve_league_entry_decks(entries)
    assert isinstance(entries[0].deck, list) and entries[0].deck
    assert all(isinstance(c, str) for c in entries[0].deck)

    # entries already holding card-ID lists are left untouched
    pre = ["BT1-001", "BT1-002"]
    e2 = [LeaguePoolEntry(name="x", weights_path="y", algorithm="mlp", deck=pre)]
    P._resolve_league_entry_decks(e2)
    assert e2[0].deck == pre
