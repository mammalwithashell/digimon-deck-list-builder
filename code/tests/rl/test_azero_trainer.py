"""AlphaZero trainer tests (add-determinized-search task 3.2).

Synthetic ``selfplay-shards-v1`` directories + a tiny untrained
``MaskablePPO`` — no engine, no real shards. Validates the shard loader's
multi-dir offset stitching, that the AZ update actually moves the policy
toward the π targets and the value head toward z, and that the saved
checkpoint round-trips through SB3.
"""

from __future__ import annotations

import json
import hashlib
from pathlib import Path

import numpy as np
import pytest

pytest.importorskip("torch")
pytest.importorskip("sb3_contrib")

import gymnasium as gym  # noqa: E402
import torch  # noqa: E402
from sb3_contrib import MaskablePPO  # noqa: E402

from digimon_gym.agents.azero_trainer import calibrate_value, load_shards, train  # noqa: E402

OBS_DIM = 12
ACTION_DIM = 9
PROFILE = "test_profile_v0"


def _contract():
    return dict(observation_profile=PROFILE, tensor_version=2, feature_schema_version="test-v1",
                tensor_size=OBS_DIM, tensor_layout_hash="test-hash", action_space_size=ACTION_DIM,
                action_schema_version=1)


@pytest.fixture(autouse=True)
def synthetic_contract_registry(monkeypatch):
    # Synthetic network dimensions only. Production validation remains strict.
    monkeypatch.setattr("model_contract.current_contract", lambda profile: _contract())


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
    np.save(shard / "terminated.npy", np.ones(rows, np.uint8))
    np.save(shard / "mover.npy", np.zeros(rows, np.uint8))
    np.save(shard / "game_id.npy", np.arange(rows, dtype=np.int32) // 2)
    (root / "manifest.json").write_text(
        json.dumps(
            {
                "schema": "selfplay-shards-v1",
                "obs_dim": OBS_DIM,
                "action_dim": ACTION_DIM,
                "observation_profile": PROFILE,
                "model_contract": _contract(),
                "value_target": "terminal_outcome",
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
    expected = np.concatenate([np.load(p / 'shard-000/obs.npy') for p in (a, b)])
    assert isinstance(ds.obs, np.memmap)
    assert ds.obs.tobytes() == expected.tobytes()
    assert ds.obs[[9, 0, 6, 2]].tobytes() == expected[[9, 0, 6, 2]].tobytes()
    assert ds.obs.flags.writeable  # private backing file; same ndarray contract


def _exclude_games(directory, ids):
    (directory / "training_exclusions.json").write_text(json.dumps({
        "schema": "selfplay-training-exclusions-v1",
        "manifest_sha256": hashlib.sha256((directory / "manifest.json").read_bytes()).hexdigest(),
        "games": [{"game_id": gid, "reason": "verified repeated-state loop"} for gid in ids],
    }))


def test_game_exclusions_preserve_sparse_rows_and_other_chunks(tmp_path):
    a = _write_shard_dir(tmp_path / "a", rows=6, seed=0)
    b = _write_shard_dir(tmp_path / "b", rows=4, seed=1)
    original = load_shards([a, b])
    _exclude_games(a, [1])
    filtered = load_shards([a, b])
    keep = [0, 1, 4, 5, 6, 7, 8, 9]
    np.testing.assert_array_equal(filtered.obs, original.obs[keep])
    np.testing.assert_array_equal(filtered.game_ids, original.game_ids[keep])
    np.testing.assert_array_equal(filtered.z, original.z[keep])
    for new, old in enumerate(keep):
        start, end = filtered.offsets[new:new+2]
        old_start, old_end = original.offsets[old:old+2]
        np.testing.assert_array_equal(filtered.actions[start:end], original.actions[old_start:old_end])
        np.testing.assert_array_equal(filtered.visits[start:end], original.visits[old_start:old_end])
    assert filtered.exclusions[0]["excluded_rows"] == 2
    assert filtered.exclusions[0]["games"][0]["game_id"] == 1


@pytest.mark.parametrize("invalid", ["hash", "unknown_game"])
def test_game_exclusions_fail_closed(tmp_path, invalid):
    a = _write_shard_dir(tmp_path / "a", rows=6, seed=0)
    _exclude_games(a, [999] if invalid == "unknown_game" else [1])
    if invalid == "hash":
        p = a / "manifest.json"
        p.write_text(p.read_text() + "\n")
    with pytest.raises(ValueError, match="exclusion"):
        load_shards([a])


def test_train_excludes_quarantined_game_from_both_losses(tmp_path):
    a = _write_shard_dir(tmp_path / "a", rows=6, seed=0)
    _exclude_games(a, [1])
    # Only the quarantined game is truncated; neither its policy nor value
    # targets may reach training. Remaining four rows fit normally.
    np.save(a / "shard-000/terminated.npy", np.array([1, 1, 0, 0, 1, 1], np.uint8))
    model = MaskablePPO("MlpPolicy", _TinyEnv(), n_steps=8, batch_size=8, verbose=0)
    model.digimon_contract = _contract()
    model.save(str(tmp_path / "seed.zip"))
    summary = train(init_from=tmp_path / "seed.zip", shard_dirs=[a], out=tmp_path / "fit.zip",
                    epochs=1, batch_size=4, device="cpu")
    assert summary["rows"] == 4
    assert summary["truncated_rows"] == 0
    assert summary["excluded_rows"] == 2
    assert summary["excluded_games"] == 1


def test_train_moves_policy_and_value_toward_targets(tmp_path):
    shard_dir = _write_shard_dir(tmp_path / "gen0", rows=32, seed=2)
    model = MaskablePPO("MlpPolicy", _TinyEnv(), n_steps=8, batch_size=8, verbose=0)
    model.digimon_contract = _contract()
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
    with pytest.raises(ValueError, match="profile"):
        load_shards([a, b])


def test_critic_calibration_preserves_actor_and_excludes_truncations(tmp_path):
    directory = _write_shard_dir(tmp_path / "data", rows=80, seed=3)
    ended = np.ones(80, np.uint8)
    ended[-10:] = 0
    np.save(directory / "shard-000" / "terminated.npy", ended)
    model = MaskablePPO("MlpPolicy", _TinyEnv(), n_steps=8, batch_size=8, verbose=0, seed=0, device="cpu")
    model.digimon_contract = _contract()
    with torch.no_grad():
        model.policy.value_net.bias.fill_(50.)  # a shaped-return head must be reset
    model.save(str(tmp_path / "seed.zip"))
    summary = calibrate_value(tmp_path / "seed.zip", [directory], tmp_path / "calibrated.zip",
                              epochs=20, batch_size=16, lr=1e-2, device="cpu")
    calibrated = MaskablePPO.load(str(tmp_path / "calibrated.zip"), device="cpu")
    obs = torch.as_tensor(load_shards([directory]).obs)
    with torch.no_grad():
        before = model.policy.get_distribution(obs).distribution.probs
        after = calibrated.policy.get_distribution(obs).distribution.probs
    assert torch.equal(before, after)
    assert summary["excluded_truncated_rows"] == 10
    assert summary["fit_rows"] + summary["heldout_rows"] == 70
    assert summary["history"][-1]["heldout_mse"] < 0.5
    assert calibrated.digimon_value_semantics == "terminal_outcome"


def test_loader_rejects_same_shape_stale_action_semantics(tmp_path):
    directory = _write_shard_dir(tmp_path / "data", rows=4, seed=0)
    path = directory / "manifest.json"
    manifest = json.loads(path.read_text())
    manifest["model_contract"]["action_schema_version"] += 1
    path.write_text(json.dumps(manifest))
    with pytest.raises(ValueError, match="action_schema_version"):
        load_shards([directory])


def test_loader_requires_terminal_semantics_and_completion_flags(tmp_path):
    directory = _write_shard_dir(tmp_path / "data", rows=4, seed=0)
    path = directory / "manifest.json"
    manifest = json.loads(path.read_text())
    manifest["value_target"] = "shaped_return"
    path.write_text(json.dumps(manifest))
    with pytest.raises(ValueError, match="terminal_outcome"):
        load_shards([directory])
