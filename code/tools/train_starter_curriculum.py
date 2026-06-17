#!/usr/bin/env python
"""Two-phase starter-deck curriculum: train vs greedy, then swap to the champion pool.

One command that:

  Phase 1 (floor)  — a FRESH MLP trains vs the greedy opponent on the six
                     worldwide starter decks (ST-1..ST-6), profile
                     ``standard_lite_deck_v2``, reward ``starter1_6_flat``, BO3.
  Phase 2 (pool)   — warm-starts (``--init-from``) from phase 1's ``final.zip``
                     and continues vs the FROZEN champion pool, reusing phase 1's
                     deck-pool snapshot so both phases (and the champions) share
                     the identical six-deck curriculum.

Each phase is the *tested* ``pilot_training`` CLI run as a subprocess, so all of
``main()``'s setup — deck validation, generalist deck-pool build + snapshot
write, reward-profile resolution, callbacks, the in-training anchored panel — is
reused verbatim. The "swap after N steps" is the boundary between the two
subprocesses; the warm-start is plain ``--init-from`` (same MLP architecture +
tensor profile across phases, so the checkpoint contract validates).

Run from the repo root that holds ``configs/training/default.yaml``, ``data/``,
and the champion weights (``cloud_downloads/``) — i.e. the BASE repo — with a
release-mode ``digimon_engine`` wheel installed (a debug wheel is ~10x slower).

Pre-flight (once):
    python code/tools/champion_admin.py emit-pool --out pool_starters.json

Launch:
    python code/tools/train_starter_curriculum.py --pool-manifest pool_starters.json

Judge the result with anchored eval (NOT the in-run win rate):
    python code/tools/anchored_eval_cli.py \
        --candidate models/starter_pool_v1/final.zip \
        --deck-pool-snapshot models/starter_floor_v1/deck_pool_snapshot.json --n 100
"""
from __future__ import annotations

import argparse
import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import List

# The six worldwide starter decks, as the canonical archetype keys in
# data/deck_library.json (see openspec battle-test-starter-decks-st1-6).
STARTER_ARCHETYPES: List[str] = [
    "ST-1 Gaia Red",
    "ST-2 Cocytus Blue",
    "ST-3 Heaven's Yellow",
    "ST-4 Giga Green",
    "ST-5 Machine Black",
    "ST-6 Venomous Violet",
]

PILOT_MODULE = "digimon_gym.agents.pilot_training"


@dataclass
class CurriculumSpec:
    """Everything the two phases need, with plan defaults."""

    pool_manifest: str
    save_dir: str = "models"
    log_dir: str = "runs"
    floor_name: str = "starter_floor_v1"
    pool_name: str = "starter_pool_v1"
    floor_steps: int = 1_000_000
    pool_steps: int = 5_000_000
    floor_lr: float = 3e-4
    pool_lr: float = 1e-4
    eval_freq: int = 50_000
    eval_episodes: int = 20
    curriculum_seed: int = 123
    eval_seed: int = 999
    tensor_profile: str = "standard_lite_deck_v2"
    reward_profile: str = "starter1_6_flat"
    match_format: str = "bo3"
    # Record eval games (full action sequences) by default so a low/odd win
    # rate is investigable post-hoc — e.g. confirm/deny concedes (action 93),
    # check suspiciously fast wins/losses. Capped to keep disk bounded.
    record_games: str = "eval"
    record_games_max: int = 300
    # Phase-1-only parallelism. The engine restricts n_envs>1 to greedy/random
    # opponents, so this speeds the floor phase (e.g. set to vCPU count on a
    # cloud box) but CANNOT apply to the pool phase, which stays single-env.
    floor_envs: int = 1
    vec_env_backend: str = "subproc"
    python: str = sys.executable

    # ---- deterministic output locations (pinned via --set run_name) ----
    def floor_final(self) -> Path:
        return Path(self.save_dir) / self.floor_name / "final.zip"

    def floor_snapshot(self) -> Path:
        # train() writes the generalist snapshot to <run_dir>/deck_pool_snapshot.json
        # when neither curriculum_pool nor curriculum_pool_out is set (phase 1).
        return Path(self.save_dir) / self.floor_name / "deck_pool_snapshot.json"

    def pool_final(self) -> Path:
        return Path(self.save_dir) / self.pool_name / "final.zip"


