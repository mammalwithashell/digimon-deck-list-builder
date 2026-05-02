from __future__ import annotations

from types import SimpleNamespace

import pytest


def fake_profile(profile_id: str, tensor_size: int):
    return SimpleNamespace(
        id=profile_id,
        game_mode="standard",
        version=2 if profile_id.endswith("_v2") else 1,
        tensor_version=2 if profile_id.endswith("_v2") else 1,
        feature_schema_version=f"{profile_id}.1",
        layout_hash=f"sha256:{profile_id.replace('_', '0')[:8]:0<64}",
        tensor_size=tensor_size,
        field_slots=15,
        slot_size=96,
        max_sources=11,
        card_id_slot_count=542,
        scalar_slot_count=tensor_size - 542,
        card_id_positions=tuple(range(542)),
        scalar_positions=tuple(range(542, tensor_size)),
        sections=(),
    )


def test_resolve_profiles_canonicalizes_compact_alias(monkeypatch):
    from digimon_gym.agents import tensor_profile_gauntlet as gauntlet

    profiles = {
        "compact_v1": fake_profile("standard_compact_v1", 1375),
        "standard_lite_v2": fake_profile("standard_lite_v2", 8320),
        "standard_full_v2": fake_profile("standard_full_v2", 43008),
    }
    monkeypatch.setattr(gauntlet, "get_tensor_profile", lambda profile_id: profiles[profile_id])

    resolved = gauntlet.resolve_profile_requests(
        ("compact_v1", "standard_lite_v2", "standard_full_v2"),
        require_profiles=True,
    )

    assert [item.requested_profile for item in resolved] == [
        "compact_v1",
        "standard_lite_v2",
        "standard_full_v2",
    ]
    assert [item.profile.id for item in resolved] == [
        "standard_compact_v1",
        "standard_lite_v2",
        "standard_full_v2",
    ]
    assert all(item.available for item in resolved)


def test_resolve_profiles_records_skip_when_profile_missing(monkeypatch):
    from digimon_gym.agents import tensor_profile_gauntlet as gauntlet

    def missing_profile(profile_id):
        raise ValueError(f"unknown tensor profile: {profile_id}")

    monkeypatch.setattr(gauntlet, "get_tensor_profile", missing_profile)

    resolved = gauntlet.resolve_profile_requests(("standard_full_v2",), require_profiles=False)

    assert len(resolved) == 1
    assert resolved[0].requested_profile == "standard_full_v2"
    assert resolved[0].profile is None
    assert resolved[0].available is False
    assert "unknown tensor profile" in resolved[0].skip_reason


def test_resolve_profiles_raises_when_required_profile_missing(monkeypatch):
    from digimon_gym.agents import tensor_profile_gauntlet as gauntlet

    def missing_profile(profile_id):
        raise ValueError(f"unknown tensor profile: {profile_id}")

    monkeypatch.setattr(gauntlet, "get_tensor_profile", missing_profile)

    with pytest.raises(ValueError, match="standard_full_v2"):
        gauntlet.resolve_profile_requests(("standard_full_v2",), require_profiles=True)


def test_memory_estimate_uses_tensor_size_and_rollout_shape():
    from digimon_gym.agents.tensor_profile_gauntlet import estimate_memory_footprint

    profile = fake_profile("standard_full_v2", 43008)

    memory = estimate_memory_footprint(profile, n_steps=128, n_envs=4)

    assert memory["tensor_bytes"] == 43008 * 4
    assert memory["tensor_kib"] == pytest.approx((43008 * 4) / 1024)
    assert memory["rollout_observation_bytes"] == 43008 * 4 * 128 * 4
    assert memory["rollout_observation_mib"] == pytest.approx((43008 * 4 * 128 * 4) / 1024 / 1024)
    assert memory["card_embedding_input_slots"] == 542
    assert memory["scalar_input_slots"] == 42466
