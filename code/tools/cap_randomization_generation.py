"""Playout-cap randomization generation comparison (search-budget experiments, 2026-10-27).

Question: does self-play data generated with KataGo-style playout-cap
randomization (``--full-search-fraction 0.25 --cheap-sims 64``; cheap rows are
value-only) train a checkpoint as strong as full-search data — per game, and per
unit of search compute? The 16-root panel and the head-to-head match could not
answer this because play-time search is unchanged; only a training generation can.

Arms (same master seed, so game ``k`` draws the same deck pair and shuffle in
every arm):

- ``full``  : 320 sims / 4 worlds on every non-forced decision, ``games`` games.
- ``cap``   : cap randomization, the same ``games`` game indices (equal games,
              ~0.36x the search compute, ~25% policy-target rows).
- ``cap_eq``: cap randomization over ``games / 0.36`` game indices — the
              ``cap`` partitions plus extra ones (equal search compute).

Stages (re-runnable; each writes under ``--out``):

    generate  spawn partitioned ``digimon-engine-cli selfplay`` jobs (``--jobs`` at once)
    train     one ``azero_trainer`` update per arm from the same parent .zip + ONNX export
    evaluate  seat-balanced policy-only head-to-head per child vs parent, vs greedy,
              and cap_eq vs full (``policy_head_to_head.py``), ``--jobs`` at once
    report    collect everything into ``report.json``

Example:
    python code/tools/cap_randomization_generation.py --out D:/wt/bench-runs/2026-10-27/cap-generation \
        --cli D:/cargo-target/search-bench/release/digimon-engine-cli.exe \
        --model D:/.../model.onnx --parent-zip D:/.../parent.zip \
        --decks D:/.../decks.json --cards-json D:/wt/search-bench/data/cards.json \
        --games 1000 --partition 250 --jobs 6 generate train evaluate report
"""

from __future__ import annotations

import argparse
import json
import math
import os
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
TOOLS = REPO_ROOT / "code" / "tools"

CAP_FLAGS = ["--full-search-fraction", "0.25", "--cheap-sims", "64"]


def partitions(start: int, games: int, size: int) -> list[tuple[int, int]]:
    return [(g, min(size, start + games - g)) for g in range(start, start + games, size)]


def plan(args) -> dict:
    cap_eq_games = int(math.ceil(args.games / args.cap_cost_ratio / args.partition)) * args.partition
    arms = {
        "full": dict(flags=[], partitions=partitions(0, args.games, args.partition)),
        "cap": dict(flags=CAP_FLAGS, partitions=partitions(0, args.games, args.partition)),
        "cap_eq": dict(flags=CAP_FLAGS, partitions=partitions(0, cap_eq_games, args.partition)),
    }
    # cap_eq shares cap's data for the first `games` indices; generate only the tail.
    arms["cap_eq"]["generate_partitions"] = partitions(args.games, cap_eq_games - args.games, args.partition)
    arms["cap"]["generate_partitions"] = arms["cap"]["partitions"]
    arms["full"]["generate_partitions"] = arms["full"]["partitions"]
    return dict(games=args.games, cap_eq_games=cap_eq_games, seed=args.seed, sims=args.sims,
                worlds=args.worlds, eval_batch_size=args.eval_batch_size, decision_cap=args.decision_cap,
                temperature_decisions=args.temperature_decisions, cap_flags=CAP_FLAGS, arms=arms)


def shard_dirs(args, arm: str, p: dict) -> list[Path]:
    source_arm = "cap" if arm == "cap_eq" else arm
    dirs = []
    for start, count in p["arms"][arm]["partitions"]:
        owner = source_arm if start < args.games else "cap_eq"
        dirs.append(args.out / "shards" / owner / f"g{start:05d}-{count}")
    return dirs


