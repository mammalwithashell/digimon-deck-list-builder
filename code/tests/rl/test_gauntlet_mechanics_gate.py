"""The two-layer training gate in gauntlet.py (Plan 2 Task 3; spec 2026-10-15 §5b).

Layer 1 -- mechanics: `data/mechanics_readiness.json` must be `ready`, checked at
library load before any deck is examined (`MechanicsNotReadyError`).
Layer 2 -- light per-card: every card of a decklist must be admissible in
`data/card_admissibility.json` (rejection reason `not_admissible`). An unmeasured
clause does not block. `legacy_clause_gate=True` only ADDS the old per-card
oracle-readiness requirement on top.

Every test here is `oracle_gate`: the conftest's permissive default stays out of
the way and each test injects its own artifacts (objects, or tmp files the path
constants point at).
"""
import json
import warnings

import pytest

import digimon_gym.agents.gauntlet as gauntlet_module
from digimon_gym.agents.card_admissibility import CardAdmissibility
from digimon_gym.agents.gauntlet import GeneralistDeckPool, MetaGauntlet

pytestmark = pytest.mark.oracle_gate

REGISTERED = {"BT12-002", "BT12-022", "BT12-031"}
MECHANICS_CMD = "PYTHONPATH=code python -m tools.rules.mechanics_readiness"
ADMISSIBILITY_CMD = "PYTHONPATH=code python -m digimon_gym.agents.card_admissibility"

READY = {
    "version": 1,
    "counts": {"proven": 245, "not_testable": 152, "gap_blocking": 0, "gap_nonblocking": 318, "total": 715},
    "blocking": [],
    "ready": True,
}
NOT_READY = {
    "version": 1,
    "counts": {"proven": 240, "not_testable": 152, "gap_blocking": 2, "gap_nonblocking": 318, "total": 712},
    "blocking": [
        {"rule": "15-7-4", "gap_id": "G-ENGINE-SECURITY-CHECK", "mechanisms": ["security"]},
        {"rule": "16-37", "gap_id": "G-ENGINE-PIERCE", "mechanisms": ["keywords"]},
    ],
    "ready": False,
}


# ─── Fixtures ────────────────────────────────────────────────────────

def _deck(*swap):
    """55 cards like the gauntlet tests' decks, with main-deck copies swapped for `swap`."""
    return ["BT12-022"] * (50 - len(swap)) + list(swap) + ["BT12-002"] * 5


def _library(tmp_path, decks):
    """A one-archetype library ("Mixed") holding `decks` as mixed_000, mixed_001, ..."""
    lib = {
        "version": 2,
        "archetypes": {
            "Mixed": {
                "archetype_name": "Mixed",
                "digilab_stats": {"times_played": 10, "conversion_rate": 0.2, "win_rate": 0.5},
                "decklists": [
                    {"deck_id": f"mixed_{i:03d}", "source": "test", "decklist": json.dumps(d)}
                    for i, d in enumerate(decks)
                ],
            }
        },
    }
    path = tmp_path / "lib.json"
    path.write_text(json.dumps(lib))
    return path


def _admissibility(*, untested=(), failed=(), diverged=(), registered=REGISTERED):
    return CardAdmissibility(
        registered=frozenset(registered),
        lowering_covered=frozenset(set(registered) - set(untested)),
        failed_verdict=frozenset(failed),
        diverged_no_quirk=frozenset(diverged),
    )


def _write_admissibility(tmp_path, adm):
    path = tmp_path / "card_admissibility.json"
    path.write_text(json.dumps(adm.build_artifact(sources={}, generated_at="2026-10-21T00:00:00Z")))
    return path


def _write_json(tmp_path, name, payload):
    path = tmp_path / name
    path.write_text(json.dumps(payload))
    return path


class _ExplodesOnUse:
    """An admissibility that fails the test if any card is examined."""

    def admissible(self, card_id):
        raise AssertionError(f"examined {card_id} before the mechanics gate")

    reason = reasons = admissible


@pytest.fixture
def artifacts(tmp_path, monkeypatch):
    """Point the gauntlet's two run-time artifacts at tmp files; returns a writer."""

    def write(*, mechanics=READY, admissibility=None):
        monkeypatch.setattr(
            gauntlet_module, "_MECHANICS_READINESS_PATH",
            _write_json(tmp_path, "mechanics_readiness.json", mechanics),
        )
        monkeypatch.setattr(
            gauntlet_module, "_CARD_ADMISSIBILITY_PATH",
            _write_admissibility(tmp_path, admissibility or _admissibility()),
        )

    return write


# ─── The six brief tests ─────────────────────────────────────────────

