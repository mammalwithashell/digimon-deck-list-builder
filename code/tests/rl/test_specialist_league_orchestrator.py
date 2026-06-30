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


def test_warmstart_accumulate_round1_is_generalist(tmp_path):
    """Round 1 has no prior round, so accumulate (default) seeds from the generalist."""
    spec = _spec(tmp_path)
    reg = L.seed_registry(spec)
    for d in DECKS:
        assert L._resolve_warmstart(spec, reg, d, 1) == spec.generalist


def test_warmstart_accumulate_round_k_uses_own_prior_checkpoint(tmp_path):
    """A kept (non-promoting) deck still warm-starts round k from its OWN r{k-1}
    checkpoint on disk — NOT from the registry champion (which stays the generalist)."""
    spec = _spec(tmp_path)
    reg = L.seed_registry(spec)
    f = spec.final_zip("ST-1 Gaia Red", 1)
    f.parent.mkdir(parents=True, exist_ok=True)
    f.write_text("r1")
    # registry champion for ST-1 is still the generalist (the deck was "kept")
    assert str(reg.get("ST-1 Gaia Red").weights_path) == spec.generalist
    # accumulate continues the deck's own chain from r1, decoupled from the gate
    assert L._resolve_warmstart(spec, reg, "ST-1 Gaia Red", 2) == str(f)


def test_warmstart_champion_uses_registry_champion(tmp_path):
    """Legacy --warmstart champion reproduces the gate-coupled behavior: it warm-starts
    from the registry champion and ignores the deck's on-disk r{k-1} checkpoint."""
    spec = _spec(tmp_path, warmstart="champion")
    reg = L.seed_registry(spec)
    f = spec.final_zip("ST-1 Gaia Red", 1)
    f.parent.mkdir(parents=True, exist_ok=True)
    f.write_text("r1")
    champ = str(reg.get("ST-1 Gaia Red").weights_path)
    assert L._resolve_warmstart(spec, reg, "ST-1 Gaia Red", 2) == champ
    assert champ != str(f)  # ignores the accumulated checkpoint


def test_warmstart_accumulate_missing_prior_falls_back_to_registry(tmp_path, capsys):
    """If round k>1 can't find the deck's r{k-1} checkpoint (e.g. --from-round resume),
    accumulate falls back to the registry champion with a warning — never aborts."""
    spec = _spec(tmp_path)
    reg = L.seed_registry(spec)
    out = L._resolve_warmstart(spec, reg, "ST-1 Gaia Red", 2)  # no r1 on disk
    assert out == str(reg.get("ST-1 Gaia Red").weights_path)
    assert "falling back to registry champion" in capsys.readouterr().out.lower()


def test_accumulate_chain_link_survives_gate_and_pool_excludes_ungated(tmp_path, monkeypatch):
    """3.1 + 4.1: a kept deck's r1 final.zip survives on disk (the warm-start chain
    link is preserved — no league-level pruning), AND that ungated r1 snapshot never
    enters the opponent pool (the pool stays the gated registry champions)."""
    spec = _spec(tmp_path, promote_min_wr=0.6, eval_n=10, warmstart="accumulate")
    reg = SpecialistRegistry(spec.registry_path)
    for d in DECKS:
        reg.set_current(Specialist(
            deck=d, weights_path=str(tmp_path / "gen.zip"), algorithm="mlp",
            observation_profile="p", tensor_layout_hash=spec.layout_hash, round=0,
            source="generalist-seed"))
    reg.save()
    for d in DECKS:
        f = spec.final_zip(d, 1); f.parent.mkdir(parents=True, exist_ok=True); f.write_text("m")

    # ST-1 regresses (kept at r0), ST-4 improves (promoted to r1)
    import digimon_gym.agents.league_eval as LE
    monkeypatch.setattr(L, "_candidate_loader", lambda: (lambda wp: ("model", wp)))
    monkeypatch.setattr(L, "_policy_loader", lambda: (lambda wp, algo: ("pol", wp)))
    monkeypatch.setattr(L, "_resolve_deck_cards",
                        lambda decks: {"ST-1 Gaia Red": ["A"], "ST-4 Giga Green": ["B"]})

    def fake_h2h(cand, prev, deck_cards, n, base_seed, tensor_profile):
        # ST-1 (["A"]) regresses -> kept; ST-4 (["B"]) improves -> promoted
        return MatchRecord(wins=9, losses=15) if deck_cards == ["A"] else MatchRecord(wins=18, losses=6)
    monkeypatch.setattr(LE, "anchored_head_to_head", fake_h2h)

    L._barrier(spec, reg, 1)
    assert reg.get("ST-1 Gaia Red").round == 0   # kept (0.375 < 0.6)
    assert reg.get("ST-4 Giga Green").round == 1  # promoted (0.75 >= 0.6)

    # 3.1: the kept deck's r1 final.zip still exists and is its r2 warm-start
    assert spec.final_zip("ST-1 Gaia Red", 1).is_file()
    assert L._resolve_warmstart(spec, reg, "ST-1 Gaia Red", 2) == str(spec.final_zip("ST-1 Gaia Red", 1))

    # 4.1: the ungated st-1@r1 snapshot is NOT in the round-2 opponent pool
    pool = json.loads(reg.write_round_pool(
        spec.pool_manifest("ST-4 Giga Green", 2), for_deck="ST-4 Giga Green",
        tensor_layout_hash=spec.layout_hash,
        keep_last_per_deck=spec.keep_last_per_deck, include_mirror=spec.include_mirror,
    ).read_text())
    names = {e["name"] for e in pool["entries"]}
    assert "st-1@r1" not in names          # ungated snapshot excluded
    assert "st-1@r0" in names              # gated (kept) champion present


