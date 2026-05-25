"""Pilot training tensor-profile config, env wiring, and metadata."""

from __future__ import annotations

import json
from pathlib import Path
from types import SimpleNamespace

import gymnasium
import numpy as np
import pytest
import yaml
from gymnasium import spaces

from digimon_gym.agents.training_config import TrainingConfig


def test_training_config_defaults_and_overrides_tensor_profile(tmp_path):
    cfg = TrainingConfig()
    assert cfg.tensor_profile == "standard_lite_v2"

    path = tmp_path / "training.yaml"
    path.write_text("tensor_profile: standard_lite_v2\n")
    loaded = TrainingConfig.from_yaml(path)
    assert loaded.tensor_profile == "standard_lite_v2"

    overridden = TrainingConfig.from_yaml(
        path,
        overrides={"tensor_profile": "standard_compact_v1"},
    )
    assert overridden.tensor_profile == "standard_compact_v1"


@pytest.mark.parametrize("blank", ["", " ", "\t\n"])
def test_training_config_rejects_blank_tensor_profile(tmp_path, blank):
    with pytest.raises(ValueError, match="tensor_profile"):
        TrainingConfig(tensor_profile=blank)

    path = tmp_path / "training.yaml"
    path.write_text(yaml.safe_dump({"tensor_profile": blank}))
    with pytest.raises(ValueError, match="tensor_profile"):
        TrainingConfig.from_yaml(path)


def test_cli_tensor_profile_override_reaches_training_config(monkeypatch, tmp_path):
    from digimon_gym.agents import pilot_training

    config_path = tmp_path / "training.yaml"
    config_path.write_text("timesteps: 1\neval_freq: 0\neval_episodes: 1\n")
    captured = {}

    def fake_train(*_args, cfg, **_kwargs):
        captured["cfg"] = cfg

    monkeypatch.setattr(pilot_training, "train", fake_train)
    monkeypatch.setattr(
        "sys.argv",
        [
            "pilot_training",
            "--config",
            str(config_path),
            "--tensor-profile",
            "standard_lite_v2",
        ],
    )

    pilot_training.main()

    assert captured["cfg"].tensor_profile == "standard_lite_v2"


def test_cli_deck2_json_reaches_train(monkeypatch, tmp_path):
    from digimon_gym.agents import pilot_training

    config_path = tmp_path / "training.yaml"
    config_path.write_text("timesteps: 1\neval_freq: 0\neval_episodes: 1\n")
    deck2_path = tmp_path / "deck2.json"
    deck2_path.write_text('["ST1-01", "ST1-03"]')
    captured = {}

    def fake_train(*_args, cfg, deck2=None, **_kwargs):
        captured["cfg"] = cfg
        captured["deck2"] = deck2

    monkeypatch.setattr(pilot_training, "train", fake_train)
    monkeypatch.setattr(
        pilot_training,
        "load_implemented_card_ids",
        lambda: {"ST1-01", "ST1-03"},
    )
    monkeypatch.setattr(
        "sys.argv",
        [
            "pilot_training",
            "--config",
            str(config_path),
            "--deck2-json",
            str(deck2_path),
        ],
    )

    pilot_training.main()

    assert captured["deck2"] == ["ST1-01", "ST1-03"]


def test_cli_rejects_unimplemented_explicit_deck(monkeypatch, tmp_path):
    from digimon_gym.agents import pilot_training

    config_path = tmp_path / "training.yaml"
    config_path.write_text("timesteps: 1\neval_freq: 0\neval_episodes: 1\n")
    deck_path = tmp_path / "deck.json"
    deck_path.write_text('["ST1-01", "BT99-999"]')
    monkeypatch.setattr(
        pilot_training,
        "load_implemented_card_ids",
        lambda: {"ST1-01"},
    )
    monkeypatch.setattr(
        "sys.argv",
        [
            "pilot_training",
            "--config",
            str(config_path),
            "--deck-json",
            str(deck_path),
        ],
    )

    with pytest.raises(SystemExit):
        pilot_training.main()


