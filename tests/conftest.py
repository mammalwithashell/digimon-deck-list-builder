"""Root conftest.py — shared fixtures for all tests.

Provides:
- reset_registry: autouse fixture that resets CardRegistry between tests
- debug_runner: factory fixture for creating DebugRunner with archetype decks
"""

import json
import pytest

from digimon_gym.data_paths import DECK_LIBRARY
from digimon_gym.engine.data.card_registry import CardRegistry


@pytest.fixture(autouse=True)
def reset_registry():
    """Reset CardRegistry before and after each test for isolation."""
    CardRegistry.reset()
    yield
    CardRegistry.reset()


@pytest.fixture
def debug_runner():
    """Factory fixture for creating DebugRunner from archetype names or card lists.

    Usage:
        runner = debug_runner(archetype1="Puppets", initial_memory=10)
        runner = debug_runner(deck1=[...], deck2=[...], skip_shuffle=True)
    """
    from digimon_gym.engine.runners.debug_runner import DebugRunner

    _cache = {}

    def _load_deck(archetype_name: str) -> list[str]:
        if archetype_name not in _cache:
            with open(DECK_LIBRARY, "r", encoding="utf-8") as f:
                library = json.load(f)
            arch = library["archetypes"].get(archetype_name)
            if not arch or not arch.get("decklists"):
                raise ValueError(f"No decklists for archetype: {archetype_name}")
            _cache[archetype_name] = json.loads(arch["decklists"][0]["decklist"])
        return list(_cache[archetype_name])

    def _create(
        deck1=None,
        deck2=None,
        archetype1=None,
        archetype2=None,
        **kwargs,
    ) -> DebugRunner:
        d1 = deck1 or _load_deck(archetype1 or "Puppets")
        d2 = deck2 or _load_deck(archetype2 or "Puppets")
        return DebugRunner(d1, d2, **kwargs)

    return _create
