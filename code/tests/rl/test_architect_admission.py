"""The Architect TRAINING path applies the training gate (final review I5; rule 34).

`architect_training.resolve_base_deck`, `architect_cotraining._resolve_base_deck`,
`architect_optimizer._pick_best_decklist` (opponents) and `architect_pool.CandidatePool`
(the mutation pool) all read `data/deck_library.json` outside `MetaGauntlet.load`;
on the training path (`training_gate=True`) each must skip / exclude what the gate
does not admit. Their defaults stay ungated for the hosted deck tools (re-review N1;
`code/tests/api/test_deck_optimizer_ungated.py` pins the `/evaluate` side). The unmarked tests inject a
selective admissibility over the conftest's permissive default; the
`oracle_gate`-marked ones read the committed artifacts.
"""
import json

import pytest

import digimon_gym.agents.architect_admission as admission
import digimon_gym.agents.architect_cotraining as cotraining
import digimon_gym.agents.architect_training as training
import digimon_gym.agents.gauntlet as gauntlet
from digimon_gym.agents.architect_optimizer import _pick_best_decklist
from digimon_gym.agents.architect_pool import CandidatePool
from digimon_gym.agents.gauntlet import MechanicsNotReadyError, NotAdmissibleDeckError

BAD = "BT12-031"
GOOD_A, GOOD_B = "BT12-022", "BT12-002"


class _Rejects:
    def __init__(self, *bad):
        self.bad = set(bad)

    def admissible(self, cid):
        return cid not in self.bad

    def reason(self, cid):
        return "untested_lowering" if cid in self.bad else None

    def reasons(self, cid):
        r = self.reason(cid)
        return [r] if r else []


@pytest.fixture(autouse=True)
def _fresh_cache():
    admission._reset_cache_for_tests()
    yield
    admission._reset_cache_for_tests()


def _reject(monkeypatch, *bad):
    monkeypatch.setattr(gauntlet, "_load_card_admissibility", lambda path=None: _Rejects(*bad))


def _deck(*extra):
    return [GOOD_A] * (50 - len(extra)) + list(extra) + [GOOD_B] * 5


def _library(tmp_path, decklists):
    path = tmp_path / "deck_library.json"
    path.write_text(json.dumps({"archetypes": {"Arch": {"decklists": decklists}}}), encoding="utf-8")
    return path


def _entry(source, deck):
    return {"source": source, "decklist": json.dumps(deck)}


@pytest.mark.parametrize("module, fn", [(training, "resolve_base_deck"), (cotraining, "_resolve_base_deck")])
def test_base_deck_skips_an_inadmissible_library_deck(tmp_path, monkeypatch, module, fn):
    _reject(monkeypatch, BAD)
    preferred = _deck(BAD)  # digilab ranks first but holds a non-admissible card
    fallback = _deck()
    monkeypatch.setattr(module, "_DECK_LIBRARY_PATH", _library(tmp_path, [_entry("egman", fallback), _entry("digilab", preferred)]))
    assert getattr(module, fn)("Arch") == fallback


@pytest.mark.parametrize("module, fn", [(training, "resolve_base_deck"), (cotraining, "_resolve_base_deck")])
def test_base_deck_fails_fast_when_no_library_deck_is_admissible(tmp_path, monkeypatch, module, fn):
    _reject(monkeypatch, BAD)
    monkeypatch.setattr(module, "_DECK_LIBRARY_PATH", _library(tmp_path, [_entry("digilab", _deck(BAD))]))
    with pytest.raises(NotAdmissibleDeckError, match=BAD):
        getattr(module, fn)("Arch")


def test_explicit_cotraining_base_deck_is_validated(tmp_path, monkeypatch):
    _reject(monkeypatch, BAD)
    with pytest.raises(NotAdmissibleDeckError, match=BAD):
        cotraining.ArchitectGauntlet(
            archetype_configs=[{"name": "Arch", "base_deck": _deck(BAD)}],
            meta_config={"archetypes": {}},
            output_dir=str(tmp_path),
        )