def test_cli_generalist_pool_reaches_train(monkeypatch, tmp_path):
    from digimon_gym.agents import pilot_training

    config_path = tmp_path / "training.yaml"
    config_path.write_text("timesteps: 1\neval_freq: 0\neval_episodes: 1\n")
    pool = SimpleNamespace(archetype_count=2, deck_count=3)
    captured = {}

    def fake_train(*_args, cfg, generalist_deck_pool=None, curriculum_seed=None, **_kwargs):
        captured["cfg"] = cfg
        captured["pool"] = generalist_deck_pool
        captured["curriculum_seed"] = curriculum_seed

    monkeypatch.setattr(pilot_training, "train", fake_train)
    monkeypatch.setattr(
        pilot_training,
        "load_generalist_deck_pool",
        lambda **_kwargs: pool,
    )
    monkeypatch.setattr(pilot_training, "load_implemented_card_ids", lambda: set())
    monkeypatch.setattr(
        "sys.argv",
        [
            "pilot_training",
            "--config",
            str(config_path),
            "--generalist",
            "--curriculum-seed",
            "123",
        ],
    )

    pilot_training.main()

    assert captured["cfg"].generalist is True
    assert captured["pool"] is pool
    assert captured["curriculum_seed"] == 123


def test_cli_rejects_deck2_with_gauntlet(monkeypatch, tmp_path):
    from digimon_gym.agents import pilot_training

    config_path = tmp_path / "training.yaml"
    config_path.write_text("timesteps: 1\neval_freq: 0\neval_episodes: 1\n")
    deck2_path = tmp_path / "deck2.json"
    deck2_path.write_text('["ST1-01", "ST1-03"]')
    monkeypatch.setattr(
        "sys.argv",
        [
            "pilot_training",
            "--config",
            str(config_path),
            "--gauntlet",
            "--deck2-json",
            str(deck2_path),
        ],
    )

    with pytest.raises(SystemExit):
        pilot_training.main()


class _FakeDigimonEnv(gymnasium.Env):
    created: list[_FakeDigimonEnv] = []

    def __init__(self, *_, deck1=None, deck2=None, tensor_profile="standard_compact_v1", **__):
        super().__init__()
        self.tensor_profile = tensor_profile
        self.observation_layout = SimpleNamespace(id=tensor_profile)
        self.observation_space = spaces.Box(
            low=-1.0,
            high=1.0,
            shape=(1,),
            dtype=np.float32,
        )
        self.action_space = spaces.Discrete(3)
        self.current_player_id = 1
        self.is_game_over = False
        self.winner_id = None
        self._deck1 = deck1 or []
        self._deck2 = deck2 or []
        self.reset_seeds: list[int | None] = []
        self.__class__.created.append(self)

    def reset(self, *, seed=None, options=None):
        super().reset(seed=seed)
        self.reset_seeds.append(seed)
        return np.zeros(1, dtype=np.float32), {"action_mask": self.action_mask()}

    def step(self, action):
        return (
            np.zeros(1, dtype=np.float32),
            0.0,
            False,
            False,
            {"action_mask": self.action_mask()},
        )

    def action_mask(self):
        return np.ones(3, dtype=np.int8)


def _patch_digimon_env(monkeypatch, fake_cls):
    """Swap DigimonEnv on every module that binds the name."""
    from digimon_gym.agents import env_utils, pilot_training

    monkeypatch.setattr(pilot_training, "DigimonEnv", fake_cls)
    monkeypatch.setattr(env_utils, "DigimonEnv", fake_cls)


def test_make_env_passes_tensor_profile(monkeypatch):
    from digimon_gym.agents import pilot_training

    _FakeDigimonEnv.created = []
    _patch_digimon_env(monkeypatch, _FakeDigimonEnv)

    env = pilot_training.make_env(
        opponent="self-play",
        tensor_profile="standard_lite_v2",
    )

    base_env = pilot_training._unwrap_to_digimon_env(env)
    assert base_env.tensor_profile == "standard_lite_v2"
    assert _FakeDigimonEnv.created[0].tensor_profile == "standard_lite_v2"


