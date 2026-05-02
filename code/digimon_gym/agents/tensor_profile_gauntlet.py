"""Tensor profile profiling gauntlet for RL observation layouts."""

from __future__ import annotations

import json
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, Iterable

from digimon_gym.tensor_profiles import TensorProfile, get_tensor_profile


DEFAULT_PROFILE_REQUESTS = (
    "compact_v1",
    "standard_lite_v2",
    "standard_full_v2",
)


@dataclass(frozen=True)
class ResolvedProfile:
    requested_profile: str
    profile: TensorProfile | Any | None
    available: bool
    skip_reason: str = ""


@dataclass(frozen=True)
class TensorProfileRunConfig:
    profiles: tuple[str, ...] = DEFAULT_PROFILE_REQUESTS
    games_per_profile: int = 25
    seeds: tuple[int, ...] = tuple(range(101, 126))
    max_steps_per_game: int = 1000
    policy: str = "greedy"
    require_profiles: bool = False
    n_steps: int = 128
    n_envs: int = 1


@dataclass(frozen=True)
class TensorProfileRunResult:
    requested_profile: str
    profile_id: str
    available: bool
    skip_reason: str
    tensor_size: int
    layout_hash: str
    feature_schema_version: str
    memory_footprint: dict[str, float | int]
    games_played: int
    steps: int
    elapsed_seconds: float
    wins: int
    losses: int
    draws: int
    trigger_order_correct: int
    trigger_order_total: int

    @property
    def steps_per_second(self) -> float:
        return self.steps / self.elapsed_seconds if self.elapsed_seconds > 0 else 0.0

    @property
    def games_per_hour(self) -> float:
        return self.games_played / (self.elapsed_seconds / 3600.0) if self.elapsed_seconds > 0 else 0.0

    @property
    def win_rate_vs_greedy(self) -> float:
        return self.wins / self.games_played if self.games_played else 0.0

    @property
    def trigger_order_accuracy(self) -> float:
        return self.trigger_order_correct / self.trigger_order_total if self.trigger_order_total else 0.0

    def to_dict(self) -> dict[str, Any]:
        data = asdict(self)
        data["steps_per_second"] = self.steps_per_second
        data["games_per_hour"] = self.games_per_hour
        data["win_rate_vs_greedy"] = self.win_rate_vs_greedy
        data["trigger_order_accuracy"] = self.trigger_order_accuracy
        return data


@dataclass(frozen=True)
class TensorProfileGauntletResult:
    config: TensorProfileRunConfig
    results: tuple[TensorProfileRunResult, ...]

    def to_dict(self) -> dict[str, Any]:
        return {
            "config": asdict(self.config),
            "results": [result.to_dict() for result in self.results],
        }

    def write_json(self, path: Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(self.to_dict(), indent=2, sort_keys=True), encoding="utf-8")


def resolve_profile_requests(
    profile_ids: Iterable[str],
    require_profiles: bool,
) -> list[ResolvedProfile]:
    resolved: list[ResolvedProfile] = []
    for requested in profile_ids:
        try:
            profile = get_tensor_profile(requested)
        except Exception as exc:
            if require_profiles:
                raise ValueError(f"required tensor profile {requested!r} is unavailable: {exc}") from exc
            resolved.append(
                ResolvedProfile(
                    requested_profile=requested,
                    profile=None,
                    available=False,
                    skip_reason=str(exc),
                )
            )
            continue
        resolved.append(
            ResolvedProfile(
                requested_profile=requested,
                profile=profile,
                available=True,
            )
        )
    return resolved


def estimate_memory_footprint(
    profile: TensorProfile | Any,
    n_steps: int,
    n_envs: int,
) -> dict[str, float | int]:
    tensor_bytes = int(profile.tensor_size) * 4
    rollout_observation_bytes = tensor_bytes * int(n_steps) * int(n_envs)
    return {
        "tensor_bytes": tensor_bytes,
        "tensor_kib": tensor_bytes / 1024,
        "rollout_observation_bytes": rollout_observation_bytes,
        "rollout_observation_mib": rollout_observation_bytes / 1024 / 1024,
        "card_embedding_input_slots": int(profile.card_id_slot_count),
        "scalar_input_slots": int(profile.scalar_slot_count),
    }