def _common_argv(spec: CurriculumSpec, name: str, steps: int, lr: float) -> List[str]:
    """Flags shared by both phases (everything except opponent/init wiring)."""
    return [
        spec.python, "-m", PILOT_MODULE,
        "--generalist",
        "--timesteps", str(steps),
        "--lr", repr(lr),
        "--match-format", spec.match_format,
        "--eval-freq", str(spec.eval_freq),
        "--eval-episodes", str(spec.eval_episodes),
        "--curriculum-seed", str(spec.curriculum_seed),
        "--eval-seed", str(spec.eval_seed),
        "--save-dir", spec.save_dir,
        "--log-dir", f"{spec.log_dir}/{name}",
        "--record-games", spec.record_games,
        "--record-games-max", str(spec.record_games_max),
        "--set", f"run_name={name}",
        "--set", f"tensor_profile={spec.tensor_profile}",
        "--set", f"reward_profile_override={spec.reward_profile}",
    ]


def build_phase1_argv(spec: CurriculumSpec) -> List[str]:
    """Phase 1: fresh MLP vs greedy, decks = the six starters (resolved fresh)."""
    argv = _common_argv(spec, spec.floor_name, spec.floor_steps, spec.floor_lr)
    argv += [
        "--opponent", "greedy",
        "--archetypes", ",".join(STARTER_ARCHETYPES),
    ]
    if spec.floor_envs > 1:
        # greedy-only parallelism — multiplies floor-phase throughput on CPU.
        argv += [
            "--set", f"n_envs={spec.floor_envs}",
            "--set", f"vec_env_backend={spec.vec_env_backend}",
        ]
    return argv


def build_phase2_argv(spec: CurriculumSpec) -> List[str]:
    """Phase 2: warm-start from phase 1, swap to the frozen champion pool.

    Reuses phase 1's snapshot via --curriculum-pool so the six decks are
    byte-identical across phases (main() ignores --archetypes in this mode).
    """
    argv = _common_argv(spec, spec.pool_name, spec.pool_steps, spec.pool_lr)
    argv += [
        "--opponent", "pool",
        "--opponent-pool-manifest", spec.pool_manifest,
        "--init-from", str(spec.floor_final()),
        "--curriculum-pool", str(spec.floor_snapshot()),
    ]
    return argv


def _run(argv: List[str], cwd: Path) -> None:
    env = dict(os.environ)
    # Ensure the in-tree packages resolve even without an editable install.
    code_dir = str((cwd / "code").resolve())
    env["PYTHONPATH"] = code_dir + os.pathsep + env.get("PYTHONPATH", "")
    print("\n$ " + " ".join(argv), flush=True)
    subprocess.run(argv, cwd=str(cwd), env=env, check=True)