def test_make_env_defaults_to_standard_lite_v2(monkeypatch):
    from digimon_gym.agents import pilot_training

    _FakeDigimonEnv.created = []
    _patch_digimon_env(monkeypatch, _FakeDigimonEnv)

    env = pilot_training.make_env(opponent="self-play")

    base_env = pilot_training._unwrap_to_digimon_env(env)
    assert base_env.tensor_profile == "standard_lite_v2"
    assert _FakeDigimonEnv.created[0].tensor_profile == "standard_lite_v2"


def test_make_env_passes_deck2(monkeypatch):
    from digimon_gym.agents import pilot_training

    _FakeDigimonEnv.created = []
    _patch_digimon_env(monkeypatch, _FakeDigimonEnv)

    env = pilot_training.make_env(
        opponent="self-play",
        deck1=["ST1-01"],
        deck2=["ST1-03"],
    )

    base_env = pilot_training._unwrap_to_digimon_env(env)
    assert base_env._deck1 == ["ST1-01"]
    assert base_env._deck2 == ["ST1-03"]


def test_make_env_wraps_generalist_pool(monkeypatch):
    from digimon_gym.agents import pilot_training
    from digimon_gym.agents.gauntlet import DeckEntry, GeneralistDeckPool

    _FakeDigimonEnv.created = []
    _patch_digimon_env(monkeypatch, _FakeDigimonEnv)
    pool = GeneralistDeckPool(
        archetypes={
            "A": [DeckEntry("deck-a", "A", ["ST1-01"])],
            "B": [DeckEntry("deck-b", "B", ["ST1-03"])],
        }
    )

    env = pilot_training.make_env(
        opponent="self-play",
        generalist_deck_pool=pool,
        curriculum_seed=1,
    )

    _obs, info = env.reset()
    assert info["deck1_archetype"] in {"A", "B"}
    assert info["opponent_archetype"] in {"A", "B"}


def test_make_vec_env_passes_config_tensor_profile(monkeypatch):
    from digimon_gym.agents import pilot_training

    _FakeDigimonEnv.created = []
    _patch_digimon_env(monkeypatch, _FakeDigimonEnv)
    cfg = TrainingConfig(
        n_envs=2,
        seed=11,
        opponent="greedy",
        tensor_profile="standard_lite_v2",
        # MatchEnv requires a real DigimonEnv in the wrapper chain;
        # this test uses a fake env to capture tensor_profile threading,
        # so we explicitly opt out of BO3 wrapping.
        match_format="single",
    )

    env = pilot_training.make_vec_env(cfg, opponent_fn=lambda _env: 0)
    try:
        env.reset()
        assert [created.tensor_profile for created in _FakeDigimonEnv.created] == [
            "standard_lite_v2",
            "standard_lite_v2",
        ]
        assert [created.reset_seeds[0] for created in _FakeDigimonEnv.created] == [
            11,
            12,
        ]
    finally:
        env.close()


def test_held_out_eval_suite_run_passes_tensor_profile(monkeypatch):
    from digimon_gym.agents import eval_suite

    class FakeEvalEnv:
        created: list[FakeEvalEnv] = []

        def __init__(self, *_, tensor_profile="standard_compact_v1", **__):
            self.tensor_profile = tensor_profile
            self.current_player_id = 1
            self.is_game_over = False
            self.winner_id = None
            self._step_count = 0
            self.__class__.created.append(self)

        def reset(self, *, seed=None):
            return np.zeros(1, dtype=np.float32), {}

        def step(self, action):
            self._step_count += 1
            self.is_game_over = True
            self.winner_id = 1
            return np.zeros(1, dtype=np.float32), 0.0, True, False, {}

        def action_mask(self):
            return np.ones(3, dtype=np.int8)

    FakeEvalEnv.created = []
    monkeypatch.setattr(eval_suite, "DigimonEnv", FakeEvalEnv)
    suite = eval_suite.HeldOutEvalSuite(
        version=1,
        opponent_policy="greedy",
        games_per_cell=1,
        matchups=[
            eval_suite.Matchup(
                name="smoke",
                deck1=["BT1-001"],
                deck2=["BT1-002"],
                seeds=[7],
            )
        ],
    )

    result = suite.run(
        agent_fn=lambda _env: 0,
        max_games_per_cell=1,
        tensor_profile="standard_lite_v2",
    )

    assert result.overall_win_rate == 1.0
    assert [env.tensor_profile for env in FakeEvalEnv.created] == [
        "standard_lite_v2"
    ]


