"""AlphaZero trainer tests (add-determinized-search task 3.2).

Synthetic ``selfplay-shards-v1`` directories + a tiny untrained
``MaskablePPO`` — no engine, no real shards. Validates the shard loader's
multi-dir offset stitching, that the AZ update actually moves the policy
toward the π targets and the value head toward z, and that the saved
checkpoint round-trips through SB3.
"""

from __future__ import annotations

import json
from pathlib import Path

import numpy as np
import pytest

pytest.importorskip("torch")
pytest.importorskip("sb3_contrib")

import gymnasium as gym  # noqa: E402
import torch  # noqa: E402
from sb3_contrib import MaskablePPO  # noqa: E402

from digimon_gym.agents.azero_trainer import load_shards, train  # noqa: E402

OBS_DIM = 12
ACTION_DIM = 9
PROFILE = "test_profile_v0"


class _TinyEnv(gym.Env):
    """Spaces-only env: MaskablePPO needs one to build the policy nets."""

    observation_space = gym.spaces.Box(-1.0, 1.0, (OBS_DIM,), np.float32)
    action_space = gym.spaces.Discrete(ACTION_DIM)

    def reset(self, *, seed=None, options=None):  # pragma: no cover - unused
        return np.zeros(OBS_DIM, np.float32), {}

    def step(self, action):  # pragma: no cover - unused
        return np.zeros(OBS_DIM, np.float32), 0.0, True, False, {}


def _write_shard_dir(root: Path, rows: int, seed: int) -> Path:
    """A learnable synthetic generation: obs[0] = ±1 decides both targets.

    obs[0] = +1 → π mass on action 1 (legal {1, 3, 5}), z = +1
    obs[0] = -1 → π mass on action 3 (legal {1, 3, 5}), z = -1
    """
    rng = np.random.default_rng(seed)
    obs = rng.normal(0, 0.1, (rows, OBS_DIM)).astype(np.float32)
    signs = np.where(np.arange(rows) % 2 == 0, 1.0, -1.0).astype(np.float32)
    obs[:, 0] = signs
    z = signs.copy()

    offsets = [0]
    actions: list[int] = []
    visits: list[float] = []
    for s in signs:
        legal = [1, 3, 5]
        vis = [80.0, 10.0, 10.0] if s > 0 else [10.0, 80.0, 10.0]
        actions.extend(legal)
        visits.extend(vis)
        offsets.append(len(actions))

    shard = root / "shard-000"
    shard.mkdir(parents=True)
    np.save(shard / "obs.npy", obs)
    np.save(shard / "policy_offsets.npy", np.asarray(offsets, np.int64))
    np.save(shard / "policy_actions.npy", np.asarray(actions, np.int32))
    np.save(shard / "policy_visits.npy", np.asarray(visits, np.float32))
    np.save(shard / "z.npy", z)
    np.save(shard / "mover.npy", np.zeros(rows, np.uint8))
    np.save(shard / "game_id.npy", np.zeros(rows, np.int32))
    (root / "manifest.json").write_text(
        json.dumps(
            {
                "schema": "selfplay-shards-v1",
                "obs_dim": OBS_DIM,
                "action_dim": ACTION_DIM,
                "observation_profile": PROFILE,
                "shards": [{"dir": "shard-000", "rows": rows}],
            }
        )
    )
    return root


def test_loader_stitches_multiple_dirs(tmp_path):
    a = _write_shard_dir(tmp_path / "gen0", rows=6, seed=0)
    b = _write_shard_dir(tmp_path / "gen1", rows=4, seed=1)
    ds = load_shards([a, b])
    assert len(ds) == 10
    assert ds.offsets.shape == (11,)
    assert ds.offsets[0] == 0
    assert ds.offsets[-1] == len(ds.actions) == 30  # 3 legal actions per row
    assert np.all(np.diff(ds.offsets) == 3)
    # Row 6 is gen1's row 0 — its slice must land on gen1 data, not gen0's.
    s, e = ds.offsets[6], ds.offsets[7]
    assert list(ds.actions[s:e]) == [1, 3, 5]


def test_train_moves_policy_and_value_toward_targets(tmp_path):
    shard_dir = _write_shard_dir(tmp_path / "gen0", rows=32, seed=2)
    model = MaskablePPO("MlpPolicy", _TinyEnv(), n_steps=8, batch_size=8, verbose=0)
    init_zip = tmp_path / "init.zip"
    model.save(str(init_zip))

    out_zip = tmp_path / "gen1.zip"
    summary = train(
        init_from=init_zip,
        shard_dirs=[shard_dir],
        out=out_zip,
        epochs=30,
        batch_size=16,
        lr=1e-2,
        device="cpu",
        seed=0,
    )
    assert out_zip.exists()
    losses = summary["epochs"]
    assert losses[-1]["policy_loss"] < losses[0]["policy_loss"]
    assert losses[-1]["value_loss"] < losses[0]["value_loss"]

    # The trained checkpoint round-trips and its predictions track targets.
    trained = MaskablePPO.load(str(out_zip), device="cpu")
    ds = load_shards([shard_dir])
    obs_t = torch.as_tensor(ds.obs)
    with torch.no_grad():
        values = trained.policy.predict_values(obs_t).squeeze(-1).numpy()
        dist = trained.policy.get_distribution(obs_t)
        probs = dist.distribution.probs.numpy()
    # Value head learned the sign of z.
    assert float(np.mean(values * ds.z)) > 0.5
    # Policy puts more mass on the π-preferred action than the alternative.
    pos = ds.obs[:, 0] > 0
    assert np.all(probs[pos, 1] > probs[pos, 3])
    assert np.all(probs[~pos, 3] > probs[~pos, 1])


def test_loader_rejects_profile_mismatch(tmp_path):
    a = _write_shard_dir(tmp_path / "gen0", rows=2, seed=0)
    b = _write_shard_dir(tmp_path / "gen1", rows=2, seed=1)
    manifest = json.loads((b / "manifest.json").read_text())
    manifest["observation_profile"] = "other_profile"
    (b / "manifest.json").write_text(json.dumps(manifest))
    with pytest.raises(ValueError, match="observation_profile"):
        load_shards([a, b])
