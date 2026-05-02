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


class FakeEnv:
    def __init__(self, deck1=None, deck2=None, tensor_profile=None):
        self.tensor_profile = tensor_profile
        self.current_player_id = 1
        self.winner_id = None
        self.is_game_over = False
        self._steps = 0

    def reset(self, seed=None):
        self._steps = 0
        self.winner_id = None
        self.is_game_over = False
        return [0.0], {"tensor_profile": self.tensor_profile}

    def step(self, action):
        self._steps += 1
        terminated = self._steps >= 3
        if terminated:
            self.is_game_over = True
            self.winner_id = 1 if action == 62 else 2
        return [0.0], 0.0, terminated, False, {}

    def action_mask(self):
        mask = [0] * 2168
        mask[62] = 1
        return mask


def clock_from(values):
    iterator = iter(values)
    return lambda: next(iterator)


def test_run_profile_games_counts_steps_wins_and_elapsed(monkeypatch):
    from digimon_gym.agents import tensor_profile_gauntlet as gauntlet

    monkeypatch.setattr(gauntlet, "DigimonEnv", FakeEnv)
    monkeypatch.setattr(gauntlet, "greedy_policy", lambda env: 62)

    profile = fake_profile("standard_lite_v2", 8320)
    clock = clock_from((10.0, 12.0))
    result = gauntlet.run_profile_games(
        requested_profile="standard_lite_v2",
        profile=profile,
        config=gauntlet.TensorProfileRunConfig(
            profiles=("standard_lite_v2",),
            games_per_profile=2,
            seeds=(11, 12),
            max_steps_per_game=10,
            policy="greedy",
        ),
        clock=clock,
    )

    assert result.profile_id == "standard_lite_v2"
    assert result.games_played == 2
    assert result.steps == 6
    assert result.elapsed_seconds == pytest.approx(2.0)
    assert result.wins == 2
    assert result.losses == 0
    assert result.draws == 0


def test_run_profile_games_marks_step_cap_as_draw(monkeypatch):
    from digimon_gym.agents import tensor_profile_gauntlet as gauntlet

    monkeypatch.setattr(gauntlet, "DigimonEnv", FakeEnv)
    monkeypatch.setattr(gauntlet, "greedy_policy", lambda env: 62)

    profile = fake_profile("standard_lite_v2", 8320)
    clock = clock_from((20.0, 21.0))
    result = gauntlet.run_profile_games(
        requested_profile="standard_lite_v2",
        profile=profile,
        config=gauntlet.TensorProfileRunConfig(
            profiles=("standard_lite_v2",),
            games_per_profile=1,
            seeds=(11,),
            max_steps_per_game=2,
            policy="greedy",
        ),
        clock=clock,
    )

    assert result.games_played == 1
    assert result.steps == 2
    assert result.wins == 0
    assert result.losses == 0
    assert result.draws == 1


def test_run_profile_games_raises_when_games_exceed_available_seeds(monkeypatch):
    from digimon_gym.agents import tensor_profile_gauntlet as gauntlet

    monkeypatch.setattr(gauntlet, "DigimonEnv", FakeEnv)
    monkeypatch.setattr(gauntlet, "greedy_policy", lambda env: 62)

    profile = fake_profile("standard_lite_v2", 8320)
    with pytest.raises(ValueError, match="games_per_profile.*seeds"):
        gauntlet.run_profile_games(
            requested_profile="standard_lite_v2",
            profile=profile,
            config=gauntlet.TensorProfileRunConfig(
                profiles=("standard_lite_v2",),
                games_per_profile=3,
                seeds=(11, 12),
                max_steps_per_game=10,
                policy="greedy",
            ),
            clock=clock_from((30.0, 31.0)),
        )


def profile_with_sections(profile_id: str, tensor_size: int, sections):
    profile = fake_profile(profile_id, tensor_size)
    profile.sections = tuple(sections)
    return profile


def section(name: str, offset: int, size: int, shape):
    return SimpleNamespace(name=name, offset=offset, size=size, shape=tuple(shape))


def test_trigger_order_accuracy_compact_profile_has_no_signal():
    from digimon_gym.agents.tensor_profile_gauntlet import score_trigger_order_accuracy

    profile = fake_profile("standard_compact_v1", 1375)

    correct, total = score_trigger_order_accuracy(profile)

    assert correct == 0
    assert total == 1