def selfplay_job(args, arm: str, start: int, count: int, flags: list[str]) -> dict:
    directory = args.out / "shards" / arm / f"g{start:05d}-{count}"
    manifest = directory / "manifest.json"
    if manifest.exists():
        return dict(arm=arm, start=start, count=count, directory=str(directory), skipped=True)
    directory.parent.mkdir(parents=True, exist_ok=True)
    command = [str(args.cli), "--pool", "implemented", "--cards-json", str(args.cards_json), "selfplay",
               "--model", str(args.model), "--decks", str(args.decks), "--games", str(count),
               "--game-start", str(start), "--sims", str(args.sims), "--worlds", str(args.worlds),
               "--seed", str(args.seed), "--eval-batch-size", str(args.eval_batch_size),
               "--decision-cap", str(args.decision_cap), "--temperature-decisions",
               str(args.temperature_decisions), "--out", str(directory)] + flags
    started = time.perf_counter()
    with (directory.parent / f"g{start:05d}-{count}.log").open("w", encoding="utf-8") as log:
        result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, env=os.environ.copy())
    record = dict(arm=arm, start=start, count=count, directory=str(directory), command=command,
                  returncode=result.returncode, wall_seconds=time.perf_counter() - started)
    if result.returncode == 0 and manifest.exists():
        m = json.loads(manifest.read_text(encoding="utf-8"))
        record.update(rows=m["rows"], decisions_total=m["decisions_total"], truncated_games=m["truncated_games"],
                      full_search_decisions=m.get("full_search_decisions"),
                      cheap_search_decisions=m.get("cheap_search_decisions"),
                      search_seconds=sum(v["seconds"] for v in m.get("phase_stats", {}).values()),
                      native_seconds=json.loads((directory / "progress.json").read_text())["elapsed_seconds"])
    print(json.dumps({k: v for k, v in record.items() if k != "command"}), flush=True)
    return record


def stage_generate(args, p: dict) -> list[dict]:
    jobs = []
    for arm in ("full", "cap", "cap_eq"):
        for start, count in p["arms"][arm]["generate_partitions"]:
            jobs.append((arm, start, count, p["arms"][arm]["flags"]))
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        records = list(pool.map(lambda j: selfplay_job(args, *j), jobs))
    failed = [r for r in records if r.get("returncode", 0) != 0]
    (args.out / "generate.json").write_text(json.dumps(records, indent=2), encoding="utf-8")
    if failed:
        raise SystemExit(f"{len(failed)} selfplay partition(s) failed; see generate.json")
    return records


def stage_train(args, p: dict) -> dict:
    out = {}
    for arm in ("full", "cap", "cap_eq"):
        model_dir = args.out / "models" / arm
        model_dir.mkdir(parents=True, exist_ok=True)
        zip_path, onnx_path = model_dir / "model.zip", model_dir / "model.onnx"
        if zip_path.exists() and onnx_path.exists():
            out[arm] = dict(zip=str(zip_path), onnx=str(onnx_path), skipped=True)
            continue
        dirs = shard_dirs(args, arm, p)
        missing = [d for d in dirs if not (d / "manifest.json").exists()]
        if missing:
            raise SystemExit(f"{arm}: missing shard dirs {missing}")
        command = [sys.executable, "-m", "digimon_gym.agents.azero_trainer", "--init-from", str(args.parent_zip),
                   "--shards", *map(str, dirs), "--out", str(zip_path), "--epochs", str(args.epochs),
                   "--batch-size", str(args.batch_size), "--lr", str(args.lr), "--seed", str(args.seed),
                   "--export-onnx", str(onnx_path)]
        env = dict(os.environ, PYTHONPATH=os.pathsep.join(filter(None, [str(REPO_ROOT / "code"), os.environ.get("PYTHONPATH", "")])))
        started = time.perf_counter()
        with (model_dir / "train.log").open("w", encoding="utf-8") as log:
            result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, env=env, cwd=str(REPO_ROOT))
        if result.returncode != 0:
            raise SystemExit(f"{arm}: trainer failed, see {model_dir / 'train.log'}")
        out[arm] = dict(zip=str(zip_path), onnx=str(onnx_path), shards=[str(d) for d in dirs],
                        wall_seconds=time.perf_counter() - started, command=command)
        print(json.dumps({k: v for k, v in out[arm].items() if k != "command"}), flush=True)
    (args.out / "train.json").write_text(json.dumps(out, indent=2), encoding="utf-8")
    return out


def h2h_job(args, name: str, candidate: Path, opponent: str) -> dict:
    result_path = args.out / "eval" / f"{name}.json"
    if result_path.exists():
        return json.loads(result_path.read_text(encoding="utf-8"))
    result_path.parent.mkdir(parents=True, exist_ok=True)
    command = [sys.executable, str(TOOLS / "policy_head_to_head.py"), "--candidate", str(candidate),
               "--opponent", opponent, "--decks", str(args.decks), "--out", str(result_path),
               "--blocks-per-pair", str(args.blocks_per_pair), "--seed", str(args.seed),
               "--on-engine-error", args.on_engine_error]
    with (args.out / "eval" / f"{name}.log").open("w", encoding="utf-8") as log:
        result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, env=os.environ.copy(), cwd=str(REPO_ROOT))
    if result.returncode != 0:
        raise SystemExit(f"evaluation {name} failed, see {args.out / 'eval' / f'{name}.log'}")
    return json.loads(result_path.read_text(encoding="utf-8"))


