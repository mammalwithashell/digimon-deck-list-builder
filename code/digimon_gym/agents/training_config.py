"""Versioned hyperparameter config for pilot training."""

from __future__ import annotations

from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, Dict, Optional

import yaml


VALID_ALGORITHMS = {"mlp", "lstm"}
VALID_OPPONENTS = {"greedy", "random", "agent", "pool", "self-play"}


@dataclass
class TrainingConfig:
    algorithm: str = "mlp"
    timesteps: int = 500_000
    seed: int = 0
    n_envs: int = 1
    learning_rate: float = 3e-4
    n_steps: int = 2048
    batch_size: int = 64
    n_epochs: int = 10
    gamma: float = 0.99
    gae_lambda: float = 0.95
    clip_range: float = 0.2
    ent_coef: float = 0.01
    vf_coef: float = 0.5
    max_grad_norm: float = 0.5
    lstm_hidden_size: int = 256
    opponent: str = "greedy"
    opponent_pool_manifest: Optional[str] = None
    opponent_pool_mode: str = "pfsp"
    deck_pool_variants: bool = False
    gauntlet_path: Optional[str] = None
    eval_freq: int = 25_000
    eval_episodes: int = 50
    eval_suite: Optional[str] = None
    checkpoint_every: int = 50_000
    keep_last_checkpoints: int = 3
    resume_from: Optional[str] = None
    models_dir: str = "models"
    tensorboard_log: str = "runs/pilot_ppo"
    run_name: Optional[str] = None
    tensor_profile: str = "standard_compact_v1"

    def __post_init__(self) -> None:
        self._validate()

    @classmethod
    def from_yaml(
        cls,
        path: Path,
        overrides: Optional[Dict[str, Any]] = None,
    ) -> "TrainingConfig":
        raw = yaml.safe_load(Path(path).read_text()) or {}
        merged = {**raw, **(overrides or {})}
        known = {k: v for k, v in merged.items() if k in cls.__dataclass_fields__}
        cfg = cls(**known)
        cfg._validate()
        return cfg

    def _validate(self) -> None:
        if self.algorithm not in VALID_ALGORITHMS:
            raise ValueError(
                f"algorithm must be one of {sorted(VALID_ALGORITHMS)}, got {self.algorithm}"
            )
        if self.opponent not in VALID_OPPONENTS:
            raise ValueError(
                f"opponent must be one of {sorted(VALID_OPPONENTS)}, got {self.opponent}"
            )
        if self.timesteps <= 0:
            raise ValueError("timesteps must be > 0")
        if self.n_envs < 1:
            raise ValueError("n_envs must be >= 1")
        if self.n_steps <= 0:
            raise ValueError("n_steps must be > 0")
        if self.batch_size <= 0:
            raise ValueError("batch_size must be > 0")
        if self.eval_freq < 0:
            raise ValueError("eval_freq must be >= 0")
        if self.eval_episodes <= 0:
            raise ValueError("eval_episodes must be > 0")
        if self.checkpoint_every < 0:
            raise ValueError("checkpoint_every must be >= 0")
        if self.keep_last_checkpoints < 1:
            raise ValueError("keep_last_checkpoints must be >= 1")
        if self.opponent == "pool" and not self.opponent_pool_manifest:
            raise ValueError("opponent=pool requires opponent_pool_manifest")
        if not isinstance(self.tensor_profile, str) or not self.tensor_profile.strip():
            raise ValueError("tensor_profile must be a non-blank string")

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)