def test_mechanics_not_ready_fails_fast_before_any_deck(tmp_path):
    g = MetaGauntlet(
        implemented_card_ids=REGISTERED,
        mechanics_readiness=NOT_READY,
        admissibility=_ExplodesOnUse(),
    )
    # Raised before the library is even opened: a missing library path would
    # otherwise surface as FileNotFoundError.
    with pytest.raises(gauntlet_module.MechanicsNotReadyError):
        g.load(str(tmp_path / "absent_library.json"))
    with pytest.raises(gauntlet_module.MechanicsNotReadyError):
        g.load(str(_library(tmp_path, [_deck()])))
    assert g.deck_count == 0


def test_admissible_deck_admitted_with_unmeasured_clauses(tmp_path, monkeypatch):
    # BT12-031 has an unmeasured clause and is not oracle-ready, but is
    # admissible: an unmeasured clause blocks nothing (§5b).
    exam = {
        "card_id": "BT12-031",
        "clauses": {
            "BT12-031#effect#0": {"card_id": "BT12-031", "verdict": "unmeasured", "reason": ""},
            "BT12-031#effect#1": {"card_id": "BT12-031", "verdict": "confirmed", "reason": ""},
        },
        "interactions": {},
    }
    adm = CardAdmissibility.load(
        registry_ids=REGISTERED, card_test_ids=REGISTERED,
        lowering_cards={cid: {"untested_constructs": [], "clause_count": 1} for cid in REGISTERED},
        verdicts_dsl={"cards": {}}, verdicts_rust={"cards": {}}, exam_verdicts=[exam],
    )
    assert adm.admissible("BT12-031")
    readiness = _write_json(tmp_path, "oracle_readiness.json", {"version": 1, "cards": {
        "BT12-002": {"status": "ready"}, "BT12-022": {"status": "ready"},
        "BT12-031": {"status": "not_ready"},
    }})
    monkeypatch.setattr(gauntlet_module, "_ORACLE_READINESS_PATH", readiness)

    g = MetaGauntlet(implemented_card_ids=REGISTERED, mechanics_readiness=READY, admissibility=adm)
    g.load(str(_library(tmp_path, [_deck(), _deck("BT12-031")])))

    assert [d.source_deck_id for d in g._deck_pool] == ["mixed_000", "mixed_001"]


def test_inadmissible_card_rejects_deck_with_reason(tmp_path, caplog):
    adm = _admissibility(diverged={"BT12-031"})
    path = _library(tmp_path, [_deck(), _deck("BT12-031"), _deck("BT99-999")])

    with caplog.at_level("DEBUG", logger="digimon_gym.agents.gauntlet"):
        g = MetaGauntlet(
            implemented_card_ids=REGISTERED, mechanics_readiness=READY, admissibility=adm,
            allowed_archetypes={"Mixed"},
        )
        g.load(str(path))

    assert [d.source_deck_id for d in g._deck_pool] == ["mixed_000"]
    messages = [r.getMessage() for r in caplog.records]
    assert any(
        "not-admissible" in m and "mixed_001" in m and "BT12-031 (diverged_no_quirk)" in m
        for m in messages
    ), messages
    # `unregistered` still catches a card the engine does not know (cheap, first).
    assert any("unimplemented" in m and "mixed_002" in m for m in messages), messages

    # The deck-level validator gives the same verdict with the reason.
    with pytest.raises(gauntlet_module.NotAdmissibleDeckError, match=r"BT12-031 \(diverged_no_quirk\)"):
        gauntlet_module.validate_admissible_deck(_deck("BT12-031"), adm, label="deck x")
    gauntlet_module.validate_admissible_deck(_deck(), adm, label="deck y")
    # The old name is a deprecated alias for one release.
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        with pytest.raises(gauntlet_module.NotAdmissibleDeckError):
            gauntlet_module.validate_oracle_ready_deck(_deck("BT12-031"), adm, label="deck z")
    assert any(issubclass(w.category, DeprecationWarning) for w in caught)


