from __future__ import annotations

import builtins
import importlib.machinery
import sys
from types import SimpleNamespace

from gymnasium import spaces
import numpy as np
import pytest


DECK = ["ST1-01"] * 5 + ["ST1-03"] * 45


def test_default_observation_profile_shape():
    from digimon_gym.tensor_profiles import get_tensor_profile
    from digimon_engine import DEFAULT_OBSERVATION_PROFILE

    profile = get_tensor_profile()

    assert profile.id == DEFAULT_OBSERVATION_PROFILE
    assert profile.id == "standard_lite_v2"
    assert profile.game_mode == "standard"
    assert profile.tensor_size == 8320
    assert profile.card_id_slot_count == 542
    assert profile.scalar_slot_count == 7778
    assert len(profile.card_id_positions) == 542
    assert len(profile.scalar_positions) == 7778


def test_compact_tensor_profile_remains_compatibility_profile():
    from digimon_gym.tensor_profiles import get_tensor_profile
    from digimon_engine import TENSOR_PROFILE_ID, TENSOR_SIZE

    profile = get_tensor_profile("standard_compact_v1")

    assert profile.id == TENSOR_PROFILE_ID
    assert profile.id == "standard_compact_v1"
    assert profile.game_mode == "standard"
    assert profile.tensor_size == TENSOR_SIZE
    assert profile.tensor_size == 1375
    assert profile.card_id_slot_count == 520
    assert profile.scalar_slot_count == 855
    assert len(profile.card_id_positions) == 520
    assert len(profile.scalar_positions) == 855


def test_get_standard_lite_v2_tensor_profile_from_rust():
    from digimon_gym.tensor_profiles import get_tensor_profile

    profile = get_tensor_profile("standard_lite_v2")

    assert profile.id == "standard_lite_v2"
    assert profile.tensor_size == 8320
    assert profile.tensor_version == 2
    assert profile.feature_schema_version == "standard_lite_v2.1"
    assert profile.card_id_slot_count == 542
    assert profile.scalar_slot_count == 7778
    assert profile.layout_hash.startswith("sha256:")
    assert profile.sections[0].name == "global_features"
    assert profile.sections[0].offset == 0
    assert profile.sections[0].shape == (64,)


def test_tensor_profile_positions_cover_tensor():
    from digimon_gym.tensor_profiles import get_tensor_profile

    profile = get_tensor_profile()
    positions = set(profile.card_id_positions) | set(profile.scalar_positions)

    assert len(positions) == profile.tensor_size
    assert min(positions) == 0
    assert max(positions) == profile.tensor_size - 1
    assert set(profile.card_id_positions).isdisjoint(profile.scalar_positions)


def test_feature_extractor_uses_profile_positions():
    import torch
    from digimon_engine import TENSOR_SIZE
    from digimon_gym.agents.features_extractor import CardEmbeddingExtractor
    from digimon_gym.tensor_profiles import get_tensor_profile

    profile = get_tensor_profile("standard_compact_v1")
    space = spaces.Box(
        shape=(TENSOR_SIZE,),
        low=-10.0,
        high=20001.0,
        dtype=np.float32,
    )

    extractor = CardEmbeddingExtractor(space)

    assert extractor.card_id_indices.numel() == profile.card_id_slot_count
    assert extractor.scalar_indices.numel() == profile.scalar_slot_count

    obs = torch.zeros((2, TENSOR_SIZE), dtype=torch.float32)
    out = extractor(obs)
    assert tuple(out.shape) == (2, 512)


def test_digimon_env_defaults_to_standard_lite_v2_under_rust_backend(monkeypatch):
    pytest.importorskip("digimon_engine")
    monkeypatch.setenv("DIGIMON_BACKEND", "rust")
    monkeypatch.delenv("DIGIMON_TENSOR_PROFILE", raising=False)

    from digimon_gym.digimon_gym import DigimonEnv

    env = DigimonEnv(deck1=DECK, deck2=DECK)
    obs, info = env.reset(seed=7)

    assert env.tensor_profile == "standard_lite_v2"
    assert env.observation_space.shape == (8320,)
    assert obs.shape == (8320,)
    assert info["tensor_profile"] == "standard_lite_v2"


def test_digimon_env_preserves_explicit_compact_profile_under_rust_backend(monkeypatch):
    pytest.importorskip("digimon_engine")
    monkeypatch.setenv("DIGIMON_BACKEND", "rust")
    monkeypatch.delenv("DIGIMON_TENSOR_PROFILE", raising=False)

    from digimon_gym.digimon_gym import DigimonEnv

    env = DigimonEnv(
        deck1=DECK,
        deck2=DECK,
        tensor_profile="standard_compact_v1",
    )
    obs, info = env.reset(seed=7)

    assert env.tensor_profile == "standard_compact_v1"
    assert env.observation_space.shape == (1375,)
    assert obs.shape == (1375,)
    assert info["tensor_profile"] == "standard_compact_v1"


