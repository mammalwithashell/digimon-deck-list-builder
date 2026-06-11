"""Tests for champion-derived opponent pools
(`harden-training-pipeline` D3 / spec `champion-pool-curriculum`)."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from digimon_gym.agents.champion_registry import Champion, ChampionRegistry
from digimon_gym.agents.opponent_pool import OpponentPool

LAYOUT = "sha256:cohort-a"
OTHER_LAYOUT = "sha256:cohort-b"


def _registry(tmp_path: Path) -> Path:
    reg = ChampionRegistry(
        tmp_path / "registry.json",
        [
            Champion("v020", "models/v020.zip", "lstm",
                     "standard_lite_deck_v2", LAYOUT),
            Champion("v022", "models/v022.zip", "lstm",
                     "standard_lite_deck_v2", LAYOUT),
            Champion("old-profile", "models/old.zip", "mlp",
                     "standard_compact_v1", OTHER_LAYOUT),
        ],
    )
    reg.save()
    return reg.path


def test_pool_from_registry_filters_to_cohort_with_uniform_weights(tmp_path):
    pool = OpponentPool.from_champion_registry(
        _registry(tmp_path), LAYOUT, manifest_path=tmp_path / "pool.json"
    )
    assert [e.name for e in pool.entries] == ["v020", "v022"]
    assert all(e.win_rate_vs_pool == 0.5 for e in pool.entries)
    assert all(e.games_played == 0 for e in pool.entries)

    pool.save()
    reloaded = OpponentPool.load(tmp_path / "pool.json")
    assert [e.name for e in reloaded.entries] == ["v020", "v022"]
    assert reloaded.entries[0].algorithm == "lstm"


def test_pool_from_registry_empty_cohort_is_explicit_error(tmp_path):
    registry_path = _registry(tmp_path)
    with pytest.raises(ValueError) as exc:
        OpponentPool.from_champion_registry(
            registry_path, "sha256:no-such-cohort"
        )
    msg = str(exc.value)
    assert "no-such-cohort" in msg
    assert str(registry_path) in msg


def test_emit_pool_cli_writes_manifest(tmp_path, monkeypatch, capsys):
    import sys

    from tools import champion_admin

    registry_path = _registry(tmp_path)
    out_path = tmp_path / "pool.json"
    monkeypatch.setattr(sys, "argv", [
        "champion_admin", "--champions", str(registry_path),
        "emit-pool", "--out", str(out_path), "--layout-hash", LAYOUT,
    ])
    champion_admin.main()

    data = json.loads(out_path.read_text())
    assert [e["name"] for e in data["entries"]] == ["v020", "v022"]
    stdout = capsys.readouterr().out
    assert "--opponent pool" in stdout