def test_wandb_flags_plumb_into_specialist_argv(tmp_path):
    """--wandb on the league flows into each specialist subprocess: grouped, with a
    per-deck/round run name, and OFF by default (no wandb flags when disabled)."""
    spec = _spec(tmp_path, wandb=True, wandb_project="digimon-rl",
                 wandb_group="specialist-league", wandb_mode="offline")
    argv = L.build_specialist_argv(spec, "ST-1 Gaia Red", 2, "/seed.zip",
                                   tmp_path / "pool.json")
    assert "--wandb" in argv
    assert argv[argv.index("--wandb-project") + 1] == "digimon-rl"
    assert argv[argv.index("--wandb-group") + 1] == "specialist-league"
    assert argv[argv.index("--wandb-run-name") + 1] == "st-1_r2"
    assert argv[argv.index("--wandb-mode") + 1] == "offline"

    # default: no W&B flags leak into the argv
    off = L.build_specialist_argv(_spec(tmp_path), "ST-1 Gaia Red", 1, "/seed.zip",
                                  tmp_path / "pool.json")
    assert "--wandb" not in off


def test_run_parallel_caps_concurrency(monkeypatch):
    state = {"running": 0, "peak": 0}

    class FakePopen:
        def __init__(self, argv, cwd=None, env=None):
            self.args = argv
            state["running"] += 1
            state["peak"] = max(state["peak"], state["running"])

        def wait(self):
            state["running"] -= 1
            return 0

    monkeypatch.setattr(L.subprocess, "Popen", FakePopen)
    L._run_parallel([["a"], ["b"], ["c"], ["d"], ["e"]], Path("."), max_parallel=2)
    assert state["peak"] <= 2  # never more than 2 specialists at once


# ── LSTM specialist support (cross-arch warm-start) ──────────────────────────
# An LSTM league seeds from the MLP generalist, so round 1 must transfer ONLY
# the shared CardEmbeddingExtractor (--init-extractor-from), while later rounds
# continue the deck's OWN LSTM checkpoint (--init-from, same architecture).

def _write_model(path: Path, algorithm: str) -> str:
    """Touch a fake <name>.zip + sibling <name>.meta.json carrying `algorithm`."""
    path.write_bytes(b"")
    path.with_suffix(".meta.json").write_text(
        json.dumps({"algorithm": algorithm}), encoding="utf-8")
    return str(path)


def test_build_argv_mlp_uses_init_from(tmp_path):
    """Regression: default (MLP) league keeps --init-from and no --lstm."""
    gen = _write_model(tmp_path / "gen.zip", "mlp")
    spec = _spec(tmp_path)  # algorithm defaults to "mlp"
    argv = L.build_specialist_argv(spec, DECKS[0], 1, gen, tmp_path / "pool.json")
    assert "--init-from" in argv
    assert "--init-extractor-from" not in argv
    assert "--lstm" not in argv


