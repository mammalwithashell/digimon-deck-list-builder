"""Tensor profile profiling gauntlet for RL observation layouts."""

from __future__ import annotations

import json
import time
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, Callable, Iterable

from digimon_gym.agents.pilot_training import random_policy
from digimon_gym.digimon_gym import DigimonEnv, greedy_policy
from digimon_gym.tensor_profiles import TensorProfile, get_tensor_profile


DEFAULT_PROFILE_REQUESTS = (
    "compact_v1",
    "standard_lite_v2",
    "standard_full_v2",
)

DEFAULT_DECK = ("ST1-01",) * 5 + ("ST1-03",) * 45


def _policy_fn(name: str) -> Callable[[Any], int]:
    if name == "greedy":
        return greedy_policy
    if name == "random":
        return random_policy
    raise ValueError(f"unknown tensor profile gauntlet policy: {name}")


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

    def to_markdown(self) -> str:
        lines = [
            "# Tensor Profile Gauntlet",
            "",
            "| Profile | Tensor Size | Steps/sec | Games/hour | Win Rate vs Greedy | Trigger Accuracy | Tensor KiB |",
            "|---|---:|---:|---:|---:|---:|---:|",
        ]
        for result in self.results:
            profile_label = result.profile_id or result.requested_profile
            if not result.available:
                lines.append(f"| {profile_label} | unavailable | 0.00 | 0.00 | 0.00% | 0.00% | 0.00 |")
                continue
            lines.append(
                "| "
                + " | ".join(
                    [
                        profile_label,
                        str(result.tensor_size),
                        f"{result.steps_per_second:.2f}",
                        f"{result.games_per_hour:.2f}",
                        f"{result.win_rate_vs_greedy:.2%}",
                        f"{result.trigger_order_accuracy:.2%}",
                        f"{float(result.memory_footprint['tensor_kib']):.2f}",
                    ]
                )
                + " |"
            )
        skipped = [result for result in self.results if not result.available]
        if skipped:
            lines.extend(["", "## Skipped Profiles", ""])
            for result in skipped:
                lines.append(f"- `{result.requested_profile}`: {result.skip_reason}")
        lines.append("")
        return "\n".join(lines)

    def write_markdown(self, path: Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(self.to_markdown(), encoding="utf-8")


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


def score_trigger_order_accuracy(profile: TensorProfile | Any) -> tuple[int, int]:
    profile_id = str(profile.id)
    section_names = {_section_name(section) for section in getattr(profile, "sections", ())}

    if profile_id == "standard_compact_v1":
        return (0, 1)

    correct = 0
    total = 1
    if "pending_choice_features" in section_names:
        correct += 1

    if profile_id == "standard_full_v2":
        total += 1
        if "action_id_features" in section_names:
            correct += 1

    return (correct, total)


def _section_name(section: Any) -> str:
    return str(getattr(section, "name", getattr(section, "id", "")))


def run_profile_games(
    requested_profile: str,
    profile: TensorProfile | Any,
    config: TensorProfileRunConfig,
    clock: Callable[[], float] | None = None,
) -> TensorProfileRunResult:
    now = clock or time.perf_counter
    policy_fn = _policy_fn(config.policy)
    if len(config.seeds) < config.games_per_profile:
        raise ValueError(
            "games_per_profile exceeds available seeds: "
            f"games_per_profile={config.games_per_profile}, seeds={len(config.seeds)}"
        )
    seeds = tuple(config.seeds[: config.games_per_profile])
    start = now()

    games_played = 0
    steps = 0
    wins = 0
    losses = 0
    draws = 0

    for seed in seeds:
        env = DigimonEnv(
            deck1=list(DEFAULT_DECK),
            deck2=list(DEFAULT_DECK),
            tensor_profile=profile.id,
        )
        env.reset(seed=seed)
        terminated = False
        truncated = False
        game_steps = 0

        while not (terminated or truncated) and game_steps < config.max_steps_per_game:
            action = int(policy_fn(env))
            _obs, _reward, terminated, truncated, _info = env.step(action)
            steps += 1
            game_steps += 1

        games_played += 1
        if game_steps >= config.max_steps_per_game and not getattr(env, "is_game_over", False):
            draws += 1
            continue
        winner_id = getattr(env, "winner_id", None)
        if winner_id == 1:
            wins += 1
        elif winner_id == 2:
            losses += 1
        else:
            draws += 1

    elapsed = max(now() - start, 1e-9)
    trigger_order_correct, trigger_order_total = score_trigger_order_accuracy(profile)
    return TensorProfileRunResult(
        requested_profile=requested_profile,
        profile_id=str(profile.id),
        available=True,
        skip_reason="",
        tensor_size=int(profile.tensor_size),
        layout_hash=str(profile.layout_hash),
        feature_schema_version=str(profile.feature_schema_version),
        memory_footprint=estimate_memory_footprint(
            profile,
            n_steps=config.n_steps,
            n_envs=config.n_envs,
        ),
        games_played=games_played,
        steps=steps,
        elapsed_seconds=elapsed,
        wins=wins,
        losses=losses,
        draws=draws,
        trigger_order_correct=trigger_order_correct,
        trigger_order_total=trigger_order_total,
    )


def unavailable_result(resolved: ResolvedProfile, config: TensorProfileRunConfig) -> TensorProfileRunResult:
    return TensorProfileRunResult(
        requested_profile=resolved.requested_profile,
        profile_id="",
        available=False,
        skip_reason=resolved.skip_reason,
        tensor_size=0,
        layout_hash="",
        feature_schema_version="",
        memory_footprint={
            "tensor_bytes": 0,
            "tensor_kib": 0.0,
            "rollout_observation_bytes": 0,
            "rollout_observation_mib": 0.0,
            "card_embedding_input_slots": 0,
            "scalar_input_slots": 0,
        },
        games_played=0,
        steps=0,
        elapsed_seconds=0.0,
        wins=0,
        losses=0,
        draws=0,
        trigger_order_correct=0,
        trigger_order_total=0,
    )


def run_tensor_profile_gauntlet(config: TensorProfileRunConfig) -> TensorProfileGauntletResult:
    results: list[TensorProfileRunResult] = []
    for resolved in resolve_profile_requests(config.profiles, config.require_profiles):
        if not resolved.available or resolved.profile is None:
            results.append(unavailable_result(resolved, config))
            continue
        results.append(
            run_profile_games(
                requested_profile=resolved.requested_profile,
                profile=resolved.profile,
                config=config,
            )
        )
    return TensorProfileGauntletResult(config=config, results=tuple(results))
