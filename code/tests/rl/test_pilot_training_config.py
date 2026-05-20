"""Pilot training tensor-profile config, env wiring, and metadata."""

from __future__ import annotations

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


class _FakeDigimonEnv(gymnasium.Env):
    created: list[_FakeDigimonEnv] = []

    def __init__(self, *_, tensor_profile="standard_compact_v1", **__):
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
        self._deck1 = []
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


def test_make_env_passes_tensor_profile(monkeypatch):
    from digimon_gym.agents import pilot_training

    _FakeDigimonEnv.created = []
    monkeypatch.setattr(pilot_training, "DigimonEnv", _FakeDigimonEnv)

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
    monkeypatch.setattr(pilot_training, "DigimonEnv", _FakeDigimonEnv)

    env = pilot_training.make_env(opponent="self-play")

    base_env = pilot_training._unwrap_to_digimon_env(env)
    assert base_env.tensor_profile == "standard_lite_v2"
    assert _FakeDigimonEnv.created[0].tensor_profile == "standard_lite_v2"


def test_make_vec_env_passes_config_tensor_profile(monkeypatch):
    from digimon_gym.agents import pilot_training

    _FakeDigimonEnv.created = []
    monkeypatch.setattr(pilot_training, "DigimonEnv", _FakeDigimonEnv)
    cfg = TrainingConfig(
        n_envs=2,
        seed=11,
        opponent="greedy",
        tensor_profile="standard_lite_v2",
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
        feature_schema_version="standard_lite_v2.1",
        tensor_size=8320,
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
            self.games_played = 0

        def close(self):
            pass

        def get_archetype_results(self):
            return []

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


def test_training_run_metadata_round_trips_tensor_profile_fields(tmp_path):
    from digimon_gym.agents.training_metrics import TrainingRunMetadata

    path = tmp_path / "run.meta.json"
    metadata = TrainingRunMetadata(
        run_id="run-1",
        started_at="2026-05-02T12:00:00",
        observation_profile="standard_lite_v2",
        tensor_version=2,
        feature_schema_version="standard_lite_v2.1",
        tensor_size=8320,
        tensor_layout_hash="sha256:test",
        action_space_size=2192,
        card_registry_capacity=4096,
        embedding_dim=16,
    )

    metadata.save(path)
    loaded = TrainingRunMetadata.load(path)

    assert loaded.observation_profile == "standard_lite_v2"
    assert loaded.tensor_version == 2
    assert loaded.feature_schema_version == "standard_lite_v2.1"
    assert loaded.tensor_size == 8320
    assert loaded.tensor_layout_hash == "sha256:test"
    assert loaded.action_space_size == 2192
    assert loaded.card_registry_capacity == 4096
    assert loaded.embedding_dim == 16


def test_train_model_kwargs_include_observation_layout(monkeypatch, tmp_path):
    from digimon_gym.agents import pilot_training

    layout = SimpleNamespace(
        id="standard_lite_v2",
        tensor_version=2,
        feature_schema_version="standard_lite_v2.1",
        tensor_size=8320,
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