def test_snapshot_load_applies_light_gate(tmp_path, artifacts):
    g = MetaGauntlet(implemented_card_ids=REGISTERED, mechanics_readiness=READY, admissibility=_admissibility())
    g.load(str(_library(tmp_path, [_deck(), _deck("BT12-031")])))
    snapshot = tmp_path / "pool.json"
    written = g.as_generalist_pool().write_snapshot(snapshot)

    # Injected objects.
    with pytest.raises(gauntlet_module.NotAdmissibleDeckError, match=r"BT12-031 \(failed_verdict\)"):
        GeneralistDeckPool.from_snapshot(
            snapshot, implemented_card_ids=REGISTERED,
            mechanics_readiness=READY, admissibility=_admissibility(failed={"BT12-031"}),
        )
    with pytest.raises(gauntlet_module.MechanicsNotReadyError):
        GeneralistDeckPool.from_snapshot(
            snapshot, implemented_card_ids=REGISTERED,
            mechanics_readiness=NOT_READY, admissibility=_ExplodesOnUse(),
        )

    # Nothing injected (eval CLIs, --curriculum-pool): the committed artifacts decide.
    artifacts(admissibility=_admissibility(untested={"BT12-031"}))
    with pytest.raises(gauntlet_module.NotAdmissibleDeckError, match=r"BT12-031 \(untested_lowering\)"):
        GeneralistDeckPool.from_snapshot(snapshot, implemented_card_ids=REGISTERED)
    artifacts()
    assert GeneralistDeckPool.from_snapshot(snapshot, implemented_card_ids=REGISTERED).snapshot_hash == written
    artifacts(mechanics=NOT_READY)
    with pytest.raises(gauntlet_module.MechanicsNotReadyError):
        GeneralistDeckPool.from_snapshot(snapshot, implemented_card_ids=REGISTERED)


def test_legacy_clause_gate_restores_old_rejections(tmp_path, monkeypatch, caplog):
    readiness = _write_json(tmp_path, "oracle_readiness.json", {"version": 1, "cards": {
        "BT12-002": {"status": "ready"}, "BT12-022": {"status": "ready"},
        "BT12-031": {"status": "not_ready"},
    }})
    monkeypatch.setattr(gauntlet_module, "_ORACLE_READINESS_PATH", readiness)
    path = str(_library(tmp_path, [_deck(), _deck("BT12-031")]))

    def gauntlet(adm=None):
        return MetaGauntlet(
            implemented_card_ids=REGISTERED, mechanics_readiness=READY,
            admissibility=adm or _admissibility(), allowed_archetypes={"Mixed"},
        )

    g = gauntlet()
    g.load(path)
    assert g.deck_count == 2

    with caplog.at_level("DEBUG", logger="digimon_gym.agents.gauntlet"):
        g = gauntlet()
        g.load(path, legacy_clause_gate=True)
    assert [d.source_deck_id for d in g._deck_pool] == ["mixed_000"]
    assert any("not-oracle-ready" in r.getMessage() and "BT12-031" in r.getMessage() for r in caplog.records)

    # The flag only ADDS a requirement: an inadmissible card stays out with it on.
    g = gauntlet(_admissibility(failed={"BT12-031"}))
    g.load(path, legacy_clause_gate=True)
    assert g.deck_count == 1

    # The generalist loader and the snapshot reload honour the flag too.
    assert gauntlet_module.load_generalist_deck_pool(
        path, implemented_card_ids=REGISTERED, mechanics_readiness=READY,
        admissibility=_admissibility(), legacy_clause_gate=True,
    ).deck_count == 1
    snapshot = tmp_path / "pool.json"
    g = gauntlet()
    g.load(path)
    g.as_generalist_pool().write_snapshot(snapshot)
    with pytest.raises(gauntlet_module.NotAdmissibleDeckError, match="BT12-031"):
        GeneralistDeckPool.from_snapshot(
            snapshot, implemented_card_ids=REGISTERED, mechanics_readiness=READY,
            admissibility=_admissibility(), legacy_clause_gate=True,
        )

    # With nothing oracle-ready the legacy pool is empty and fails fast.
    monkeypatch.setattr(
        gauntlet_module, "_ORACLE_READINESS_PATH",
        _write_json(tmp_path, "none_ready.json", {"version": 1, "cards": {}}),
    )
    with pytest.raises(gauntlet_module.EmptyTrainingPoolError, match="not_oracle_ready"):
        gauntlet().load(path, legacy_clause_gate=True)


def test_blocker_message_names_gap_ids_and_regenerate_command(tmp_path):
    # Mechanics not ready: the blocking rules and gap ids, and how to regenerate.
    with pytest.raises(gauntlet_module.MechanicsNotReadyError) as exc:
        MetaGauntlet(
            implemented_card_ids=REGISTERED, mechanics_readiness=NOT_READY,
            admissibility=_admissibility(),
        ).load(str(_library(tmp_path, [_deck()])))
    message = str(exc.value)
    for needle in ("15-7-4", "G-ENGINE-SECURITY-CHECK", "16-37", "G-ENGINE-PIERCE", MECHANICS_CMD):
        assert needle in message, message

    # Empty pool: the cards blocking the most decklists, with their reasons,
    # plus both regenerate commands.
    adm = _admissibility(untested={"BT12-022"}, diverged={"BT12-031"})
    with pytest.raises(gauntlet_module.EmptyTrainingPoolError) as exc:
        MetaGauntlet(
            implemented_card_ids=REGISTERED, mechanics_readiness=READY, admissibility=adm,
        ).load(str(_library(tmp_path, [_deck(), _deck("BT12-031")])))
    message = str(exc.value)
    assert "BT12-022 ExVeemon (2 decklists, untested_lowering)" in message, message
    assert "BT12-031" in message and "diverged_no_quirk" in message, message
    assert ADMISSIBILITY_CMD in message and MECHANICS_CMD in message, message