def test_build_argv_lstm_round1_warmstarts_extractor_from_mlp(tmp_path):
    """LSTM round 1 from the MLP generalist → --init-extractor-from (not --init-from)."""
    gen = _write_model(tmp_path / "gen.zip", "mlp")
    spec = _spec(tmp_path, algorithm="lstm")
    argv = L.build_specialist_argv(spec, DECKS[0], 1, gen, tmp_path / "pool.json")
    assert "--lstm" in argv
    assert "--init-extractor-from" in argv
    assert argv[argv.index("--init-extractor-from") + 1] == gen
    assert "--init-from" not in argv  # mutually exclusive in pilot_training


def test_build_argv_lstm_round2_uses_init_from_own_checkpoint(tmp_path):
    """LSTM round >1 from its OWN lstm checkpoint (same arch) → --init-from."""
    ckpt = _write_model(tmp_path / "r1_final.zip", "lstm")
    spec = _spec(tmp_path, algorithm="lstm")
    argv = L.build_specialist_argv(spec, DECKS[0], 2, ckpt, tmp_path / "pool.json")
    assert "--lstm" in argv
    assert "--init-from" in argv
    assert argv[argv.index("--init-from") + 1] == ckpt
    assert "--init-extractor-from" not in argv


# ── Parallel launch hardening: stagger + sequential retry ─────────────────────
# Two specialists each build a SubprocVecEnv(start_method="spawn"); launching
# them simultaneously makes the spawn bursts collide -> one dies with
# BrokenPipeError. _run_parallel staggers concurrent launches and retries a
# failed specialist sequentially (a lone run can't collide).

def test_run_parallel_staggers_concurrent_launches(monkeypatch):
    sleeps, launches = [], []

    class FakePopen:
        def __init__(self, argv, cwd=None, env=None):
            self.args = argv
            launches.append(argv)

        def wait(self):
            return 0

    monkeypatch.setattr(L.subprocess, "Popen", FakePopen)
    monkeypatch.setattr(L.time, "sleep", lambda s: sleeps.append(s))
    L._run_parallel([["a"], ["b"]], Path("."), max_parallel=2,
                    stagger_seconds=45, retries=0)
    assert 45 in sleeps          # waited between the two concurrent launches
    assert launches == [["a"], ["b"]]  # both still launched


def test_run_parallel_no_stagger_when_disabled(monkeypatch):
    sleeps = []

    class FakePopen:
        def __init__(self, argv, cwd=None, env=None):
            self.args = argv

        def wait(self):
            return 0

    monkeypatch.setattr(L.subprocess, "Popen", FakePopen)
    monkeypatch.setattr(L.time, "sleep", lambda s: sleeps.append(s))
    L._run_parallel([["a"], ["b"]], Path("."), max_parallel=2,
                    stagger_seconds=0, retries=0)
    assert sleeps == []          # legacy behaviour: no waiting


def test_run_parallel_retries_failed_sequentially(monkeypatch):
    """A specialist that fails in the concurrent wave is retried via _run (alone)."""
    ran = []

    class FakePopen:
        def __init__(self, argv, cwd=None, env=None):
            self.args = argv

        def wait(self):
            return 1 if self.args == ["b"] else 0   # only "b" fails concurrently

    monkeypatch.setattr(L.subprocess, "Popen", FakePopen)
    monkeypatch.setattr(L.time, "sleep", lambda s: None)
    monkeypatch.setattr(L, "_run", lambda argv, cwd: ran.append(argv))  # retry succeeds
    L._run_parallel([["a"], ["b"]], Path("."), max_parallel=2,
                    stagger_seconds=0, retries=1)
    assert ran == [["b"]]        # only the failed one, retried sequentially; no raise


def test_run_parallel_raises_when_retry_exhausted(monkeypatch):
    class FakePopen:
        def __init__(self, argv, cwd=None, env=None):
            self.args = argv

        def wait(self):
            return 1             # fails in the concurrent wave

    def fake_run(argv, cwd):
        raise L.subprocess.CalledProcessError(1, argv)  # and fails the retry too

    monkeypatch.setattr(L.subprocess, "Popen", FakePopen)
    monkeypatch.setattr(L.time, "sleep", lambda s: None)
    monkeypatch.setattr(L, "_run", fake_run)
    with pytest.raises(RuntimeError):
        L._run_parallel([["a"]], Path("."), max_parallel=1,
                        stagger_seconds=0, retries=1)