def test_held_out_eval_suite_defaults_to_standard_lite_v2():
    from digimon_gym.agents import eval_suite

    suite = eval_suite.HeldOutEvalSuite(
        version=1,
        opponent_policy="greedy",
        games_per_cell=1,
    )

    assert suite.tensor_profile == "standard_lite_v2"


def test_held_out_eval_suite_from_yaml_defaults_to_standard_lite_v2(tmp_path):
    from digimon_gym.agents import eval_suite

    path = tmp_path / "eval.yaml"
    path.write_text("version: 1\nopponent_policy: greedy\ngames_per_cell: 1\n")

    suite = eval_suite.HeldOutEvalSuite.from_yaml(path)

    assert suite.tensor_profile == "standard_lite_v2"


def test_train_passes_config_tensor_profile_to_held_out_eval_suite(
    monkeypatch, tmp_path
):
    from digimon_gym.agents import pilot_training

    layout = SimpleNamespace(
        id="standard_lite_v2",
        tensor_version=2,
        feature_schema_version="standard_lite_v2.2",
        tensor_size=8410,
        layout_hash="sha256:test",
    )
    captured = {}

    class FakeSuite:
        @classmethod
        def from_yaml(cls, path, tensor_profile="standard_compact_v1"):
            captured["path"] = Path(path)
            captured["tensor_profile"] = tensor_profile
            return cls()

    class FakePPO:
        def __init__(self, *_args, **_kwargs):
            self.num_timesteps = 0

        def learn(self, *_, **__):
            return self

        def save(self, path):
            Path(path).write_text("model")

    class FakeWinRateCallback:
        def __init__(self, *_, eval_suite=None, **__):
            captured["eval_suite"] = eval_suite
            self.last_win_rate = 0.0
            self.last_mean_reward = 0.0
            self.last_draw_rate = 0.0
            self.last_mean_eval_terminal_score = 0.0
            self.last_mean_eval_dense_reward = 0.0
            self.last_mean_eval_episode_length = 0.0
            self.games_played = 0

        def close(self):
            pass

        def get_archetype_results(self):
            return []

        def get_eval_suite_results(self):
            return {
                "overall_win_rate": 0.75,
                "suite_path": "from-callback",
                "cells": {},
            }

    eval_suite_path = tmp_path / "eval.yaml"
    eval_suite_path.write_text("version: 1\nopponent_policy: greedy\ngames_per_cell: 1\n")

    monkeypatch.setattr(pilot_training, "get_tensor_profile", lambda _profile: layout)
    monkeypatch.setattr(pilot_training, "make_env", lambda **_kwargs: object())
    monkeypatch.setattr(pilot_training, "MaskablePPO", FakePPO)
    monkeypatch.setattr(pilot_training, "WinRateCallback", FakeWinRateCallback)
    monkeypatch.setattr(
        "digimon_gym.agents.eval_suite.HeldOutEvalSuite",
        FakeSuite,
    )

    cfg = TrainingConfig(
        timesteps=1,
        eval_freq=0,
        eval_episodes=1,
        checkpoint_every=0,
        models_dir=str(tmp_path),
        run_name="eval-suite-profile-test",
        tensor_profile="standard_lite_v2",
        eval_suite=str(eval_suite_path),
    )

    pilot_training.train(cfg=cfg, verbose=0)

    assert captured["path"] == eval_suite_path
    assert captured["tensor_profile"] == "standard_lite_v2"
    assert isinstance(captured["eval_suite"], FakeSuite)
    meta = json.loads((tmp_path / "eval-suite-profile-test" / "final.meta.json").read_text())
    assert meta["eval_suite_results"] == {
        "overall_win_rate": 0.75,
        "suite_path": "from-callback",
        "cells": {},
    }


