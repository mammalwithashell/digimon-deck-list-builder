"""Tests for the champion registry — frozen model snapshots used as fixed
evaluation anchors (part of add-model-evaluation-harness)."""
import json
import pytest

from digimon_gym.agents.champion_registry import (
    Champion, ChampionRegistry, should_promote,
)


def _champ(name, hash_="sha256:aaa", profile="standard_lite_deck_v2"):
    return Champion(
        name=name, weights_path=f"/models/{name}.zip", algorithm="lstm",
        observation_profile=profile, tensor_layout_hash=hash_,
        source="run-x", created_at="2026-05-31",
    )


def test_register_and_get():
    reg = ChampionRegistry("unused.json")
    reg.register(_champ("v22"))
    assert reg.names() == ["v22"]
    assert reg.get("v22").weights_path == "/models/v22.zip"


def test_register_rejects_duplicate_name():
    reg = ChampionRegistry("unused.json")
    reg.register(_champ("v22"))
    with pytest.raises(ValueError):
        reg.register(_champ("v22"))


def test_get_missing_raises():
    reg = ChampionRegistry("unused.json")
    with pytest.raises(KeyError):
        reg.get("nope")


def test_compatible_filters_by_layout_hash():
    reg = ChampionRegistry("unused.json")
    reg.register(_champ("a", hash_="sha256:match"))
    reg.register(_champ("b", hash_="sha256:other"))
    compat = reg.compatible("sha256:match")
    assert [c.name for c in compat] == ["a"]


def test_load_save_roundtrip(tmp_path):
    path = tmp_path / "champions" / "registry.json"
    reg = ChampionRegistry(path)
    reg.register(_champ("v22", hash_="sha256:h1"))
    reg.register(_champ("v20", hash_="sha256:h1"))
    reg.save()
    assert path.exists()

    reloaded = ChampionRegistry.load(path)
    assert reloaded.names() == ["v22", "v20"]
    assert reloaded.get("v20").tensor_layout_hash == "sha256:h1"
    # provenance fields survive the roundtrip
    assert reloaded.get("v22").source == "run-x"


def test_load_missing_file_is_empty(tmp_path):
    reg = ChampionRegistry.load(tmp_path / "absent.json")
    assert reg.names() == []


def test_should_promote_gate():
    assert should_promote(11, 20) is True    # 55% exactly -> promote
    assert should_promote(10, 20) is False   # 50% -> no
    assert should_promote(60, 100) is True
    assert should_promote(0, 0) is False     # no games -> no


def test_from_meta_parses_profile_and_algorithm():
    meta = {
        "algorithm": "LSTM",
        "observation_profile": "standard_lite_deck_v2",
        "tensor_layout_hash": "sha256:a20462fb",
    }
    c = Champion.from_meta("v22", "/models/v22.zip", meta,
                           source="cloud-v0.22", created_at="2026-05-31")
    assert c.algorithm == "lstm"
    assert c.observation_profile == "standard_lite_deck_v2"
    assert c.tensor_layout_hash == "sha256:a20462fb"
    assert c.source == "cloud-v0.22"
