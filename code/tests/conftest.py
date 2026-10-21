"""Shared test configuration.

The training gate (gauntlet.py, spec 2026-10-15 §5b) is ON in production with no
override: the mechanics layer (`data/mechanics_readiness.json` must be ready)
and the light per-card layer (`data/card_admissibility.json`). Tests that build
synthetic deck libraries are not about the gate, so by default they see an open
mechanics gate and an admissibility that admits every card (and, for the
`--legacy-clause-gate` path, an oracle-ready set that contains every card). A
test that exercises the gate itself is marked `@pytest.mark.oracle_gate` and
injects its own artifacts (pass the objects, or point `_MECHANICS_READINESS_PATH`
/ `_CARD_ADMISSIBILITY_PATH` / `_ORACLE_READINESS_PATH` at fixture files).
"""
import pytest


class _EveryCard(frozenset):
    def __contains__(self, item):  # every card is "ready"
        return True


class _AdmitsEveryCard:
    """A permissive `CardAdmissibility` stand-in: every card is admissible."""

    def admissible(self, card_id):
        return True

    def reason(self, card_id):
        return None

    def reasons(self, card_id):
        return []


_MECHANICS_READY = {"version": 1, "counts": {}, "blocking": [], "ready": True}


@pytest.fixture(autouse=True)
def _oracle_gate_permissive_unless_marked(request, monkeypatch):
    if request.node.get_closest_marker("oracle_gate"):
        return
    try:
        import digimon_gym.agents.gauntlet as gauntlet
    except ImportError:  # engine binding not built in this environment
        return
    monkeypatch.setattr(gauntlet, "_load_mechanics_readiness", lambda path=None: dict(_MECHANICS_READY))
    monkeypatch.setattr(gauntlet, "_load_card_admissibility", lambda path=None: _AdmitsEveryCard())
    monkeypatch.setattr(gauntlet, "_load_oracle_ready_card_ids", lambda path=None: _EveryCard())