def test_training_run_metadata_round_trips_tensor_profile_fields(tmp_path):
    from digimon_gym.agents.training_metrics import TrainingRunMetadata

    path = tmp_path / "run.meta.json"
    metadata = TrainingRunMetadata(
        run_id="run-1",
        started_at="2026-05-02T12:00:00",
        observation_profile="standard_lite_v2",
        tensor_version=2,
        feature_schema_version="standard_lite_v2.2",
        tensor_size=8410,
        tensor_layout_hash="sha256:test",
        action_space_size=2192,
        card_registry_capacity=4096,
        embedding_dim=16,
        training_mode="generalist",
        sampling_policy="uniform_archetype_then_deck",
        training_seed=42,
        curriculum_seed=123,
        eval_seed=999,
        deck_pool_snapshot_path="models/run/deck_pool_snapshot.json",
        deck_pool_snapshot_hash="sha256:pool",
        eligible_archetypes=["A", "B"],
        eligible_deck_count=3,
        base_checkpoint="models/base/final.zip",
        fine_tune_deck_config={"deck1_card_count": 55},
    )

    metadata.save(path)
    loaded = TrainingRunMetadata.load(path)

    assert loaded.observation_profile == "standard_lite_v2"
    assert loaded.tensor_version == 2
    assert loaded.feature_schema_version == "standard_lite_v2.2"
    assert loaded.tensor_size == 8410
    assert loaded.tensor_layout_hash == "sha256:test"
    assert loaded.action_space_size == 2192
    assert loaded.card_registry_capacity == 4096
    assert loaded.embedding_dim == 16
    assert loaded.training_mode == "generalist"
    assert loaded.sampling_policy == "uniform_archetype_then_deck"
    assert loaded.training_seed == 42
    assert loaded.curriculum_seed == 123
    assert loaded.eval_seed == 999
    assert loaded.deck_pool_snapshot_hash == "sha256:pool"
    assert loaded.eligible_archetypes == ["A", "B"]
    assert loaded.eligible_deck_count == 3
    assert loaded.base_checkpoint == "models/base/final.zip"
    assert loaded.fine_tune_deck_config == {"deck1_card_count": 55}


def test_checkpoint_contract_rejects_incompatible_tensor_profile(tmp_path):
    from digimon_gym.agents import pilot_training

    checkpoint = tmp_path / "base.zip"
    checkpoint.write_text("model")
    checkpoint.with_suffix(".meta.json").write_text(
        '{"observation_profile":"old","tensor_layout_hash":"sha256:test","action_space_size":2192}'
    )
    layout = SimpleNamespace(id="standard_lite_v2", layout_hash="sha256:test")

    with pytest.raises(ValueError, match="observation_profile"):
        pilot_training._validate_checkpoint_contract(checkpoint, layout)


def test_train_model_kwargs_include_observation_layout(monkeypatch, tmp_path):
    from digimon_gym.agents import pilot_training

    layout = SimpleNamespace(
        id="standard_lite_v2",
        tensor_version=2,
        feature_schema_version="standard_lite_v2.2",
        tensor_size=8410,
        layout_hash="sha256:test",
    )
    captured = {}

    class FakePPO:
        def __init__(self, *_args, policy_kwargs=None, **_kwargs):
            captured["policy_kwargs"] = policy_kwargs
            self.num_timesteps = 0

        def learn(self, *_, **__):
            return self

        def save(self, path):
            Path(path).write_text("model")

    monkeypatch.setattr(pilot_training, "get_tensor_profile", lambda _profile: layout)
    monkeypatch.setattr(pilot_training, "make_env", lambda **_kwargs: object())
    monkeypatch.setattr(pilot_training, "MaskablePPO", FakePPO)

    cfg = TrainingConfig(
        timesteps=1,
        eval_freq=0,
        eval_episodes=1,
        checkpoint_every=0,
        models_dir=str(tmp_path),
        run_name="tensor-profile-test",
        tensor_profile="standard_lite_v2",
    )

    pilot_training.train(cfg=cfg, verbose=0)

    extractor_kwargs = captured["policy_kwargs"]["features_extractor_kwargs"]
    assert extractor_kwargs["observation_layout"] is layout


