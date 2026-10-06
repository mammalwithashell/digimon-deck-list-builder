"""Shared test configuration.

The oracle readiness gate (gauntlet.py) is ON in production with no override.
Tests that build synthetic deck libraries are not about readiness, so by
default they see a ready set that contains every card. A test that exercises
the gate itself is marked `@pytest.mark.oracle_gate` and injects its own set
(or points `_ORACLE_READINESS_PATH` at a fixture artifact).
"""
import pytest


class _EveryCard(frozenset):
    def __contains__(self, item):  # every card is "ready"
        return True


@pytest.fixture(autouse=True)
def _oracle_gate_permissive_unless_marked(request, monkeypatch):
    if request.node.get_closest_marker("oracle_gate"):
        return
    try:
        import digimon_gym.agents.gauntlet as gauntlet
    except ImportError:  # engine binding not built in this environment
        return
    monkeypatch.setattr(gauntlet, "_load_oracle_ready_card_ids", lambda path=None: _EveryCard())