def test_trigger_order_accuracy_lite_profile_scores_pending_choice_section():
    from digimon_gym.agents.tensor_profile_gauntlet import score_trigger_order_accuracy

    profile = profile_with_sections(
        "standard_lite_v2",
        8320,
        [section("pending_choice_features", 4992, 3072, (32, 96))],
    )

    correct, total = score_trigger_order_accuracy(profile)

    assert correct == 1
    assert total == 1


def test_trigger_order_accuracy_full_profile_scores_pending_and_action_rows():
    from digimon_gym.agents.tensor_profile_gauntlet import score_trigger_order_accuracy

    profile = profile_with_sections(
        "standard_full_v2",
        43008,
        [
            section("pending_choice_features", 4992, 3072, (32, 96)),
            section("action_id_features", 8064, 34688, (2168, 16)),
        ],
    )

    correct, total = score_trigger_order_accuracy(profile)

    assert correct == 2
    assert total == 2


def test_trigger_order_accuracy_rejects_full_profile_without_action_rows():
    from digimon_gym.agents.tensor_profile_gauntlet import score_trigger_order_accuracy

    profile = profile_with_sections(
        "standard_full_v2",
        43008,
        [section("pending_choice_features", 4992, 3072, (32, 96))],
    )

    correct, total = score_trigger_order_accuracy(profile)

    assert correct == 1
    assert total == 2


def test_run_tensor_profile_gauntlet_includes_unavailable_profiles(monkeypatch):
    from digimon_gym.agents import tensor_profile_gauntlet as gauntlet

    available = fake_profile("standard_lite_v2", 8320)

    def resolve(profile_ids, require_profiles):
        return [
            gauntlet.ResolvedProfile("standard_lite_v2", available, True, ""),
            gauntlet.ResolvedProfile("missing_profile", None, False, "unknown tensor profile"),
        ]

    def run_profile_games(requested_profile, profile, config, clock=None):
        return gauntlet.TensorProfileRunResult(
            requested_profile=requested_profile,
            profile_id=profile.id,
            available=True,
            skip_reason="",
            tensor_size=profile.tensor_size,
            layout_hash=profile.layout_hash,
            feature_schema_version=profile.feature_schema_version,
            memory_footprint=gauntlet.estimate_memory_footprint(profile, 128, 1),
            games_played=1,
            steps=3,
            elapsed_seconds=1.5,
            wins=1,
            losses=0,
            draws=0,
            trigger_order_correct=1,
            trigger_order_total=1,
        )

    monkeypatch.setattr(gauntlet, "resolve_profile_requests", resolve)
    monkeypatch.setattr(gauntlet, "run_profile_games", run_profile_games)

    result = gauntlet.run_tensor_profile_gauntlet(
        gauntlet.TensorProfileRunConfig(
            profiles=("standard_lite_v2", "missing_profile"),
            games_per_profile=1,
            seeds=(1,),
        )
    )

    assert len(result.results) == 2
    assert result.results[0].available is True
    assert result.results[0].steps_per_second == pytest.approx(2.0)
    assert result.results[1].available is False
    assert result.results[1].skip_reason == "unknown tensor profile"


def test_gauntlet_result_writes_json_and_markdown(tmp_path):
    from digimon_gym.agents import tensor_profile_gauntlet as gauntlet

    profile = fake_profile("standard_lite_v2", 8320)
    run_result = gauntlet.TensorProfileRunResult(
        requested_profile="standard_lite_v2",
        profile_id="standard_lite_v2",
        available=True,
        skip_reason="",
        tensor_size=8320,
        layout_hash=profile.layout_hash,
        feature_schema_version="standard_lite_v2.1",
        memory_footprint=gauntlet.estimate_memory_footprint(profile, 128, 1),
        games_played=2,
        steps=6,
        elapsed_seconds=3.0,
        wins=1,
        losses=1,
        draws=0,
        trigger_order_correct=1,
        trigger_order_total=1,
    )
    result = gauntlet.TensorProfileGauntletResult(
        config=gauntlet.TensorProfileRunConfig(profiles=("standard_lite_v2",)),
        results=(run_result,),
    )

    json_path = tmp_path / "result.json"
    md_path = tmp_path / "result.md"
    result.write_json(json_path)
    result.write_markdown(md_path)

    assert json_path.read_text(encoding="utf-8").startswith("{")
    markdown = md_path.read_text(encoding="utf-8")
    assert "| Profile | Tensor Size | Steps/sec | Games/hour | Win Rate vs Greedy | Trigger Accuracy | Tensor KiB |" in markdown
    assert "| standard_lite_v2 | 8320 | 2.00 | 2400.00 | 50.00% | 100.00% | 32.50 |" in markdown