# ─── Artifact loading (run time reads only data/) ───────────────────

def test_missing_artifacts_fail_at_load_naming_the_regenerate_command(tmp_path, monkeypatch, artifacts):
    path = str(_library(tmp_path, [_deck()]))
    artifacts()
    monkeypatch.setattr(gauntlet_module, "_MECHANICS_READINESS_PATH", tmp_path / "absent_m.json")
    with pytest.raises(gauntlet_module.MechanicsReadinessMissingError, match=MECHANICS_CMD):
        MetaGauntlet(implemented_card_ids=REGISTERED).load(path)

    artifacts()
    monkeypatch.setattr(gauntlet_module, "_CARD_ADMISSIBILITY_PATH", tmp_path / "absent_a.json")
    with pytest.raises(gauntlet_module.CardAdmissibilityMissingError, match=ADMISSIBILITY_CMD):
        MetaGauntlet(implemented_card_ids=REGISTERED).load(path)


def test_gate_applies_to_any_library_path_from_the_artifacts(tmp_path, artifacts):
    artifacts(admissibility=_admissibility(failed={"BT12-031"}))
    g = MetaGauntlet(implemented_card_ids=REGISTERED)
    g.load(str(_library(tmp_path, [_deck(), _deck("BT12-031")])))
    assert [d.source_deck_id for d in g._deck_pool] == ["mixed_000"]


def test_card_missing_from_the_artifact_is_not_admissible(tmp_path, artifacts):
    # Registered in the engine but absent from a stale artifact: the artifact's
    # own answer (`unregistered`) rejects it rather than admitting it by default.
    artifacts(admissibility=_admissibility(registered={"BT12-002", "BT12-022"}))
    with pytest.raises(gauntlet_module.EmptyTrainingPoolError, match=r"BT12-031.*unregistered"):
        MetaGauntlet(implemented_card_ids=REGISTERED).load(str(_library(tmp_path, [_deck("BT12-031")])))


def test_allowed_archetypes_empty_set_is_quietly_empty(tmp_path):
    g = MetaGauntlet(
        implemented_card_ids=REGISTERED, mechanics_readiness=READY,
        admissibility=_admissibility(untested=REGISTERED), allowed_archetypes=set(),
    )
    g.load(str(_library(tmp_path, [_deck()])))
    assert g.deck_count == 0


def test_starter_curriculum_archetypes_are_gated():
    # ST-1 lives in the real deck library; with nothing admissible it fails
    # fast instead of training on nothing.
    with pytest.raises(gauntlet_module.EmptyTrainingPoolError):
        gauntlet_module.load_generalist_deck_pool(
            allowed_archetypes={"ST-1 Gaia Red"},
            mechanics_readiness=READY,
            admissibility=_admissibility(registered=set()),
        )


def test_run_training_job_library_deck_is_gated(tmp_path, monkeypatch, artifacts):
    # `agent_deck: {source: deck_id}` reads a deck straight from the library;
    # it passes both layers like the pool (explicit file decks stay ungated).
    import data_paths
    from tools import run_training_job

    monkeypatch.setattr(data_paths, "DECK_LIBRARY", _library(tmp_path, [_deck(), _deck("BT12-031")]))
    artifacts(admissibility=_admissibility(failed={"BT12-031"}))
    assert len(run_training_job.load_deck({"source": "deck_id", "deck_id": "mixed_000"})) == 55
    with pytest.raises(gauntlet_module.NotAdmissibleDeckError, match="mixed_001.*BT12-031"):
        run_training_job.load_deck({"source": "deck_id", "deck_id": "mixed_001"})
    artifacts(mechanics=NOT_READY)
    with pytest.raises(gauntlet_module.MechanicsNotReadyError):
        run_training_job.load_deck({"source": "deck_id", "deck_id": "mixed_000"})


def test_real_artifacts_load():
    # The committed artifacts parse, and today's mechanics gate is open.
    readiness = gauntlet_module._load_mechanics_readiness()
    gauntlet_module.require_mechanics_ready(readiness)
    adm = gauntlet_module._load_card_admissibility()
    assert adm.admissible("BT12-022") in (True, False)
