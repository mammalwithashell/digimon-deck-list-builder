"""Unit and integration tests for Hybrid Search <-> LSTM PPO seams (Seams 1-4)."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import gymnasium as gym
import numpy as np
import pytest
import torch

from model_contract import current_contract, validate_checkpoint


def _write_synthetic_shard_dir(
    root: Path,
    *,
    games: int = 4,
    steps_per_seat: int = 3,
    profile_id: str = "standard_lite_v2",
) -> Path:
    contract = current_contract(profile_id)
    obs_dim = contract["tensor_size"]
    action_dim = contract["action_space_size"]

    shard = root / "shard-000"
    shard.mkdir(parents=True, exist_ok=True)
    rng = np.random.default_rng(1234)

    obs_list = []
    offsets = [0]
    actions: list[int] = []
    visits: list[float] = []
    weights: list[float] = []
    z_list: list[float] = []
    term_list: list[int] = []
    movers: list[int] = []
    game_ids: list[int] = []

    for g in range(games):
        winner = g % 2
        for t in range(steps_per_seat):
            for seat in (0, 1):
                row_obs = np.zeros(obs_dim, dtype=np.float32)
                row_obs[0] = float(g + 1)
                row_obs[1] = float(t + 1)
                row_obs[2] = float(seat)
                obs_list.append(row_obs)
                legal = [0, 1, 62] if t == 0 else [60, 62, 100 + g]
                vis = [10.0, 2.0, 4.0] if seat == winner else [1.0, 8.0, 3.0]
                actions.extend(legal)
                visits.extend(vis)
                offsets.append(len(actions))
                # Alternate cheap (0.0) and full (1.0) search rows to test policy_weight masking.
                weights.append(0.0 if (t == 1 and seat == 1) else 1.0)
                z_list.append(1.0 if seat == winner else -1.0)
                term_list.append(1)
                movers.append(seat)
                game_ids.append(g)

    np.save(shard / "obs.npy", np.asarray(obs_list, dtype=np.float32))
    np.save(shard / "policy_offsets.npy", np.asarray(offsets, dtype=np.int64))
    np.save(shard / "policy_actions.npy", np.asarray(actions, dtype=np.int32))
    np.save(shard / "policy_visits.npy", np.asarray(visits, dtype=np.float32))
    np.save(shard / "policy_weight.npy", np.asarray(weights, dtype=np.float32))
    np.save(shard / "z.npy", np.asarray(z_list, dtype=np.float32))
    np.save(shard / "terminated.npy", np.asarray(term_list, dtype=np.uint8))
    np.save(shard / "mover.npy", np.asarray(movers, dtype=np.uint8))
    np.save(shard / "game_id.npy", np.asarray(game_ids, dtype=np.int32))

    manifest = {
        "schema": "selfplay-shards-v1",
        "observation_profile": profile_id,
        "model_contract": contract,
        "obs_dim": obs_dim,
        "action_dim": action_dim,
        "value_target": "terminal_outcome",
        "information": {"regime": "open_decklist", "knowledge_schema": 2},
        "games": games,
        "truncated_games": 0,
        "shards": [{"dir": "shard-000", "rows": len(obs_list)}],
    }
    (root / "manifest.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    return root


def test_search_budget_flags_macro_substeps_default_and_opt_in():
    from tools.run_selfplay_generation import search_budget_flags

    default_args = argparse.Namespace(
        full_search=False,
        reduced_sims=64,
        full_search_fraction=0.25,
        cheap_sims=64,
        macro_substeps=0,
    )
    assert "--macro-substeps" not in search_budget_flags(default_args)

    macro_args = argparse.Namespace(
        full_search=False,
        reduced_sims=64,
        full_search_fraction=0.25,
        cheap_sims=64,
        macro_substeps=16,
    )
    flags = search_budget_flags(macro_args)
    assert flags[-2:] == ["--macro-substeps", "16"]

    full_macro_args = argparse.Namespace(full_search=True, macro_substeps=16)
    assert search_budget_flags(full_macro_args) == ["--macro-substeps", "16"]


def test_terminal_strategic_v1_reward_profile_is_pure_unit_terminal():
    from digimon_gym.agents.reward.profile_loader import ProfileLoader

    repo = Path(__file__).resolve().parents[3]
    loader = ProfileLoader(
        gameplay_path=repo / "code" / "digimon_gym" / "agents" / "reward" / "gameplay.yaml",
        profiles_path=repo / "code" / "digimon_gym" / "agents" / "reward" / "profiles.yaml",
    )
    prof = loader.profiles.resolve_active(archetype=None, override="terminal_strategic_v1")
    by_kind = {mc.kind: mc.component for mc in prof.components}
    term = by_kind["terminal_outcome"]
    assert term.win_base == pytest.approx(1.0)
    assert term.loss == pytest.approx(-1.0)
    assert term.draw == pytest.approx(0.0)
    assert term.fast_win_bonus_max == pytest.approx(0.0)
    assert by_kind["quick_win_bonus"].peak_value == pytest.approx(0.0)
    assert by_kind["stall_penalty"].scale == pytest.approx(0.0)
    assert by_kind["step_penalty"].weight == pytest.approx(0.0)


def test_azero_lstm_distill_and_onnx_export_and_search_opponent_validation(tmp_path: Path):
    from digimon_gym.agents.azero_lstm_distill import distill_lstm, load_sequence_shards
    from digimon_gym.agents.pilot_training import _validate_checkpoint_contract
    from digimon_gym.tensor_profiles import get_tensor_profile
    from tools.search_opponents import SCHEMA, checkpoint_entry, validate_pool

    shard_dir = _write_synthetic_shard_dir(tmp_path / "shards", games=4, steps_per_seat=3)
    ds = load_sequence_shards([shard_dir])
    assert len(ds.sequences) == 8  # 4 games * 2 seats
    assert ds.total_rows == 24

    out_zip = tmp_path / "lstm_distilled.zip"
    out_onnx = tmp_path / "lstm_distilled.onnx"
    summary = distill_lstm(
        shard_dirs=[shard_dir],
        out=out_zip,
        lstm_hidden_size=32,
        net_arch=[32, 32],
        features_dim=32,
        embedding_dim=4,
        epochs=2,
        seq_batch_size=4,
        lr=1e-3,
        holdout_fraction=0.25,
        device="cpu",
        seed=7,
        export_onnx=out_onnx,
    )
    assert out_zip.exists()
    assert out_onnx.exists()
    assert Path(f"{out_onnx}.meta.json").exists()
    assert summary["heldout_sequences"] == 2  # 1 whole game * 2 seats held out
    assert summary["fit_sequences"] == 6

    layout = get_tensor_profile("standard_lite_v2")
    _validate_checkpoint_contract(out_zip, layout)

    onnx_meta = json.loads(Path(f"{out_onnx}.meta.json").read_text(encoding="utf-8"))
    assert onnx_meta["value_semantics"] == "terminal_outcome"

    # Verify search_opponents.validate_pool accepts mode="policy_lstm" alongside a current MLP.
    current_onnx = tmp_path / "current_mlp.onnx"
    current_onnx.write_bytes(b"dummy-current-mlp-bytes")
    Path(f"{current_onnx}.meta.json").write_text(json.dumps(onnx_meta, indent=2), encoding="utf-8")

    pool_doc = {
        "schema": SCHEMA,
        "selfplay_probability": 0.5,
        "opponents": [
            checkpoint_entry("sparring-lstm", out_onnx, weight=2.0, mode="policy_lstm"),
        ],
    }
    validated = validate_pool(pool_doc, tmp_path, current_onnx)
    assert validated["opponents"][0]["mode"] == "policy_lstm"
    assert validated["opponents"][0]["weight"] == 2.0


def test_maskable_recurrent_ppo_kl_anchor_and_save_load(tmp_path: Path):
    from sb3_contrib.common.wrappers import ActionMasker
    from digimon_gym.agents.maskable_recurrent import (
        MaskableMlpLstmPolicy,
        MaskableRecurrentPPO,
    )

    class _ToyMaskedEnv(gym.Env):
        observation_space = gym.spaces.Box(-1.0, 1.0, (12,), np.float32)
        action_space = gym.spaces.Discrete(6)

        def __init__(self):
            self._step = 0

        def action_mask(self):
            mask = np.zeros(6, dtype=np.int8)
            mask[[1, 3, 5]] = 1
            return mask

        def reset(self, *, seed=None, options=None):
            self._step = 0
            return np.zeros(12, dtype=np.float32), {"action_mask": self.action_mask()}

        def step(self, action):
            assert int(action) in (1, 3, 5)
            self._step += 1
            done = self._step >= 4
            reward = 1.0 if (done and int(action) == 3) else 0.0
            return (
                np.full(12, float(self._step) * 0.1, dtype=np.float32),
                reward,
                done,
                False,
                {"action_mask": self.action_mask()},
            )

    def _make():
        return ActionMasker(_ToyMaskedEnv(), lambda e: e.action_mask())

    anchor_zip = tmp_path / "anchor.zip"
    anchor = MaskableRecurrentPPO(
        MaskableMlpLstmPolicy,
        _make(),
        n_steps=16,
        batch_size=16,
        n_epochs=2,
        policy_kwargs=dict(lstm_hidden_size=16, net_arch=dict(pi=[16], vf=[16])),
        seed=11,
        device="cpu",
    )
    anchor.save(str(anchor_zip))

    learner = MaskableRecurrentPPO.load(
        str(anchor_zip),
        env=_make(),
        device="cpu",
        learning_rate=3e-4,
        n_steps=16,
        batch_size=16,
        n_epochs=2,
        kl_anchor_path=str(anchor_zip),
        kl_coef=0.05,
    )
    assert learner._kl_anchor_policy is not None
    learner.learn(total_timesteps=32)
    assert "train/anchor_kl" in learner.logger.name_to_value
    assert learner.logger.name_to_value["train/anchor_kl"] >= -1e-6

    saved_zip = tmp_path / "learner_saved.zip"
    learner.save(str(saved_zip))
    # Delete anchor file to prove inference load (env=None) is 100% self-contained.
    anchor_zip.unlink()
    reloaded = MaskableRecurrentPPO.load(str(saved_zip), device="cpu")
    act, state = reloaded.predict(
        np.zeros(12, dtype=np.float32),
        state=None,
        episode_start=np.array([True]),
        deterministic=True,
        action_masks=np.array([0, 1, 0, 1, 0, 1], dtype=np.int8),
    )
    assert int(act) in (1, 3, 5)
    assert state is not None


def test_auto_advance_forced_wrapper_skips_single_action_states():
    from digimon_gym.agents.pilot_training import AutoAdvanceForcedWrapper

    class _FakeInner(gym.Env):
        observation_space = gym.spaces.Box(0.0, 1.0, (4,), np.float32)
        action_space = gym.spaces.Discrete(4)

        def __init__(self):
            self.idx = 0
            self.is_game_over = False
            # Masks sequence: 2-choice -> 1-choice(forced=2) -> 1-choice(forced=1) -> 2-choice -> terminal
            self.masks = [
                np.array([1, 1, 0, 0], dtype=np.int8),
                np.array([0, 0, 1, 0], dtype=np.int8),
                np.array([0, 1, 0, 0], dtype=np.int8),
                np.array([1, 0, 0, 1], dtype=np.int8),
            ]
            self.applied: list[int] = []

        def action_mask(self):
            return self.masks[min(self.idx, len(self.masks) - 1)]

        def reset(self, *, seed=None, options=None):
            self.idx = 0
            self.is_game_over = False
            self.applied.clear()
            return np.zeros(4, dtype=np.float32), {"action_mask": self.action_mask()}

        def step(self, action):
            self.applied.append(int(action))
            self.idx += 1
            done = self.idx >= len(self.masks)
            self.is_game_over = done
            return (
                np.full(4, float(self.idx), dtype=np.float32),
                0.25,
                done,
                False,
                {"action_mask": self.action_mask()},
            )

    _FakeInner.__module__ = "digimon_gym.digimon_gym"
    _FakeInner.__name__ = "DigimonEnv"
    inner = _FakeInner()
    wrapped = AutoAdvanceForcedWrapper(inner)
    obs, _ = wrapped.reset()
    assert inner.applied == []
    # Taking action 0 advances through the two forced states (2, 1) and stops at idx=3 ([1,0,0,1]).
    obs, reward, terminated, truncated, _ = wrapped.step(0)
    assert not terminated and not truncated
    assert inner.applied == [0, 2, 1]
    assert reward == pytest.approx(0.75)  # 3 inner steps * 0.25 accumulated
    obs, reward, terminated, truncated, _ = wrapped.step(3)
    assert terminated
    assert inner.applied == [0, 2, 1, 3]


def test_search_budget_flags_macro_substep_min_prior():
    from tools.run_selfplay_generation import search_budget_flags

    base = argparse.Namespace(
        full_search=True,
        macro_substeps=16,
        macro_substep_min_prior=0.0,
    )
    assert search_budget_flags(base) == ["--macro-substeps", "16"]

    gated = argparse.Namespace(
        full_search=True,
        macro_substeps=16,
        macro_substep_min_prior=0.85,
    )
    assert search_budget_flags(gated) == [
        "--macro-substeps",
        "16",
        "--macro-substep-min-prior",
        "0.85",
    ]

    disabled_macro = argparse.Namespace(
        full_search=True,
        macro_substeps=0,
        macro_substep_min_prior=0.85,
    )
    assert search_budget_flags(disabled_macro) == []


def test_lstm_onnx_export_dynamic_batch_parity_and_regularization(tmp_path: Path):
    import onnxruntime as ort
    from digimon_gym.agents.azero_lstm_distill import distill_lstm

    shard_dir = _write_synthetic_shard_dir(tmp_path / "shards", games=4, steps_per_seat=3)
    out_zip = tmp_path / "lstm_reg.zip"
    out_onnx = tmp_path / "lstm_reg.onnx"
    value_onnx = tmp_path / "lstm_reg.value.onnx"

    summary = distill_lstm(
        shard_dirs=[shard_dir],
        out=out_zip,
        lstm_hidden_size=32,
        net_arch=[32, 32],
        features_dim=32,
        embedding_dim=4,
        epochs=2,
        seq_batch_size=4,
        lr=1e-3,
        weight_decay=1e-4,
        value_label_smoothing=0.05,
        holdout_fraction=0.25,
        device="cpu",
        seed=19,
        export_onnx=out_onnx,
    )
    assert summary["weight_decay"] == pytest.approx(1e-4)
    assert summary["value_label_smoothing"] == pytest.approx(0.05)
    assert np.isfinite(summary["epochs"][0]["heldout"]["value_mse"])
    assert out_onnx.exists()
    assert value_onnx.exists()

    # Verify dynamic batch=4 matches 4 independent batch=1 evaluations on both actor and critic ONNX.
    rng = np.random.default_rng(77)
    obs_dim = current_contract("standard_lite_v2")["tensor_size"]
    batch = 4
    obs = rng.standard_normal((batch, obs_dim), dtype=np.float32) * 0.1
    h_in = rng.standard_normal((1, batch, 32), dtype=np.float32) * 0.1
    c_in = rng.standard_normal((1, batch, 32), dtype=np.float32) * 0.1

    actor_sess = ort.InferenceSession(str(out_onnx), providers=["CPUExecutionProvider"])
    b_logits, b_h, b_c = actor_sess.run(
        ["logits", "h_out", "c_out"],
        {"obs": obs, "h_in": h_in, "c_in": c_in},
    )
    assert b_logits.shape[0] == batch
    assert b_h.shape == (1, batch, 32)
    assert b_c.shape == (1, batch, 32)

    critic_sess = ort.InferenceSession(str(value_onnx), providers=["CPUExecutionProvider"])
    b_val, b_hv, b_cv = critic_sess.run(
        ["value", "h_out", "c_out"],
        {"obs": obs, "h_in": h_in, "c_in": c_in},
    )
    assert b_val.shape == (batch, 1)
    assert b_hv.shape == (1, batch, 32)
    assert b_cv.shape == (1, batch, 32)

    for i in range(batch):
        s_logits, s_h, s_c = actor_sess.run(
            ["logits", "h_out", "c_out"],
            {"obs": obs[i : i + 1], "h_in": h_in[:, i : i + 1, :], "c_in": c_in[:, i : i + 1, :]},
        )
        np.testing.assert_allclose(b_logits[i], s_logits[0], atol=1e-5, rtol=1e-4)
        np.testing.assert_allclose(b_h[:, i, :], s_h[:, 0, :], atol=1e-5, rtol=1e-4)
        np.testing.assert_allclose(b_c[:, i, :], s_c[:, 0, :], atol=1e-5, rtol=1e-4)

        s_val, s_hv, s_cv = critic_sess.run(
            ["value", "h_out", "c_out"],
            {"obs": obs[i : i + 1], "h_in": h_in[:, i : i + 1, :], "c_in": c_in[:, i : i + 1, :]},
        )
        np.testing.assert_allclose(b_val[i], s_val[0], atol=1e-5, rtol=1e-4)
        np.testing.assert_allclose(b_hv[:, i, :], s_hv[:, 0, :], atol=1e-5, rtol=1e-4)
        np.testing.assert_allclose(b_cv[:, i, :], s_cv[:, 0, :], atol=1e-5, rtol=1e-4)


def test_campaign_cli_iterative_lstm_and_architecture_flags(tmp_path: Path):
    from tools.run_hybrid_small_campaign import build_parser, make_config

    args = build_parser().parse_args(
        [
            "--run-dir",
            str(tmp_path / "iter_run"),
            "--mode",
            "iterative-lstm",
            "--rounds",
            "3",
            "--iterative-search-generations",
            "2",
            "--net-arch",
            "512,512",
            "--features-dim",
            "384",
            "--lstm-hidden-size",
            "384",
            "--macro-substep-min-prior",
            "0.85",
            "--weight-decay",
            "1e-4",
            "--value-label-smoothing",
            "0.05",
        ]
    )
    cfg = make_config(args)
    assert cfg.mode == "iterative-lstm"
    assert cfg.iterative_rounds == 3
    assert cfg.iterative_search_generations == 2
    assert cfg.net_arch == (512, 512)
    assert cfg.features_dim == 384
    assert cfg.lstm_hidden_size == 384
    assert cfg.macro_substep_min_prior == pytest.approx(0.85)
    assert cfg.weight_decay == pytest.approx(1e-4)
    assert cfg.value_label_smoothing == pytest.approx(0.05)

