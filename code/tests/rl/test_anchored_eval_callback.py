"""Tests for the in-training anchored eval callback
(`harden-training-pipeline` D2 / spec `in-training-anchored-eval`).

The callback is driven manually (num_timesteps set directly, `_on_step()`
called) so no SB3 training loop is needed; panel games are faked via
monkeypatched `play_matchup` except where noted.
"""

from __future__ import annotations

import json
from pathlib import Path
from types import SimpleNamespace

import pytest

from digimon_gym.agents import anchored_eval_callback as aec
from digimon_gym.agents.anchored_eval import MatchRecord, _seat_balanced_seed
from digimon_gym.agents.anchored_eval_callback import (
    AnchoredEvalCallback,
    build_anchored_deck_pool,
)
from digimon_gym.agents.champion_registry import Champion, ChampionRegistry

LAYOUT = "sha256:layout-under-test"
OTHER_LAYOUT = "sha256:some-other-layout"


class _RecordingLogger:
    def __init__(self):
        self.records: dict = {}

    def record(self, key, value):
        self.records[key] = value


def _write_registry(path: Path, champions):
    reg = ChampionRegistry(path, champions)
    reg.save()
    return path


def _make_callback(tmp_path, *, freq=100, games=4, champions=(),
                   include_greedy=True, pool=None):
    registry_path = _write_registry(
        tmp_path / "registry.json", list(champions)
    )
    cb = AnchoredEvalCallback(
        freq=freq,
        games=games,
        registry_path=registry_path,
        layout_hash=LAYOUT,
        tensor_profile="standard_lite_v2",
        deck_pool_factory=lambda: pool or SimpleNamespace(),
        sidecar_path=tmp_path / "runs" / "anchored_evals.jsonl",
        include_greedy=include_greedy,
    )
    cb.model = SimpleNamespace(logger=_RecordingLogger(), predict=None)
    return cb


def _champion(name, layout=LAYOUT):
    return Champion(
        name=name,
        weights_path=f"models/{name}.zip",
        algorithm="mlp",
        observation_profile="standard_lite_v2",
        tensor_layout_hash=layout,
    )


def test_seat_balanced_seed_alternates_first_player():
    # Game::new picks the first player by seed % 2 — parity must alternate
    # with the game index so each anchor matchup is seat-balanced.
    parities = [_seat_balanced_seed(777, i) % 2 for i in range(8)]
    assert parities == [0, 1, 0, 1, 0, 1, 0, 1]


def test_panel_cadence_runs_exactly_n_times(tmp_path, monkeypatch):
    cb = _make_callback(tmp_path, freq=100)
    panels = []
    monkeypatch.setattr(cb, "_run_panel", lambda: panels.append(cb.num_timesteps))

    for step in range(0, 501, 50):
        cb.num_timesteps = step
        assert cb._on_step() is True

    # Panels at 100, 200, 300, 400, 500 — exactly steps//freq panels.
    assert panels == [100, 200, 300, 400, 500]


def test_freq_zero_never_panels_and_writes_nothing(tmp_path, monkeypatch):
    cb = _make_callback(tmp_path, freq=0)
    monkeypatch.setattr(
        cb, "_run_panel",
        lambda: (_ for _ in ()).throw(AssertionError("panel must not run")),
    )
    for step in range(0, 1_000_001, 100_000):
        cb.num_timesteps = step
        assert cb._on_step() is True
    assert not cb.sidecar_path.exists()


def test_panel_emits_scalars_and_sidecar_row(tmp_path, monkeypatch):
    champ = _champion("v022")
    cb = _make_callback(tmp_path, freq=100, games=6, champions=[champ])
    monkeypatch.setattr(
        aec, "play_matchup",
        lambda model, fn, pool, n, seed, profile: MatchRecord(wins=4, losses=2),
    )
    # Avoid loading real champion weights.
    cb._anchor_fns = [("greedy", object()), ("v022", object())]

    cb.num_timesteps = 100
    assert cb._on_step() is True

    logs = cb.model.logger.records
    assert logs["pilot/anchored/greedy/win_rate"] == pytest.approx(4 / 6)
    assert logs["pilot/anchored/v022/win_rate"] == pytest.approx(4 / 6)
    assert logs["pilot/anchored/panel_mean"] == pytest.approx(4 / 6)

    rows = [
        json.loads(line)
        for line in cb.sidecar_path.read_text().splitlines()
    ]
    assert len(rows) == 1
    row = rows[0]
    assert row["step"] == 100
    assert row["games_per_anchor"] == 6
    assert row["anchors"]["greedy"]["wins"] == 4
    assert row["anchors"]["v022"]["failed"] is False
    assert "panel_seconds" in row


def test_anchor_crash_marks_failed_and_training_continues(tmp_path, monkeypatch):
    cb = _make_callback(tmp_path, freq=100, games=4)
    cb._anchor_fns = [("greedy", object()), ("crashy", object())]

    def maybe_crash(model, fn, pool, n, seed, profile):
        if fn is cb._anchor_fns[1][1]:
            raise RuntimeError("engine panic mid-panel")
        return MatchRecord(wins=3, losses=1)

    monkeypatch.setattr(aec, "play_matchup", maybe_crash)

    cb.num_timesteps = 100
    assert cb._on_step() is True  # training continues

    row = json.loads(cb.sidecar_path.read_text().splitlines()[0])
    assert row["anchors"]["crashy"]["failed"] is True
    assert row["anchors"]["crashy"]["win_rate"] is None
    assert row["anchors"]["greedy"]["failed"] is False
    # Panel mean computed over the completed anchors only.
    assert row["panel_mean_win_rate"] == pytest.approx(3 / 4)


def test_incompatible_champion_excluded_from_panel(tmp_path, monkeypatch):
    compatible = _champion("good", layout=LAYOUT)
    incompatible = _champion("wrong-layout", layout=OTHER_LAYOUT)
    cb = _make_callback(
        tmp_path, champions=[compatible, incompatible]
    )

    cb._on_training_start()
    assert cb.excluded_champions == ["wrong-layout"]

    loaded = []
    monkeypatch.setattr(
        "digimon_gym.agents.pilot_training.make_agent_opponent_fn",
        lambda path, algorithm: loaded.append(path) or (lambda env: 0),
    )
    anchors = cb._build_anchor_fns()
    names = [name for name, _fn in anchors]
    assert names == ["greedy", "good"]
    assert loaded == ["models/good.zip"]


def test_panel_bookkeeping_bug_never_aborts_training(tmp_path, monkeypatch):
    cb = _make_callback(tmp_path, freq=100)
    monkeypatch.setattr(
        cb, "_run_panel",
        lambda: (_ for _ in ()).throw(RuntimeError("bookkeeping bug")),
    )
    cb.num_timesteps = 100
    assert cb._on_step() is True


def test_deck_pool_fallback_prefers_generalist_pool():
    sentinel = object()
    assert build_anchored_deck_pool(generalist_deck_pool=sentinel) is sentinel


def test_deck_pool_fallback_uses_run_decks():
    pool = build_anchored_deck_pool(deck1=["ST1-01"] * 50, deck2=None)
    import numpy as np

    rng = np.random.default_rng(0)
    entry = pool.sample_uniform_archetype(rng)
    assert entry.archetype_name == "run-decks"
    assert entry.card_ids == ["ST1-01"] * 50