def test_train_generalist_metadata_includes_curriculum(monkeypatch, tmp_path):
    from digimon_gym.agents import pilot_training

    layout = SimpleNamespace(
        id="standard_lite_v2",
        tensor_version=2,
        feature_schema_version="standard_lite_v2.2",
        tensor_size=8410,
        layout_hash="sha256:test",
    )

    class FakePPO:
        def __init__(self, *_args, **_kwargs):
            self.num_timesteps = 0

        def learn(self, *_, **__):
            return self

        def save(self, path):
            Path(path).write_text("model")

    class FakePool:
        archetype_count = 2
        deck_count = 3
        archetype_names = ["A", "B"]
        snapshot_path = ""
        snapshot_hash = ""

        def write_snapshot(self, path):
            self.snapshot_path = str(path)
            self.snapshot_hash = "sha256:pool"
            Path(path).parent.mkdir(parents=True, exist_ok=True)
            Path(path).write_text("{}")
            return self.snapshot_hash

    monkeypatch.setattr(pilot_training, "get_tensor_profile", lambda _profile: layout)
    monkeypatch.setattr(pilot_training, "make_env", lambda **_kwargs: object())
    monkeypatch.setattr(pilot_training, "MaskablePPO", FakePPO)

    cfg = TrainingConfig(
        timesteps=1,
        eval_freq=0,
        eval_episodes=1,
        checkpoint_every=0,
        models_dir=str(tmp_path),
        run_name="generalist-meta",
        tensor_profile="standard_lite_v2",
        generalist=True,
        curriculum_seed=123,
        eval_seed=999,
    )

    pilot_training.train(cfg=cfg, generalist_deck_pool=FakePool(), verbose=0)

    meta = (tmp_path / "generalist-meta" / "final.meta.json").read_text()
    assert '"training_mode": "generalist"' in meta
    assert '"sampling_policy": "uniform_archetype_then_deck"' in meta
    assert '"curriculum_seed": 123' in meta
    assert '"eval_seed": 999' in meta
    assert '"deck_pool_snapshot_hash": "sha256:pool"' in meta


def test_train_init_from_records_base_checkpoint(monkeypatch, tmp_path):
    from digimon_gym.agents import pilot_training

    layout = SimpleNamespace(
        id="standard_lite_v2",
        tensor_version=2,
        feature_schema_version="standard_lite_v2.2",
        tensor_size=8410,
        layout_hash="sha256:test",
    )
    base = tmp_path / "base.zip"
    base.write_text("model")
    base.with_suffix(".meta.json").write_text(
        '{"observation_profile":"standard_lite_v2","tensor_layout_hash":"sha256:test","action_space_size":2192}'
    )

    class FakePPO:
        num_timesteps = 0

        def __init__(self, *_args, **_kwargs):
            pass

        @classmethod
        def load(cls, *_args, **_kwargs):
            return cls()

        def learn(self, *_, **__):
            return self

        def save(self, path):
            Path(path).write_text("model")

    monkeypatch.setattr(pilot_training, "get_tensor_profile", lambda _profile: layout)
    monkeypatch.setattr(pilot_training, "make_env", lambda **_kwargs: object())
    monkeypatch.setattr(pilot_training, "MaskablePPO", FakePPO)

    cfg = TrainingConfig(
        timesteps=1,
        eval_freq=0,
        eval_episodes=1,
        checkpoint_every=0,
        models_dir=str(tmp_path),
        run_name="fine-tune-meta",
        tensor_profile="standard_lite_v2",
        init_from=str(base),
    )

    pilot_training.train(cfg=cfg, deck1=["ST1-01"], verbose=0)

    meta = (tmp_path / "fine-tune-meta" / "final.meta.json").read_text()
    assert '"training_mode": "fine_tune"' in meta
    assert f'"base_checkpoint": "{str(base).replace(chr(92), chr(92) + chr(92))}"' in meta


