"""Tests for run-provenance hardening (`harden-training-pipeline` D5).

- `resolve_git_sha`: HEAD inside a git checkout, "unknown" outside.
- `TrainingRunMetadata` round-trips the new provenance fields and stays
  backward-compatible with pre-feature sidecars.
"""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

import pytest

from digimon_gym.agents.training_metrics import (
    TrainingRunMetadata,
    resolve_git_sha,
)


class TestResolveGitSha:
    def test_returns_head_inside_repo(self):
        sha = resolve_git_sha(Path(__file__).parent)
        expected = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=str(Path(__file__).parent),
            capture_output=True,
            text=True,
        ).stdout.strip()
        assert sha == expected
        assert len(sha) == 40

    def test_returns_unknown_outside_repo(self, tmp_path, monkeypatch):
        # A bare tmp dir is not a git repo; also defeat discovery of any
        # parent repo by pointing GIT_DIR at a nonexistent location.
        monkeypatch.setenv("GIT_DIR", str(tmp_path / "no-such-git-dir"))
        assert resolve_git_sha(tmp_path) == "unknown"

    def test_returns_unknown_when_git_binary_missing(self, monkeypatch):
        def boom(*args, **kwargs):
            raise FileNotFoundError("git not on PATH")

        monkeypatch.setattr(subprocess, "run", boom)
        assert resolve_git_sha() == "unknown"


class TestJobRunnerInitFromForwarding:
    def test_train_kwargs_include_new_keys(self):
        from tools.run_training_job import _TRAIN_KWARGS

        assert "init_from" in _TRAIN_KWARGS
        assert "opponent_pool_manifest" in _TRAIN_KWARGS
        assert "opponent_pool_mode" in _TRAIN_KWARGS

    def test_run_job_forwards_init_from_to_train(self, monkeypatch, tmp_path):
        import json as _json

        from tools import run_training_job

        received = {}

        def fake_train(**kwargs):
            received.update(kwargs)

        monkeypatch.setattr(
            "digimon_gym.agents.pilot_training.train", fake_train
        )
        monkeypatch.setattr(
            run_training_job, "build_gauntlet", lambda *a, **k: None
        )
        config = {
            "job_name": "init-from-forwarding",
            "training": {
                "timesteps": 1,
                "init_from": "/workspace/models/v22/final.zip",
                "opponent": "pool",
                "opponent_pool_manifest": "/workspace/pool.json",
            },
        }
        config_path = tmp_path / "job.json"
        config_path.write_text(_json.dumps(config))

        run_training_job.run_job(str(config_path))

        assert received["init_from"] == "/workspace/models/v22/final.zip"
        assert received["opponent"] == "pool"
        assert received["opponent_pool_manifest"] == "/workspace/pool.json"

    def test_kwargs_path_init_from_reaches_contract_validation(self, tmp_path):
        from digimon_gym.agents import pilot_training

        missing = tmp_path / "no_such_checkpoint.zip"
        with pytest.raises(ValueError, match="metadata sidecar not found"):
            pilot_training.train(
                total_timesteps=1,
                init_from=str(missing),
                save_dir=str(tmp_path / "models"),
                tensorboard_log=str(tmp_path / "runs"),
                verbose=0,
            )

    def test_init_from_and_resume_from_mutually_exclusive(self, tmp_path):
        from digimon_gym.agents import pilot_training
        from digimon_gym.agents.training_config import TrainingConfig

        cfg = TrainingConfig(
            timesteps=1,
            init_from="a.zip",
            resume_from="b.zip",
            models_dir=str(tmp_path / "models"),
            tensorboard_log=str(tmp_path / "runs"),
        )
        with pytest.raises(ValueError, match="mutually exclusive"):
            pilot_training.train(cfg=cfg, verbose=0)


class TestProvenanceSidecarFields:
    def test_round_trip(self, tmp_path):
        meta = TrainingRunMetadata(
            run_id="run-1",
            started_at="2026-06-11T00:00:00",
            git_sha="a" * 40,
            bounty_threshold=0.25,
            bounty_bonus=0.75,
            opponent_pool_manifest="models/pool.json",
        )
        path = tmp_path / "meta.json"
        meta.save(path)
        loaded = TrainingRunMetadata.load(path)
        assert loaded.git_sha == "a" * 40
        assert loaded.bounty_threshold == 0.25
        assert loaded.bounty_bonus == 0.75
        assert loaded.opponent_pool_manifest == "models/pool.json"

    def test_pre_feature_sidecar_loads_with_defaults(self, tmp_path):
        # A sidecar written before the provenance fields existed must
        # load cleanly with neutral defaults.
        legacy = {
            "run_id": "old-run",
            "started_at": "2026-05-01T00:00:00",
        }
        path = tmp_path / "legacy.json"
        path.write_text(json.dumps(legacy))
        loaded = TrainingRunMetadata.load(path)
        assert loaded.git_sha == ""
        assert loaded.bounty_threshold == 0.0
        assert loaded.bounty_bonus == 0.0
        assert loaded.opponent_pool_manifest == ""
