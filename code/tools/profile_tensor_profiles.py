from __future__ import annotations

import argparse
import sys
from pathlib import Path

CODE_ROOT = Path(__file__).resolve().parents[1]
if str(CODE_ROOT) not in sys.path:
    sys.path.insert(0, str(CODE_ROOT))

from digimon_gym.agents.tensor_profile_gauntlet import (
    DEFAULT_PROFILE_REQUESTS,
    TensorProfileRunConfig,
    run_tensor_profile_gauntlet,
)


def positive_int(raw: str) -> int:
    try:
        value = int(raw)
    except ValueError as exc:
        raise argparse.ArgumentTypeError(f"{raw!r} is not a valid positive integer") from exc
    if value <= 0:
        raise argparse.ArgumentTypeError(f"{raw!r} must be a positive integer")
    return value


def parse_seed_range(raw: str) -> tuple[int, ...]:
    if not raw.strip():
        raise ValueError("seed range must not be empty")
    if ":" in raw:
        parts = raw.split(":")
        if len(parts) != 2:
            raise ValueError("seed range must use start:stop")
        start_raw, stop_raw = parts
        try:
            start = int(start_raw)
            stop = int(stop_raw)
        except ValueError as exc:
            raise ValueError("seed range bounds must be integers") from exc
        if stop <= start:
            raise ValueError("seed range stop must be greater than start")
        return tuple(range(start, stop))
    try:
        seeds = tuple(int(part.strip()) for part in raw.split(",") if part.strip())
    except ValueError as exc:
        raise ValueError("seed list entries must be integers") from exc
    if not seeds:
        raise ValueError("seed list must contain at least one seed")
    return seeds


def parse_args(argv: list[str] | None = None):
    parser = argparse.ArgumentParser(
        description="Compare board-state tensor profiles with fixed-seed RL gauntlet metrics."
    )
    parser.add_argument(
        "--profiles",
        default=",".join(DEFAULT_PROFILE_REQUESTS),
        help="Comma-separated tensor profile IDs to compare.",
    )
    parser.add_argument("--games", type=positive_int, default=25, help="Games per profile.")
    parser.add_argument(
        "--seeds",
        default="101:126",
        help="Seed range start:stop or comma-separated seed list.",
    )
    parser.add_argument(
        "--max-steps-per-game",
        type=positive_int,
        default=1000,
        help="Step cap per game; capped games count as draws.",
    )
    parser.add_argument(
        "--policy",
        choices=["greedy", "random"],
        default="greedy",
        help="Policy used for player 1 during benchmark games.",
    )
    parser.add_argument(
        "--n-steps",
        type=positive_int,
        default=128,
        help="Rollout step count used for memory footprint estimates.",
    )
    parser.add_argument(
        "--n-envs",
        type=positive_int,
        default=1,
        help="Vectorized env count used for memory footprint estimates.",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=Path("profile_runs") / "tensor_profiles" / "latest",
        help="Directory where result.json and result.md are written.",
    )
    parser.add_argument(
        "--require-profiles",
        action="store_true",
        help="Fail if any requested profile is unavailable.",
    )
    args = parser.parse_args(argv)
    try:
        parse_seed_range(args.seeds)
    except ValueError as exc:
        parser.error(str(exc))
    return args


def config_from_args(args) -> TensorProfileRunConfig:
    return TensorProfileRunConfig(
        profiles=tuple(part.strip() for part in args.profiles.split(",") if part.strip()),
        games_per_profile=args.games,
        seeds=parse_seed_range(args.seeds),
        max_steps_per_game=args.max_steps_per_game,
        policy=args.policy,
        require_profiles=args.require_profiles,
        n_steps=args.n_steps,
        n_envs=args.n_envs,
    )


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    result = run_tensor_profile_gauntlet(config_from_args(args))
    args.out.mkdir(parents=True, exist_ok=True)
    json_path = args.out / "result.json"
    markdown_path = args.out / "result.md"
    result.write_json(json_path)
    result.write_markdown(markdown_path)
    print(result.to_markdown())
    print(f"Wrote {json_path}")
    print(f"Wrote {markdown_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