def test_cotrainer_rejects_an_inadmissible_base_deck(monkeypatch):
    _reject(monkeypatch, BAD)
    with pytest.raises(NotAdmissibleDeckError, match=BAD):
        cotraining.CoTrainer(archetype_name="Arch", base_deck=_deck(BAD), meta_config={"archetypes": {}})
    cotraining.CoTrainer(archetype_name="Arch", base_deck=_deck(), meta_config={"archetypes": {}})


def test_optimizer_opponent_pick_skips_inadmissible_decklists(monkeypatch):
    _reject(monkeypatch, BAD)
    implemented = {GOOD_A, GOOD_B, BAD}
    good = _deck()
    bad_first = [_entry("digilab", _deck(BAD)), _entry("egman", good)]
    assert _pick_best_decklist(bad_first, implemented, training_gate=True) == good
    assert _pick_best_decklist([_entry("digilab", _deck(BAD))], implemented, training_gate=True) is None
    # Default (hosted deck tools): ungated, the preferred list wins as before.
    assert _pick_best_decklist(bad_first, implemented) == _deck(BAD)


def test_candidate_pool_excludes_inadmissible_cards(monkeypatch):
    _reject(monkeypatch, BAD)
    pool = CandidatePool(
        base_deck=_deck(), archetype_name="Arch", custom_pool=[GOOD_A, GOOD_B, BAD], training_gate=True
    )
    assert BAD not in pool.candidates
    assert GOOD_A in pool.candidates


def test_candidate_pool_is_unfiltered_by_default(monkeypatch):
    """The shared pool's default (hosted deck tools, store_night) is ungated: it
    never consults the gate, even when the mechanics gate is closed."""
    _reject(monkeypatch, BAD)

    def closed(path=None):
        raise AssertionError("the default pool must not read the training gate")

    monkeypatch.setattr(gauntlet, "_load_mechanics_readiness", closed)
    monkeypatch.setattr(gauntlet, "_load_card_admissibility", closed)
    pool = CandidatePool(base_deck=_deck(), archetype_name="Arch", custom_pool=[GOOD_A, GOOD_B, BAD])
    assert BAD in pool.candidates


def test_meta_optimizer_training_gate_validates_the_base_deck(monkeypatch):
    from digimon_gym.agents.architect_optimizer import MetaOptimizer

    _reject(monkeypatch, BAD)
    with pytest.raises(NotAdmissibleDeckError, match=BAD):
        MetaOptimizer("Arch", _deck(BAD), {"archetypes": {}}, training_gate=True)
    assert MetaOptimizer("Arch", _deck(BAD), {"archetypes": {}}).training_gate is False


def test_artifacts_are_loaded_once_per_process(monkeypatch):
    calls = []

    def loader(path=None):
        calls.append(1)
        return _Rejects(BAD)

    monkeypatch.setattr(gauntlet, "_load_card_admissibility", loader)
    admission.admissible_card_ids([GOOD_A])
    admission.is_admissible_deck(_deck())
    assert len(calls) == 1


@pytest.mark.oracle_gate
def test_committed_artifact_excludes_an_untested_lowering_card_from_the_pool():
    """Negative, on the committed artifacts: BT12-031 is `untested_lowering` in
    data/card_admissibility.json, so the mutation pool never offers it."""
    adm = admission.card_admissibility()
    assert not adm.admissible(BAD) and adm.reason(BAD) == "untested_lowering"
    pool = CandidatePool(base_deck=[GOOD_B] * 5, archetype_name="Arch", custom_pool=[BAD, GOOD_B], training_gate=True)
    assert BAD not in pool.candidates


@pytest.mark.oracle_gate
def test_closed_mechanics_gate_stops_base_deck_resolution(tmp_path, monkeypatch):
    not_ready = {
        "version": 1,
        "counts": {"gap_blocking": 1},
        "blocking": [{"rule": "16-37", "gap_id": "G-X", "mechanisms": ["keywords"]}],
        "ready": False,
    }
    monkeypatch.setattr(gauntlet, "_load_mechanics_readiness", lambda path=None: dict(not_ready))
    monkeypatch.setattr(training, "_DECK_LIBRARY_PATH", _library(tmp_path, [_entry("digilab", [GOOD_B] * 50)]))
    with pytest.raises(MechanicsNotReadyError, match="16-37"):
        training.resolve_base_deck("Arch")