def test_training_config_mulligan_log_default_and_validation(tmp_path):
    cfg = TrainingConfig()
    assert cfg.mulligan_log == "on"

    # Override via yaml — quote the value so YAML 1.1 doesn't treat it as bool
    path = tmp_path / "training.yaml"
    path.write_text('mulligan_log: "off"\n')
    loaded = TrainingConfig.from_yaml(path)
    assert loaded.mulligan_log == "off"

    # Invalid value rejected
    with pytest.raises(ValueError, match="mulligan_log"):
        TrainingConfig(mulligan_log="maybe")


def test_mulligan_log_flag_argparse_default_and_off(monkeypatch, tmp_path):
    """The --mulligan-log flag flows into TrainingConfig.mulligan_log."""
    from digimon_gym.agents import pilot_training

    # Default (no flag): expect "on"
    monkeypatch.setattr("sys.argv", ["pilot_training.py"])
    parser = pilot_training._build_argparser()
    args = parser.parse_args([])
    assert args.mulligan_log == "on"

    # Explicit off
    args = parser.parse_args(["--mulligan-log", "off"])
    assert args.mulligan_log == "off"

    # Invalid value rejected by argparse choices
    with pytest.raises(SystemExit):
        parser.parse_args(["--mulligan-log", "maybe"])


def test_training_config_bool_yaml_field_still_loads_as_bool(tmp_path):
    """Guard against a regression where stripping YAML's bool resolver
    silently turned `record_game_tensors: false` into the truthy string
    `"false"`."""
    path = tmp_path / "training.yaml"
    path.write_text("record_game_tensors: false\n")
    loaded = TrainingConfig.from_yaml(path)
    assert loaded.record_game_tensors is False
    assert isinstance(loaded.record_game_tensors, bool)


def test_yaml_allowed_archetypes_loads_as_list(tmp_path):
    path = tmp_path / "training.yaml"
    path.write_text(
        "allowed_archetypes:\n  - Rocks\n  - Yellow Hybrid\n"
    )
    loaded = TrainingConfig.from_yaml(path)
    assert loaded.allowed_archetypes == ["Rocks", "Yellow Hybrid"]


def test_cli_archetypes_flag_matches_yaml(monkeypatch, tmp_path):
    """`--archetypes Rocks,Yellow Hybrid` produces the same cfg as YAML."""
    from digimon_gym.agents import pilot_training

    config_path = tmp_path / "training.yaml"
    config_path.write_text("timesteps: 1\neval_freq: 0\neval_episodes: 1\n")
    captured = {}

    def fake_train(*_args, cfg, **_kwargs):
        captured["cfg"] = cfg

    monkeypatch.setattr(pilot_training, "train", fake_train)
    monkeypatch.setattr(
        "sys.argv",
        [
            "pilot_training",
            "--config",
            str(config_path),
            "--archetypes",
            "Rocks, Yellow Hybrid",  # trailing space exercises strip()
        ],
    )

    pilot_training.main()

    assert captured["cfg"].allowed_archetypes == ["Rocks", "Yellow Hybrid"]


def test_cli_archetypes_overrides_yaml(monkeypatch, tmp_path):
    """CLI --archetypes wins over the YAML field, matching other override flags."""
    from digimon_gym.agents import pilot_training

    config_path = tmp_path / "training.yaml"
    config_path.write_text(
        "timesteps: 1\n"
        "eval_freq: 0\n"
        "eval_episodes: 1\n"
        "allowed_archetypes:\n  - Old\n  - From\n  - Yaml\n"
    )
    captured = {}

    def fake_train(*_args, cfg, **_kwargs):
        captured["cfg"] = cfg

    monkeypatch.setattr(pilot_training, "train", fake_train)
    monkeypatch.setattr(
        "sys.argv",
        [
            "pilot_training",
            "--config",
            str(config_path),
            "--archetypes",
            "Rocks,Yellow Hybrid",
        ],
    )

    pilot_training.main()

    assert captured["cfg"].allowed_archetypes == ["Rocks", "Yellow Hybrid"]
