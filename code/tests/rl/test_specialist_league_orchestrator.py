"""Offline e2e for the league orchestrator (add-deck-specialist-league, 7.2 + 5.3).

Fakes the training subprocess (no pilot_training, no engine, no compute) so the
round loop -> barrier -> registry advance, the round-pool emission (mirror), and
the promotion gate are all exercised end to end.
"""
from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path

import pytest

from digimon_gym.agents.anchored_eval import MatchRecord
from digimon_gym.agents.specialist_registry import Specialist, SpecialistRegistry

# Load the orchestrator (code/tools is not a package).
_SPEC = importlib.util.spec_from_file_location(
    "tsl_orch", Path(__file__).resolve().parents[2] / "tools" / "train_specialist_league.py")
L = importlib.util.module_from_spec(_SPEC)
sys.modules["tsl_orch"] = L
_SPEC.loader.exec_module(L)

DECKS = ["ST-1 Gaia Red", "ST-4 Giga Green"]


def _spec(tmp_path, **kw):
    sd = tmp_path / "spec"
    return L.LeagueSpec(
        generalist=str(tmp_path / "gen.zip"), decks=list(DECKS),
        save_dir=str(sd), registry_path=str(sd / "registry.json"),
        pool_dir=str(sd / "_pools"), rounds=1, **kw)


def _fake_train_factory():
    """Return a fake _run that writes a final.zip at <--save-dir>/<run_name>/final.zip."""
    def fake_run(argv, cwd):
        sd = argv[argv.index("--save-dir") + 1]
        rn = [a.split("=", 1)[1] for a in argv if a.startswith("run_name=")][0]
        out = Path(sd) / rn / "final.zip"
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text("fake-model")
    return fake_run


def test_round_advances_registry_and_writes_mirror_pool(tmp_path, monkeypatch):
    spec = _spec(tmp_path)
    monkeypatch.setattr(L, "_run", _fake_train_factory())
    reg = L.seed_registry(spec)            # round 0: generalist for both decks
    L.run_round(spec, reg, 1, tmp_path)    # promote_min_wr=0 -> unconditional

    # both decks advanced to round 1
    assert all(reg.get(d).round == 1 for d in DECKS)
    # the round pool for ST-1 included the mirror (ST-1 faces ST-1)
    pool = json.loads(Path(spec.pool_manifest("ST-1 Gaia Red", 1)).read_text())
    assert "ST-1 Gaia Red" in {e["deck"] for e in pool["entries"]}
    # warm-start for round 1 was the round-0 generalist
    assert reg.history[0].source == "generalist-seed"


def test_promotion_gate_keeps_regressing_deck(tmp_path, monkeypatch):
    spec = _spec(tmp_path, promote_min_wr=0.6, eval_n=10)
    # round-0 seeds (the "prev" for the gate)
    reg = SpecialistRegistry(spec.registry_path)
    for d in DECKS:
        reg.set_current(Specialist(
            deck=d, weights_path=str(tmp_path / "gen.zip"), algorithm="mlp",
            observation_profile="p", tensor_layout_hash=spec.layout_hash, round=0,
            source="generalist-seed"))
    reg.save()
    # fake round-1 final.zips on disk
    for d in DECKS:
        f = spec.final_zip(d, 1); f.parent.mkdir(parents=True, exist_ok=True); f.write_text("m")

    # fake the eval machinery: ST-1 improves (promote), ST-4 regresses (keep)
    import digimon_gym.agents.league_eval as LE
    monkeypatch.setattr(L, "_candidate_loader", lambda: (lambda wp: ("model", wp)))
    monkeypatch.setattr(L, "_policy_loader", lambda: (lambda wp, algo: ("pol", wp)))
    monkeypatch.setattr(L, "_resolve_deck_cards",
                        lambda decks: {"ST-1 Gaia Red": ["A"], "ST-4 Giga Green": ["B"]})

    def fake_h2h(cand, prev, deck_cards, n, base_seed, tensor_profile):
        return MatchRecord(wins=18, losses=6) if deck_cards == ["A"] else MatchRecord(wins=9, losses=15)
    monkeypatch.setattr(LE, "anchored_head_to_head", fake_h2h)

    L._barrier(spec, reg, 1)

    assert reg.get("ST-1 Gaia Red").round == 1   # 0.75 >= 0.6 -> promoted
    assert reg.get("ST-4 Giga Green").round == 0  # 0.375 < 0.6 -> kept at round 0