def stage_evaluate(args) -> dict:
    models = {arm: args.out / "models" / arm / "model.zip" for arm in ("full", "cap", "cap_eq")}
    jobs = {f"{arm}-vs-parent": (models[arm], str(args.parent_zip)) for arm in models}
    jobs.update({f"{arm}-vs-greedy": (models[arm], "greedy") for arm in models})
    jobs["parent-vs-greedy"] = (args.parent_zip, "greedy")
    jobs["cap_eq-vs-full"] = (models["cap_eq"], str(models["full"]))
    jobs["cap-vs-full"] = (models["cap"], str(models["full"]))
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        results = dict(zip(jobs, pool.map(lambda kv: h2h_job(args, kv[0], *kv[1]), jobs.items())))
    summary = {name: {k: r.get(k, 0) for k in ("games", "wins", "losses", "draws", "incomplete", "score", "score_ci95")}
               for name, r in results.items()}
    (args.out / "evaluate.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2), flush=True)
    return summary


def stage_report(args, p: dict) -> dict:
    report = dict(plan=p, generated=json.loads((args.out / "generate.json").read_text(encoding="utf-8")),
                  trained=json.loads((args.out / "train.json").read_text(encoding="utf-8")),
                  evaluated=json.loads((args.out / "evaluate.json").read_text(encoding="utf-8")))
    per_arm = {}
    for arm in ("full", "cap", "cap_eq"):
        recs = [r for r in report["generated"] if Path(r["directory"]) in set(shard_dirs(args, arm, p))]
        per_arm[arm] = dict(
            games=sum(r["count"] for r in recs), rows=sum(r.get("rows", 0) for r in recs),
            search_seconds=sum(r.get("search_seconds", 0.0) for r in recs),
            native_seconds=sum(r.get("native_seconds", 0.0) for r in recs),
            policy_rows=sum((r.get("full_search_decisions") or r.get("rows", 0)) for r in recs),
            truncated_games=sum(r.get("truncated_games", 0) for r in recs))
    report["per_arm"] = per_arm
    (args.out / "report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(per_arm, indent=2), flush=True)
    return report


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    ap.add_argument("stages", nargs="+", choices=["plan", "generate", "train", "evaluate", "report"])
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--cli", type=Path, required=True)
    ap.add_argument("--model", type=Path, required=True, help="parent policy+value ONNX driving the search")
    ap.add_argument("--parent-zip", type=Path, required=True, help="same parent as SB3 .zip for the trainer")
    ap.add_argument("--decks", type=Path, required=True)
    ap.add_argument("--cards-json", type=Path, required=True)
    ap.add_argument("--games", type=int, default=1000)
    ap.add_argument("--partition", type=int, default=250)
    ap.add_argument("--cap-cost-ratio", type=float, default=0.36, help="measured cap/full seconds per decision")
    ap.add_argument("--jobs", type=int, default=6)
    ap.add_argument("--seed", type=int, default=27102026)
    ap.add_argument("--sims", type=int, default=320)
    ap.add_argument("--worlds", type=int, default=4)
    ap.add_argument("--eval-batch-size", type=int, default=4)
    ap.add_argument("--decision-cap", type=int, default=1200)
    ap.add_argument("--temperature-decisions", type=int, default=16)
    ap.add_argument("--epochs", type=int, default=2)
    ap.add_argument("--batch-size", type=int, default=512)
    ap.add_argument("--lr", type=float, default=1e-4)
    ap.add_argument("--blocks-per-pair", type=int, default=1)
    ap.add_argument("--on-engine-error", choices=["fail", "skip"], default="skip",
                    help="head-to-head games an engine fault aborts are recorded as incomplete, never scored")
    args = ap.parse_args(argv)
    args.out.mkdir(parents=True, exist_ok=True)
    p = plan(args)
    (args.out / "plan.json").write_text(json.dumps(p, indent=2), encoding="utf-8")
    for stage in args.stages:
        if stage == "plan":
            print(json.dumps({k: v for k, v in p.items() if k != "arms"} | {
                arm: dict(generate=a["generate_partitions"], train=a["partitions"]) for arm, a in p["arms"].items()}, indent=2))
        elif stage == "generate":
            stage_generate(args, p)
        elif stage == "train":
            stage_train(args, p)
        elif stage == "evaluate":
            stage_evaluate(args)
        elif stage == "report":
            stage_report(args, p)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