def main() -> None:
    p = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    p.add_argument("--pool-manifest", default="pool_starters.json",
                   help="Champion pool manifest (champion_admin.py emit-pool).")
    p.add_argument("--save-dir", default="models")
    p.add_argument("--log-dir", default="runs")
    p.add_argument("--floor-name", default="starter_floor_v1")
    p.add_argument("--pool-name", default="starter_pool_v1")
    p.add_argument("--floor-steps", type=int, default=1_000_000)
    p.add_argument("--pool-steps", type=int, default=5_000_000)
    p.add_argument("--floor-lr", type=float, default=3e-4)
    p.add_argument("--pool-lr", type=float, default=1e-4)
    p.add_argument("--floor-envs", type=int, default=1,
                   help="Parallel envs for the floor (vs-greedy) phase only; "
                        "set to ~vCPU count on a cloud box. The pool phase is "
                        "single-env regardless (engine limits n_envs>1 to greedy).")
    p.add_argument("--eval-freq", type=int, default=50_000)
    p.add_argument("--eval-episodes", type=int, default=20)
    p.add_argument("--curriculum-seed", type=int, default=123)
    p.add_argument("--eval-seed", type=int, default=999)
    p.add_argument("--tensor-profile", default="standard_lite_deck_v2")
    p.add_argument("--reward-profile", default="starter1_6_flat")
    p.add_argument("--match-format", default="bo3", choices=["bo3", "single"])
    p.add_argument("--record-games", default="eval",
                   help="Which games to persist as recordings (default: eval — "
                        "logs eval games with full action sequences for "
                        "investigation). One of off/all/sampled/draws/anomalies/eval.")
    p.add_argument("--record-games-max", type=int, default=300,
                   help="Cap on recorded-game artifacts (bounds disk).")
    p.add_argument("--python", default=sys.executable,
                   help="Python interpreter for the phase subprocesses.")
    p.add_argument("--cwd", default=None,
                   help="Repo root to run from (default: current directory).")
    p.add_argument("--force-floor", action="store_true",
                   help="Re-run phase 1 even if its final.zip already exists.")
    p.add_argument("--skip-floor", action="store_true",
                   help="Skip phase 1 and go straight to phase 2 "
                        "(phase 1 outputs must already exist).")
    p.add_argument("--floor-only", action="store_true",
                   help="Run ONLY phase 1 (vs greedy), skip the champion phase. "
                        "For floor-isolation / control experiments "
                        "(e.g. --floor-only --floor-envs 1 to test the subproc path).")
    p.add_argument("--dry-run", action="store_true",
                   help="Print both phase commands and exit without training.")
    args = p.parse_args()

    spec = CurriculumSpec(
        pool_manifest=args.pool_manifest,
        save_dir=args.save_dir,
        log_dir=args.log_dir,
        floor_name=args.floor_name,
        pool_name=args.pool_name,
        floor_steps=args.floor_steps,
        pool_steps=args.pool_steps,
        floor_lr=args.floor_lr,
        pool_lr=args.pool_lr,
        eval_freq=args.eval_freq,
        eval_episodes=args.eval_episodes,
        curriculum_seed=args.curriculum_seed,
        eval_seed=args.eval_seed,
        tensor_profile=args.tensor_profile,
        reward_profile=args.reward_profile,
        match_format=args.match_format,
        record_games=args.record_games,
        record_games_max=args.record_games_max,
        floor_envs=args.floor_envs,
        python=args.python,
    )
    cwd = Path(args.cwd).resolve() if args.cwd else Path.cwd()

    phase1 = build_phase1_argv(spec)
    phase2 = build_phase2_argv(spec)

    if args.dry_run:
        print("# Phase 1 — fresh MLP vs greedy "
              f"({spec.floor_steps:,} steps) -> {spec.floor_final()}")
        print(" ".join(phase1))
        if not args.floor_only:
            print("\n# Phase 2 — warm-start + swap to champion pool "
                  f"({spec.pool_steps:,} steps) -> {spec.pool_final()}")
            print(" ".join(phase2))
        return

    if args.floor_only:
        # Floor-isolation / control experiments: train ONLY phase 1, no champions.
        if spec.floor_final().is_file() and not args.force_floor:
            print(f"[floor-only] already trained: {spec.floor_final()}")
        else:
            print(f"[floor-only] fresh MLP vs greedy, {spec.floor_steps:,} steps "
                  "(no champion phase)")
            _run(phase1, cwd)
        print(f"\nDone (floor only). Model: {spec.floor_final()}")
        return

    if not Path(spec.pool_manifest).is_file():
        p.error(
            f"pool manifest not found: {spec.pool_manifest}\n"
            "Emit one first:  python code/tools/champion_admin.py "
            "emit-pool --out pool_starters.json"
        )

    floor_done = spec.floor_final().is_file()
    if args.skip_floor or (floor_done and not args.force_floor):
        why = "skip-floor requested" if args.skip_floor else "already trained"
        print(f"[phase 1] skipped ({why}): {spec.floor_final()}")
        if not floor_done:
            p.error(f"--skip-floor set but phase-1 model is missing: "
                    f"{spec.floor_final()}")
    else:
        print(f"[phase 1] fresh MLP vs greedy, {spec.floor_steps:,} steps")
        _run(phase1, cwd)

    # Hard gate before the (expensive) phase 2: the warm-start + curriculum
    # handoff must exist, or phase 2 would silently start fresh / wrong decks.
    for needed in (spec.floor_final(), spec.floor_snapshot()):
        if not needed.is_file():
            p.error(f"phase-1 output missing, cannot warm-start phase 2: {needed}")

    print(f"\n[phase 2] swap to champion pool, warm-start from "
          f"{spec.floor_final()}, {spec.pool_steps:,} steps")
    _run(phase2, cwd)

    print(f"\nDone. Champion-pool model: {spec.pool_final()}")
    print("Judge it with anchored eval (NOT the in-run win rate):")
    print(f"  python code/tools/anchored_eval_cli.py "
          f"--candidate {spec.pool_final()} "
          f"--deck-pool-snapshot {spec.floor_snapshot()} --n 100")


if __name__ == "__main__":
    main()