def test_feature_extractor_accepts_observation_layout():
    import torch
    from digimon_gym.agents.features_extractor import CardEmbeddingExtractor
    from digimon_gym.tensor_profiles import get_tensor_profile

    profile = get_tensor_profile("standard_lite_v2")
    space = spaces.Box(
        low=-10.0,
        high=20001.0,
        shape=(profile.tensor_size,),
        dtype=np.float32,
    )
    extractor = CardEmbeddingExtractor(space, observation_layout=profile)
    out = extractor(torch.zeros((2, profile.tensor_size), dtype=torch.float32))

    assert extractor.card_id_indices.numel() == 542
    assert extractor.scalar_indices.numel() == 7778
    assert tuple(out.shape) == (2, 512)


def test_feature_extractor_infers_unique_profile_from_observation_space_shape():
    from digimon_gym.agents.features_extractor import CardEmbeddingExtractor
    from digimon_gym.tensor_profiles import get_tensor_profile

    profile = get_tensor_profile("standard_lite_v2")
    space = spaces.Box(
        low=-10.0,
        high=20001.0,
        shape=(profile.tensor_size,),
        dtype=np.float32,
    )

    extractor = CardEmbeddingExtractor(space)

    assert extractor.card_id_indices.numel() == profile.card_id_slot_count
    assert extractor.scalar_indices.numel() == profile.scalar_slot_count


def test_tensor_profile_falls_back_when_engine_function_missing(monkeypatch):
    from digimon_gym.tensor_profiles import get_tensor_profile

    monkeypatch.setitem(sys.modules, "digimon_engine", SimpleNamespace())

    profile = get_tensor_profile()

    assert profile.id == "standard_compact_v1"
    assert profile.game_mode == "standard"
    assert profile.card_id_slot_count == 520
    assert profile.scalar_slot_count == 855


def test_tensor_profile_falls_back_when_engine_module_absent(monkeypatch):
    from digimon_gym.tensor_profiles import get_tensor_profile, list_tensor_profiles

    monkeypatch.delitem(sys.modules, "digimon_engine", raising=False)
    monkeypatch.setattr("importlib.util.find_spec", lambda name: None)

    profile = get_tensor_profile()

    assert profile.id == "standard_compact_v1"
    assert profile.game_mode == "standard"
    assert list_tensor_profiles() == ["standard_compact_v1"]


@pytest.mark.parametrize(
    "profile_id",
    [None, "standard_compact_v1", "standard_v1", "compact_v1"],
)
def test_tensor_profile_fallback_accepts_legacy_aliases(monkeypatch, profile_id):
    from digimon_gym.tensor_profiles import get_tensor_profile

    monkeypatch.setitem(sys.modules, "digimon_engine", SimpleNamespace())

    profile = get_tensor_profile(profile_id)

    assert profile.id == "standard_compact_v1"
    assert profile.game_mode == "standard"
    assert profile.card_id_slot_count == 520
    assert profile.scalar_slot_count == 855


def test_tensor_profile_fallback_rejects_unknown_profile(monkeypatch):
    from digimon_gym.tensor_profiles import get_tensor_profile

    monkeypatch.setitem(sys.modules, "digimon_engine", SimpleNamespace())

    with pytest.raises(ValueError, match="unknown tensor profile: unknown_v1"):
        get_tensor_profile("unknown_v1")


def test_tensor_profile_fallback_rejects_standard_lite_v2_without_layout(monkeypatch):
    from digimon_gym.tensor_profiles import get_tensor_profile

    monkeypatch.setitem(sys.modules, "digimon_engine", SimpleNamespace())

    with pytest.raises(
        ValueError,
        match="standard_lite_v2 requires digimon_engine observation layout support",
    ):
        get_tensor_profile("standard_lite_v2")


