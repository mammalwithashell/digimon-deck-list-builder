"""Bounded diagnostics only: profile search or scale a fixed global game set.

No trainer is imported or launched. Deliberately capped outputs are excluded
from training. Re-run on the actual Linux host before sizing an unattended job.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import platform
import subprocess
import time

from tools.run_selfplay_generation import run_selfplay_stage
from tools.selfplay_scheduler import ARRAY_NAMES, atomic_json, sha256


def target_hashes(directories: list[Path]) -> dict:
    result = {}
    for directory in directories:
        manifest = json.loads((directory / "manifest.json").read_text())
        expected = {directory / shard["dir"] / f"{name}.npy"
                    for shard in manifest["shards"] for name in ARRAY_NAMES}
        if set(directory.glob("shard-*/*.npy")) != expected:
            raise ValueError(f"Missing or unexpected target arrays: {directory}")
        result.update({f"{directory.name}/{file.relative_to(directory).as_posix()}": sha256(file)
                       for file in expected})
    return result


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["profile", "scale"])
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--model", type=Path, required=True)
    parser.add_argument("--decks", type=Path, required=True)
    parser.add_argument("--cards-json", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--workers", nargs="+", type=int, default=[1, 2, 4])
    parser.add_argument("--games", type=int, default=8)
    parser.add_argument("--chunk-games", type=int, default=1)
    parser.add_argument("--repeats", type=int, default=1)
    parser.add_argument("--seed", type=int, default=21261025)
    parser.add_argument("--decision-cap", type=int, default=40)
    parser.add_argument("--sims", type=int, default=320)
    parser.add_argument("--worlds", type=int, default=4)
    parser.add_argument("--batch-size", type=int, default=4)
    parser.add_argument("--worker-timeout-seconds", type=float, default=3600)
    args = parser.parse_args(argv)
    if min(args.games, args.chunk_games, args.repeats, args.decision_cap,
           args.sims, args.worlds, args.batch_size, *args.workers) < 1:
        parser.error("counts, search settings and decision cap must be positive")
    args.out.mkdir(parents=True, exist_ok=False)
    quota = Path("/sys/fs/cgroup/cpu.max")
    report = {"diagnostic_only": True, "platform": platform.platform(),
              "logical_cpus": os.cpu_count(),
              "affinity_cpus": sorted(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else None,
              "cgroup_v2_cpu_max": quota.read_text().strip() if quota.exists() else None,
              "settings": {key: str(value) if isinstance(value, Path) else value for key, value in vars(args).items()},
              "cases": [], "all_target_arrays_equal": True}
    path = args.out / "benchmark.json"
    atomic_json(path, report)
    baseline = None
    # A prefix parity reference makes profiling overhead visible as well.
    cases = [("profile-off", False), ("profile-on", True)] if args.mode == "profile" else [
        (f"repeat-{repeat}-workers-{workers}", workers)
        for repeat in range(args.repeats)
        for workers in (args.workers if repeat % 2 == 0 else list(reversed(args.workers)))]
    old_profile = os.environ.get("DIGIMON_SEARCH_PROFILE")
    old_cache = os.environ.get("DIGIMON_SEARCH_EVAL_CACHE_ENTRIES")
    os.environ["DIGIMON_SEARCH_EVAL_CACHE_ENTRIES"] = "0"
    try:
        for name, value in cases:
            directory = args.out / name
            started = time.perf_counter()
            os.environ["DIGIMON_SEARCH_PROFILE"] = "1" if args.mode == "profile" and value else "0"
            if args.mode == "profile":
                directory.mkdir()
                command = [str(args.cli), "--pool", "implemented", "--cards-json", str(args.cards_json),
                    "selfplay", "--model", str(args.model), "--decks", str(args.decks),
                    "--games", str(args.games), "--sims", str(args.sims), "--worlds", str(args.worlds),
                    "--eval-batch-size", str(args.batch_size), "--decision-cap", str(args.decision_cap),
                    "--seed", str(args.seed), "--out", str(directory / "native")]
                with (directory / "native.log").open("w", encoding="utf-8") as log:
                    subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True,
                                   timeout=args.worker_timeout_seconds)
                directories = [directory / "native"]
                workers = 1
            else:
                workers = value
                directories = run_selfplay_stage(str(args.cli), directory, str(args.model), False,
                    args.decks, args.games, workers, args.sims, args.worlds, "pimc", args.seed,
                    16, args.decision_cap, 4096, args.cards_json, "implemented",
                    eval_batch_size=args.batch_size, chunk_games=args.chunk_games,
                    worker_timeout_seconds=args.worker_timeout_seconds)
            elapsed = time.perf_counter() - started
            manifests = [json.loads((p / "manifest.json").read_text()) for p in directories]
            hashes = target_hashes(directories)
            if baseline is None:
                baseline = hashes
            equal = hashes == baseline and bool(hashes)
            rows = sum(m["rows"] for m in manifests)
            native_times = [json.loads((p / "progress.json").read_text())["elapsed_seconds"] for p in directories]
            case = {"name": name, "workers": workers, "wall_seconds": elapsed,
                    "rows": rows, "decisions": sum(m["decisions_total"] for m in manifests),
                    "truncated_games": sum(m["truncated_games"] for m in manifests),
                    "positions_generated_per_second": rows / elapsed,
                    "budgeted_simulations_per_second": rows * args.sims / elapsed,
                    "native_chunk_seconds": native_times,
                    "native_work_utilization": sum(native_times) / (workers * elapsed),
                    "profiling": [m.get("profiling") for m in manifests] if args.mode == "profile" else None,
                    "all_target_arrays_equal": equal}
            report["cases"].append(case)
            report["all_target_arrays_equal"] &= equal
            atomic_json(path, report)
            print(json.dumps(case), flush=True)
            if not equal:
                raise RuntimeError("Fixed game set changed with profiling or worker count")
    finally:
        for key, old in (("DIGIMON_SEARCH_PROFILE", old_profile), ("DIGIMON_SEARCH_EVAL_CACHE_ENTRIES", old_cache)):
            if old is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = old
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
