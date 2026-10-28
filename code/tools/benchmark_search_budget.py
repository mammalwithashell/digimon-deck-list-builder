"""Search-budget throughput diagnostics; outputs are NOT a training generation.

Runs the native `selfplay` CLI once per candidate budget policy on the SAME
global game range and master seed, so every candidate starts from identical
deck pairs and shuffles, then reports per-searched-decision cost split by root
phase (Main vs everything else) from the manifest's `phase_stats`, plus
positions/sec and decisions/sec. Candidates change which action is played, so
after the opening the games diverge: the per-decision cost is the primary
metric, total wall time is secondary.

Exactness guards (`--guards`): a candidate whose settings reduce to the
baseline (reduced budget == full budget, full-search fraction 1.0, PUCT root)
must reproduce the baseline's shard arrays byte-for-byte.

See docs/superpowers/research/2026-10-27-search-budget-experiments.md.
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import time
from pathlib import Path

import numpy as np

# name -> {"flags": extra `selfplay` flags, "sims": optional total-budget
# override}. The baseline adds nothing.
CANDIDATES: dict[str, dict] = {
    "B-320": {"flags": []},
    "C1-main320-other64": {"flags": ["--reduced-sims", "64"]},
    "C2-cap0.25-cheap64": {"flags": ["--full-search-fraction", "0.25", "--cheap-sims", "64"]},
    "C3-both": {"flags": ["--reduced-sims", "64", "--full-search-fraction", "0.25", "--cheap-sims", "64"]},
}

# Guard candidates: settings that must be byte-identical to the baseline.
GUARDS: dict[str, dict] = {
    "guard-reduced-equals-full": {"flags": ["--reduced-sims", "320"]},
    "guard-fraction-one": {"flags": ["--full-search-fraction", "1.0", "--cheap-sims", "64"]},
}


def run_case(args, name: str, candidate: dict, env: dict) -> dict:
    directory = args.out / name
    extra = candidate["flags"]
    sims = candidate.get("sims", args.sims)
    command = [args.cli, "--pool", "implemented", "--cards-json", args.cards_json,
               "selfplay", "--model", args.model, "--decks", args.decks,
               "--games", str(args.games), "--game-start", str(args.game_start),
               "--sims", str(sims), "--worlds", str(args.worlds), "--seed", str(args.seed),
               "--eval-batch-size", str(args.batch_size), "--decision-cap", str(args.decision_cap),
               "--temperature-decisions", str(args.temperature_decisions),
               "--out", str(directory)] + extra
    started = time.perf_counter()
    with (args.out / f"{name}.log").open("w", encoding="utf-8") as log:
        subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    wall = time.perf_counter() - started
    manifest = json.loads((directory / "manifest.json").read_text())
    progress = json.loads((directory / "progress.json").read_text())
    elapsed = progress["elapsed_seconds"]
    phases = manifest.get("phase_stats", {})
    main = phases.get("Main", {"searched": 0, "seconds": 0.0, "simulations": 0})
    other = {k: sum(v[k] for p, v in phases.items() if p != "Main") for k in ("searched", "seconds", "simulations")}
    searched = main["searched"] + other["searched"]
    search_seconds = main["seconds"] + other["seconds"]
    case = {
        "name": name, "flags": extra, "total_simulations": sims, "directory": str(directory), "command": command,
        "games": manifest["games"], "truncated_games": manifest["truncated_games"],
        "rows": manifest["rows"], "decisions_total": manifest["decisions_total"],
        "forced_decisions_total": manifest["forced_decisions_total"],
        "full_search_decisions": manifest.get("full_search_decisions"),
        "cheap_search_decisions": manifest.get("cheap_search_decisions"),
        "reduced_budget_decisions": manifest.get("reduced_budget_decisions"),
        "native_seconds": elapsed, "process_wall_seconds": wall,
        "search_seconds": search_seconds,
        "positions_per_second": manifest["rows"] / elapsed if elapsed else None,
        "decisions_per_second": manifest["decisions_total"] / elapsed if elapsed else None,
        "seconds_per_searched_decision": search_seconds / searched if searched else None,
        "main": {**main, "seconds_per_search": main["seconds"] / main["searched"] if main["searched"] else None},
        "other": {**other, "seconds_per_search": other["seconds"] / other["searched"] if other["searched"] else None},
        "non_main_share_of_searches": other["searched"] / searched if searched else None,
        "non_main_share_of_search_seconds": other["seconds"] / search_seconds if search_seconds else None,
        "phase_stats": phases,
        "wins": manifest["wins"],
    }
    return case


def arrays_equal(left: Path, right: Path) -> dict[str, bool]:
    checks = {}
    for shard in sorted(left.glob("shard-*")):
        other = right / shard.name
        for p in sorted(shard.glob("*.npy")):
            q = other / p.name
            checks[f"{shard.name}/{p.name}"] = q.exists() and bool(np.array_equal(np.load(p), np.load(q)))
    return checks


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--cli", required=True)
    parser.add_argument("--model", required=True)
    parser.add_argument("--decks", required=True)
    parser.add_argument("--cards-json", required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--seed", type=int, default=24261027)
    parser.add_argument("--games", type=int, default=8)
    parser.add_argument("--game-start", type=int, default=0)
    parser.add_argument("--sims", type=int, default=320)
    parser.add_argument("--worlds", type=int, default=4)
    parser.add_argument("--batch-size", type=int, default=4)
    parser.add_argument("--decision-cap", type=int, default=1200)
    parser.add_argument("--temperature-decisions", type=int, default=16)
    parser.add_argument("--candidates", nargs="+", default=list(CANDIDATES),
                        help=f"subset of {sorted(CANDIDATES)}")
    parser.add_argument("--guards", action="store_true",
                        help="also run the exactness guards against the baseline (requires B-320)")
    parser.add_argument("--repeats", type=int, default=1,
                        help="repeat the whole candidate list this many times, reversing order each time")
    args = parser.parse_args()
    unknown = [c for c in args.candidates if c not in CANDIDATES]
    if unknown:
        parser.error(f"unknown candidates {unknown}")
    if args.guards and "B-320" not in args.candidates:
        parser.error("--guards needs the B-320 baseline among --candidates")
    if min(args.games, args.sims, args.worlds, args.batch_size, args.decision_cap, args.repeats) < 1:
        parser.error("games, sims, worlds, batch size, decision cap and repeats must be positive")
    args.out.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ, DIGIMON_SEARCH_EVAL_CACHE_ENTRIES="0", DIGIMON_SEARCH_PROFILE="0")
    env.pop("DIGIMON_SELFPLAY_TRACE_DECISIONS", None)
    report = {"diagnostic_only": True, "settings": vars(args) | {"out": str(args.out)},
              "cases": [], "guards": []}
    report_path = args.out / "benchmark.json"

    def save():
        report_path.write_text(json.dumps(report, indent=2), encoding="utf-8")

    for repeat in range(args.repeats):
        order = list(args.candidates) if repeat % 2 == 0 else list(reversed(args.candidates))
        for name in order:
            case = run_case(args, f"{name}-r{repeat}", CANDIDATES[name], env)
            case["candidate"], case["repeat"] = name, repeat
            report["cases"].append(case)
            save()
            print(json.dumps({k: case[k] for k in ("name", "rows", "decisions_total", "native_seconds",
                  "positions_per_second", "seconds_per_searched_decision", "non_main_share_of_searches",
                  "non_main_share_of_search_seconds", "truncated_games")}), flush=True)
    if args.guards:
        baseline_dir = args.out / "B-320-r0"
        for name, candidate in GUARDS.items():
            case = run_case(args, name, candidate, env)
            checks = arrays_equal(baseline_dir, Path(case["directory"]))
            guard = {"name": name, "flags": candidate["flags"], "arrays_equal": checks,
                     "passed": bool(checks) and all(checks.values()), "native_seconds": case["native_seconds"]}
            report["guards"].append(guard)
            save()
            print(json.dumps({k: guard[k] for k in ("name", "passed")}), flush=True)
            if not guard["passed"]:
                raise RuntimeError(f"exactness guard failed: {guard}")
    baseline = [c for c in report["cases"] if c["candidate"] == "B-320"]
    if baseline:
        b = baseline[0]
        for case in report["cases"]:
            if b["seconds_per_searched_decision"] and case["seconds_per_searched_decision"]:
                case["cost_per_searched_decision_vs_baseline"] = (
                    case["seconds_per_searched_decision"] / b["seconds_per_searched_decision"])
            if b["positions_per_second"] and case["positions_per_second"]:
                case["positions_per_second_vs_baseline"] = case["positions_per_second"] / b["positions_per_second"]
        save()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
