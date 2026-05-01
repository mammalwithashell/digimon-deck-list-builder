from __future__ import annotations

import sys
from types import SimpleNamespace

from gymnasium import spaces
import numpy as np
import pytest


def test_default_tensor_profile_shape():
    from digimon_gym.tensor_profiles import get_tensor_profile
    from digimon_engine import TENSOR_PROFILE_ID, TENSOR_SIZE

    profile = get_tensor_profile()

    assert profile.id == TENSOR_PROFILE_ID
    assert profile.tensor_size == TENSOR_SIZE
    assert profile.card_id_slot_count == 520
    assert profile.scalar_slot_count == 855
    assert len(profile.card_id_positions) == 520
    assert len(profile.scalar_positions) == 855


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

    profile = get_tensor_profile()
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


def test_tensor_profile_falls_back_when_engine_function_missing(monkeypatch):
    from digimon_gym.tensor_profiles import get_tensor_profile

    monkeypatch.setitem(sys.modules, "digimon_engine", SimpleNamespace())

    profile = get_tensor_profile()

    assert profile.id == "standard_v1"
    assert profile.card_id_slot_count == 520
    assert profile.scalar_slot_count == 855


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