def test_tensor_profile_for_tensor_size_requires_unique_registry_match(monkeypatch):
    from digimon_gym.tensor_profiles import get_tensor_profile_for_tensor_size

    def raw_profile(profile_id, tensor_size):
        return {
            "profile_id": profile_id,
            "game_mode": "standard",
            "version": 1,
            "tensor_version": 1,
            "feature_schema_version": f"{profile_id}.1",
            "layout_hash": "sha256:test",
            "tensor_size": tensor_size,
            "field_slots": 1,
            "slot_size": 1,
            "max_sources": 1,
            "card_id_slot_count": 1,
            "scalar_slot_count": tensor_size - 1,
            "card_id_positions": (0,),
            "scalar_positions": tuple(range(1, tensor_size)),
            "sections": (),
        }

    layouts = {
        "profile_a": raw_profile("profile_a", 4),
        "profile_b": raw_profile("profile_b", 4),
        "profile_c": raw_profile("profile_c", 7),
    }
    monkeypatch.setitem(
        sys.modules,
        "digimon_engine",
        SimpleNamespace(
            list_observation_profiles=lambda: list(layouts),
            get_observation_layout=lambda profile_id=None: layouts[profile_id],
        ),
    )

    with pytest.raises(
        ValueError,
        match=(
            "multiple tensor profiles match observation tensor size 4: "
            "profile_a, profile_b"
        ),
    ):
        get_tensor_profile_for_tensor_size(4)


def test_tensor_profile_for_tensor_size_rejects_unknown_shape(monkeypatch):
    from digimon_gym.tensor_profiles import get_tensor_profile_for_tensor_size

    monkeypatch.setitem(
        sys.modules,
        "digimon_engine",
        SimpleNamespace(
            list_observation_profiles=lambda: ["profile_a"],
            get_observation_layout=lambda profile_id=None: {
                "profile_id": "profile_a",
                "game_mode": "standard",
                "version": 1,
                "tensor_version": 1,
                "feature_schema_version": "profile_a.1",
                "layout_hash": "sha256:test",
                "tensor_size": 4,
                "field_slots": 1,
                "slot_size": 1,
                "max_sources": 1,
                "card_id_slot_count": 1,
                "scalar_slot_count": 3,
                "card_id_positions": (0,),
                "scalar_positions": (1, 2, 3),
                "sections": (),
            },
        ),
    )

    with pytest.raises(
        ValueError,
        match="no registered tensor profile matches observation tensor size 9",
    ):
        get_tensor_profile_for_tensor_size(9)


def test_tensor_profile_canonicalizes_engine_alias(monkeypatch):
    from digimon_gym.tensor_profiles import get_tensor_profile

    raw_profile = SimpleNamespace(
        id="standard_v1",
        game_mode="standard",
        version=1,
        tensor_size=1375,
        field_slots=27,
        slot_size=45,
        max_sources=6,
        card_id_slot_count=520,
        scalar_slot_count=855,
        card_id_positions=range(520),
        scalar_positions=range(520, 1375),
    )
    monkeypatch.setitem(
        sys.modules,
        "digimon_engine",
        SimpleNamespace(get_tensor_profile=lambda _profile_id=None: raw_profile),
    )

    profile = get_tensor_profile("standard_v1")

    assert profile.id == "standard_compact_v1"


def test_tensor_profile_does_not_hide_engine_import_error(monkeypatch):
    from digimon_gym.tensor_profiles import get_tensor_profile, list_tensor_profiles

    original_import = builtins.__import__

    def broken_import(name, *args, **kwargs):
        if name == "digimon_engine":
            raise ImportError("binding load failed")
        return original_import(name, *args, **kwargs)

    monkeypatch.delitem(sys.modules, "digimon_engine", raising=False)
    monkeypatch.setattr(
        "importlib.util.find_spec",
        lambda name: importlib.machinery.ModuleSpec(name, loader=None),
    )
    monkeypatch.setattr(builtins, "__import__", broken_import)

    with pytest.raises(ImportError, match="binding load failed"):
        get_tensor_profile()
    with pytest.raises(ImportError, match="binding load failed"):
        list_tensor_profiles()


def test_tensor_profile_malformed_engine_profile_raises(monkeypatch):
    from digimon_gym.tensor_profiles import get_tensor_profile

    engine = SimpleNamespace(
        get_tensor_profile=lambda _profile_id=None: SimpleNamespace(id="standard_v1")
    )
    monkeypatch.setitem(sys.modules, "digimon_engine", engine)

    with pytest.raises(AttributeError):
        get_tensor_profile()


def test_list_tensor_profiles_does_not_hide_binding_errors(monkeypatch):
    from digimon_gym.tensor_profiles import list_tensor_profiles

    def broken_list_tensor_profiles():
        raise RuntimeError("binding exploded")

    monkeypatch.setitem(
        sys.modules,
        "digimon_engine",
        SimpleNamespace(list_tensor_profiles=broken_list_tensor_profiles),
    )

    with pytest.raises(RuntimeError, match="binding exploded"):
        list_tensor_profiles()
