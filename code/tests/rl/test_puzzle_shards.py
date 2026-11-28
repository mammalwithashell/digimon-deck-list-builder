"""Tests for `code/tools/emit_puzzle_shards.py` + contrastive `avoid` margin in AZ trainers."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import gymnasium as gym
import numpy as np
import pytest

pytest.importorskip("torch")
pytest.importorskip("sb3_contrib")

import torch  # noqa: E402
from sb3_contrib import MaskablePPO  # noqa: E402

_REPO_ROOT = Path(__file__).resolve().parents[3]
_TOOLS_DIR = _REPO_ROOT / "code" / "tools"
if str(_TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(_TOOLS_DIR))

from digimon_gym.agents import azero_lstm_distill, azero_trainer  # noqa: E402
from digimon_gym.agents.features_extractor import CardEmbeddingExtractor  # noqa: E402
from digimon_gym.agents.maskable_recurrent import (  # noqa: E402
    MaskableMlpLstmPolicy,
    MaskableRecurrentPPO,
)
from emit_puzzle_shards import emit_puzzle_shards, main as emit_cli_main  # noqa: E402
from model_contract import current_contract  # noqa: E402

IMPERIALDRAMON_EXAM = (
    _REPO_ROOT / "qa" / "search-exams" / "ex13-imperialdramon-dna-paildramon-jamming-lethal.json"
)
FILLER_DECK = ["ST1-01"] * 4 + ["ST1-03"] * 25 + ["ST1-07"] * 25


class _ContractSpacesEnv(gym.Env):
    def __init__(self, profile: str = "standard_lite_v2"):
        super().__init__()
        contract = current_contract(profile)
        self.observation_space = gym.spaces.Box(
            -10.0, 20001.0, (contract["tensor_size"],), np.float32
        )
        self.action_space = gym.spaces.Discrete(contract["action_space_size"])

    def reset(self, *, seed=None, options=None):
        return np.zeros(self.observation_space.shape, dtype=np.float32), {}

    def step(self, action):
        return np.zeros(self.observation_space.shape, dtype=np.float32), 0.0, True, False, {}


def _write_synthetic_studio_puzzle(path: Path) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    doc = {
        "schema": "studio-puzzle-v1",
        "id": "synth-evo-over-pass",
        "title": "Digivolve Greymon and swing lethal instead of passing or hard-playing",
        "description": "synthetic puzzle with target and avoid lines",
        "folder": "synthetic",
        "tags": ["lethal"],
        "source": {"kind": "composed"},
        "decks": {"1": FILLER_DECK, "2": FILLER_DECK},
        "seed": 3,
        "seat": 1,
        "setup": {
            "kind": "staged",
            "state": {"memory": 4, "phase": "Main", "turn": 3, "first_player": 1},
            "zones": {
                "1": {
                    "hand": ["ST1-07", "ST1-03"],
                    "field": [{"stack": ["ST1-03"], "is_suspended": False, "turn_played": 1}],
                    "security": ["ST1-03"],
                    "trash": [],
                },
                "2": {
                    "hand": [],
                    "field": [],
                    "security": [],
                    "trash": [],
                },
            },
            "prefix_actions": [],
        },
        "lines": [
            {"id": "target-win", "name": "evo-swing", "role": "target", "actions": [400, 114]},
            {"id": "avoid-pass", "name": "bad-pass", "role": "avoid", "actions": [62]},
            {"id": "avoid-play", "name": "bad-play", "role": "avoid", "actions": [1]},
        ],
        "expectations": [],
        "assertions": {"engine": [], "ui": []},
        "rl_goal": {"mode": "playout_to_win"},
    }
    path.write_text(json.dumps(doc), encoding="utf-8")
    return path


def _mlp_masked_probs(model: MaskablePPO, obs_row: np.ndarray, legal_actions: np.ndarray) -> np.ndarray:
    policy = model.policy
    policy.set_training_mode(False)
    dev = policy.device
    obs_t = torch.as_tensor(obs_row[None, :], dtype=torch.float32, device=dev)
    mask_np = np.zeros((1, policy.action_space.n), dtype=bool)
    mask_np[0, legal_actions] = True
    mask_t = torch.as_tensor(mask_np, device=dev)
    with torch.no_grad():
        logits, _ = azero_trainer._forward(policy, obs_t)
        masked = logits.masked_fill(~mask_t, torch.finfo(logits.dtype).min)
        probs = torch.softmax(masked, dim=-1)[0].cpu().numpy()
    return probs


def _lstm_masked_probs(
    model: MaskableRecurrentPPO, obs_row: np.ndarray, legal_actions: np.ndarray
) -> np.ndarray:
    policy = model.policy
    policy.set_training_mode(False)
    dev = policy.device
    obs_tb = torch.as_tensor(obs_row.reshape(1, 1, -1), dtype=torch.float32, device=dev)
    mask_np = np.zeros((1, 1, policy.action_space.n), dtype=bool)
    mask_np[0, 0, legal_actions] = True
    mask_t = torch.as_tensor(mask_np, device=dev)
    with torch.no_grad():
        logits, _ = azero_lstm_distill._forward_sequences(policy, obs_tb)
        masked = logits.masked_fill(~mask_t, torch.finfo(logits.dtype).min)
        probs = torch.softmax(masked, dim=-1)[0, 0].cpu().numpy()
    return probs


def test_emit_puzzle_shards_on_imperialdramon_search_exam(tmp_path: Path):
    shard_out = tmp_path / "imperialdramon_shards"
    manifest = emit_puzzle_shards(
        [IMPERIALDRAMON_EXAM],
        shard_out,
        permutations_per_puzzle=8,
        observation_profile="standard_lite_v2",
        seed=7,
    )

    expected_contract = current_contract("standard_lite_v2")
    assert manifest["schema"] == "selfplay-shards-v1"
    assert manifest["model_contract"] == expected_contract
    assert manifest["games"] == 9  # canonical + 8 permutations
    assert manifest["avoid_edges"] > 0
    assert manifest["forced_decisions_total"] >= 9  # 2nd DNA material step is forced (1 legal choice)

    s0 = shard_out / "shard-000"
    for fname in (
        "obs.npy",
        "policy_offsets.npy",
        "policy_actions.npy",
        "policy_visits.npy",
        "z.npy",
        "terminated.npy",
        "game_id.npy",
        "mover.npy",
        "policy_weight.npy",
        "avoid_offsets.npy",
        "avoid_actions.npy",
    ):
        assert (s0 / fname).exists(), f"missing {fname}"

    ds = azero_trainer.load_shards([shard_out])
    seq_ds = azero_lstm_distill.load_sequence_shards([shard_out])
    seq_ds_incl = azero_lstm_distill.load_sequence_shards(
        [shard_out], include_forced_selection=True
    )
    forced_rows = int((ds.forced_selection != 0).sum())
    assert forced_rows == manifest["forced_selection_rows"]
    assert forced_rows >= 9
    assert len(ds) == manifest["rows"]
    assert seq_ds.total_rows == manifest["rows"] - forced_rows
    assert seq_ds_incl.total_rows == manifest["rows"]
    assert len(seq_ds.sequences) == 9
    assert np.all(ds.z == 1.0)
    assert np.all(ds.terminated)

    # Row 0 is the canonical Imperialdramon root: target=63, avoid={114, 129, 400, 401}.
    r0_acts = ds.actions[ds.offsets[0] : ds.offsets[1]]
    r0_vis = ds.visits[ds.offsets[0] : ds.offsets[1]]
    assert int(r0_acts[int(np.argmax(r0_vis))]) == 63
    assert float(r0_vis.max()) == pytest.approx(100.0)
    assert float(r0_vis.sum()) == pytest.approx(100.0)
    r0_avoid = set(ds.avoid_actions[ds.avoid_offsets[0] : ds.avoid_offsets[1]].tolist())
    assert r0_avoid == {114, 129, 400, 401}

    # Across permuted trajectories, root DNA hand index varies beyond slot 0 (action 63).
    root_targets = set()
    for seq in seq_ds.sequences:
        a0 = seq.actions[seq.offsets[0] : seq.offsets[1]]
        v0 = seq.visits[seq.offsets[0] : seq.offsets[1]]
        root_targets.add(int(a0[int(np.argmax(v0))]))
    assert len(root_targets) >= 2


def test_azero_trainer_and_lstm_distill_boost_target_and_suppress_avoid(tmp_path: Path):
    synth_puzzle = _write_synthetic_studio_puzzle(tmp_path / "puzzles" / "synth.json")
    shard_dir = tmp_path / "pooled_puzzle_shards"

    rc = emit_cli_main(
        [
            "--inputs",
            str(IMPERIALDRAMON_EXAM),
            str(synth_puzzle),
            "--out",
            str(shard_dir),
            "--permutations-per-puzzle",
            "12",
            "--observation-profile",
            "standard_lite_v2",
            "--seed",
            "11",
        ]
    )
    assert rc == 0

    ds = azero_trainer.load_shards([shard_dir])
    assert len(ds.avoid_actions) > 0

    # Identify canonical Imperialdramon root (row 0) and canonical synthetic puzzle root.
    imp_obs = np.asarray(ds.obs[0], dtype=np.float32)
    imp_legal = ds.actions[ds.offsets[0] : ds.offsets[1]]
    imp_avoid = ds.avoid_actions[ds.avoid_offsets[0] : ds.avoid_offsets[1]]
    assert 63 in imp_legal and len(imp_avoid) == 4

    # Find first row belonging to the synthetic puzzle (where target is 400 and avoid contains 62).
    synth_row_idx = None
    for i in range(len(ds)):
        s, e = ds.offsets[i], ds.offsets[i + 1]
        acts_i = ds.actions[s:e]
        vis_i = ds.visits[s:e]
        av_i = ds.avoid_actions[ds.avoid_offsets[i] : ds.avoid_offsets[i + 1]]
        if int(acts_i[int(np.argmax(vis_i))]) == 400 and 62 in set(av_i.tolist()):
            synth_row_idx = i
            break
    assert synth_row_idx is not None
    synth_obs = np.asarray(ds.obs[synth_row_idx], dtype=np.float32)
    synth_legal = ds.actions[ds.offsets[synth_row_idx] : ds.offsets[synth_row_idx + 1]]
    synth_avoid = ds.avoid_actions[
        ds.avoid_offsets[synth_row_idx] : ds.avoid_offsets[synth_row_idx + 1]
    ]

    # ── 1. MLP AlphaZero Trainer (2 epochs) ──────────────────────────────
    spaces_env = _ContractSpacesEnv("standard_lite_v2")
    mlp_init = MaskablePPO(
        "MlpPolicy",
        spaces_env,
        n_steps=16,
        batch_size=16,
        seed=0,
        device="cpu",
        verbose=0,
        policy_kwargs=dict(
            net_arch=dict(pi=[64], vf=[64]),
            features_extractor_class=CardEmbeddingExtractor,
            features_extractor_kwargs=dict(
                features_dim=64,
                embedding_dim=8,
                tensor_profile_id="standard_lite_v2",
            ),
        ),
    )
    mlp_init.digimon_contract = current_contract("standard_lite_v2")
    mlp_init_zip = tmp_path / "mlp_init.zip"
    mlp_out_zip = tmp_path / "mlp_out.zip"
    mlp_init.save(str(mlp_init_zip))

    imp_before_mlp = _mlp_masked_probs(mlp_init, imp_obs, imp_legal)
    synth_before_mlp = _mlp_masked_probs(mlp_init, synth_obs, synth_legal)

    mlp_summary = azero_trainer.train(
        init_from=mlp_init_zip,
        shard_dirs=[shard_dir],
        out=mlp_out_zip,
        epochs=2,
        batch_size=8,
        lr=3e-3,
        avoid_margin_coef=0.5,
        avoid_margin=1.5,
        device="cpu",
        seed=0,
    )
    assert "avoid_loss" in mlp_summary["epochs"][0]
    mlp_trained = MaskablePPO.load(str(mlp_out_zip), device="cpu")
    imp_after_mlp = _mlp_masked_probs(mlp_trained, imp_obs, imp_legal)
    synth_after_mlp = _mlp_masked_probs(mlp_trained, synth_obs, synth_legal)

    assert imp_after_mlp[63] > imp_before_mlp[63]
    assert imp_after_mlp[imp_avoid].sum() < imp_before_mlp[imp_avoid].sum()
    assert synth_after_mlp[400] > synth_before_mlp[400]
    assert synth_after_mlp[synth_avoid].sum() < synth_before_mlp[synth_avoid].sum()

    # ── 2. Recurrent LSTM Distillation Trainer (2 epochs) ────────────────
    lstm_init = MaskableRecurrentPPO(
        MaskableMlpLstmPolicy,
        spaces_env,
        n_steps=16,
        batch_size=16,
        seed=0,
        device="cpu",
        verbose=0,
        policy_kwargs=dict(
            lstm_hidden_size=64,
            n_lstm_layers=1,
            enable_critic_lstm=True,
            net_arch=dict(pi=[64], vf=[64]),
            features_extractor_class=CardEmbeddingExtractor,
            features_extractor_kwargs=dict(
                features_dim=64,
                embedding_dim=8,
                tensor_profile_id="standard_lite_v2",
            ),
        ),
    )
    lstm_init.digimon_contract = current_contract("standard_lite_v2")
    lstm_init_zip = tmp_path / "lstm_init.zip"
    lstm_out_zip = tmp_path / "lstm_out.zip"
    lstm_init.save(str(lstm_init_zip))

    imp_before_lstm = _lstm_masked_probs(lstm_init, imp_obs, imp_legal)
    synth_before_lstm = _lstm_masked_probs(lstm_init, synth_obs, synth_legal)

    lstm_summary = azero_lstm_distill.distill_lstm(
        shard_dirs=[shard_dir],
        out=lstm_out_zip,
        init_from=lstm_init_zip,
        epochs=2,
        seq_batch_size=1,
        lr=3e-3,
        holdout_fraction=0.0,
        avoid_margin_coef=0.5,
        avoid_margin=1.5,
        device="cpu",
        seed=0,
    )
    assert "avoid_loss" in lstm_summary["epochs"][0]
    lstm_trained = MaskableRecurrentPPO.load(str(lstm_out_zip), device="cpu")
    imp_after_lstm = _lstm_masked_probs(lstm_trained, imp_obs, imp_legal)
    synth_after_lstm = _lstm_masked_probs(lstm_trained, synth_obs, synth_legal)

    assert imp_after_lstm[63] > imp_before_lstm[63]
    assert imp_after_lstm[imp_avoid].sum() < imp_before_lstm[imp_avoid].sum()
    assert synth_after_lstm[400] > synth_before_lstm[400]
    assert synth_after_lstm[synth_avoid].sum() < synth_before_lstm[synth_avoid].sum()
