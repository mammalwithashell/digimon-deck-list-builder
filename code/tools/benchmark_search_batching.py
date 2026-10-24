"""Paired native-search diagnostics; outputs are NOT a training generation.

Compare batch sizes on identical seeded decision sequences, checking every
saved target array exactly. Games can be deliberately capped; report generated
positions/sec, never label capped rows as usable terminal-outcome data.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import subprocess
import time

import numpy as np


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", required=True)
    parser.add_argument("--model", required=True)
    parser.add_argument("--decks", required=True)
    parser.add_argument("--cards-json", required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--seeds", type=int, nargs="+", default=[21261025, 21261023])
    parser.add_argument("--repeats", type=int, default=1)
    parser.add_argument("--batch-size", type=int, default=4)
    parser.add_argument("--sims", type=int, default=320)
    parser.add_argument("--worlds", type=int, default=4)
    parser.add_argument("--decision-cap", type=int, default=40)
    parser.add_argument("--cache-entries", type=int, default=0)
    args = parser.parse_args()
    if min(args.repeats, args.batch_size, args.worlds, args.decision_cap) < 1:
        parser.error("repeat, batch, world and decision-cap values must be positive")
    args.out.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ, DIGIMON_SEARCH_EVAL_CACHE_ENTRIES=str(args.cache_entries))
    env.pop("DIGIMON_SELFPLAY_TRACE_DECISIONS", None)
    report = {"diagnostic_only": True, "settings": vars(args) | {"out": str(args.out)},
        "cases": [], "comparisons": []}
    report_path = args.out / "benchmark.json"
    for seed in args.seeds:
        for repeat in range(args.repeats):
            pair = {}
            sizes = [1, args.batch_size] if repeat % 2 == 0 else [args.batch_size, 1]
            for size in dict.fromkeys(sizes):
                name = f"seed-{seed}-repeat-{repeat}-batch-{size}"
                directory = args.out / name
                command = [args.cli, "--pool", "implemented", "--cards-json", args.cards_json,
                    "selfplay", "--model", args.model, "--decks", args.decks, "--games", "1",
                    "--sims", str(args.sims), "--worlds", str(args.worlds), "--seed", str(seed),
                    "--eval-batch-size", str(size), "--decision-cap", str(args.decision_cap),
                    "--out", str(directory)]
                started = time.perf_counter()
                with (args.out / f"{name}.log").open("w", encoding="utf-8") as log:
                    subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
                wall = time.perf_counter() - started
                manifest = json.loads((directory / "manifest.json").read_text())
                progress = json.loads((directory / "progress.json").read_text())
                elapsed = progress["elapsed_seconds"]
                case = {"seed": seed, "repeat": repeat, "batch_size": size, "directory": str(directory),
                    "rows": manifest["rows"], "truncated_games": manifest["truncated_games"],
                    "native_seconds": elapsed, "process_wall_seconds": wall,
                    "positions_generated_per_second": manifest["rows"] / elapsed}
                pair[size] = case
                report["cases"].append(case)
                report_path.write_text(json.dumps(report, indent=2), encoding="utf-8")
                print(json.dumps(case), flush=True)
            baseline, batched = pair[1], pair[args.batch_size]
            left = Path(baseline["directory"]) / "shard-000"
            right = Path(batched["directory"]) / "shard-000"
            checks = {p.name: bool(np.array_equal(np.load(p), np.load(right / p.name)))
                for p in left.glob("*.npy")}
            comparison = {"seed": seed, "repeat": repeat, "arrays_equal": checks,
                "throughput_ratio": baseline["native_seconds"] / batched["native_seconds"]}
            report["comparisons"].append(comparison)
            report_path.write_text(json.dumps(report, indent=2), encoding="utf-8")
            if not checks or not all(checks.values()):
                raise RuntimeError(f"Serial/batched target parity failed: {comparison}")
            print(json.dumps(comparison), flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
