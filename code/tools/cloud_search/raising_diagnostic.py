"""Paired frozen-position diagnostic; never fits or changes a training run."""
from __future__ import annotations

import argparse
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor, as_completed
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

import numpy as np


def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def write(path, data):
    path = Path(path)
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    temporary.replace(path)


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def prepare(root, engine):
    inputs = root / "inputs"
    snapshot = read(inputs / "combined_diagnostic_snapshot.json")
    decks = [dict(name=d["archetype"], cards=d["card_ids"]) for d in snapshot["decks"]]
    models = [dict(name=f"checkpoint{cp}", path=str((inputs / f"cp{cp}.onnx").resolve())) for cp in (2, 4, 5, 6)]
    repo = Path(__file__).resolve().parents[3]
    assets = [engine, repo / "data/cards.json", inputs / "combined_diagnostic_snapshot.json"]
    assets += [Path(m["path"]) for m in models]
    assets += [Path(m["path"] + ".meta.json") for m in models]
    plan = dict(schema="raising-counterfactual-v1", diagnostic_only=True,
        engine=str(engine.resolve()), cards_json=str(repo / "data/cards.json"),
        models=models, actors=[0, 2], opponents=[0, 1, 3], collection_opponent=1,
        decks=decks, decision_cap=1200, simulations=320, worlds=4, batch=4,
        exploration_noise=0, seed=28261028,
        position_sampling="One reservoir-sampled legal promote/wait position per completed source game; four games per archetype, two per actor model, both seats and available list variants.",
        continuation_mode="Forced promote or wait, then deterministic policy-only choices; same cloned state and RNG, paired by position/model/opponent.",
        limitations=["Exploratory fixed-pool screen; four roots per archetype do not establish per-deck strength.",
            "Search is exact-composition open-decklist PIMC, not a closed-decklist test.",
            "Policy continuation outcomes alone cannot prove a search/value defect: a searched choice may require searched follow-up play.",
            "No incomplete continuation counts as a draw; source games without a completed result are not sampled.",
            "One actual hidden-state realization per root; 4-world root search does not make the continuation four independent games."],
        source_commit=subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip(),
        engine_patch_sha256=hashlib.sha256(subprocess.check_output(["git", "diff", "--", "code/digimon-engine/src"], cwd=repo)).hexdigest(),
        hashes={str(p.resolve()): digest(p) for p in assets},
        driver_sha256=digest(__file__), native_driver_sha256=digest(repo / "code/digimon-engine/examples/raising_diagnostic.rs"))
    target = root / "plan.json"
    if target.exists():
        raise FileExistsError(target)
    write(target, plan)
    return plan


def requests(root, plan, stage):
    if stage == "collect":
        groups = defaultdict(list)
        for i, deck in enumerate(plan["decks"]):
            groups[deck["name"]].append(i)
        names = sorted(groups)
        for n, name in enumerate(names):
            for repetition in range(2):
                for source_index, model in enumerate(plan["actors"]):
                    actor = (repetition + source_index) % 2
                    own = groups[name][repetition % len(groups[name])]
                    opponent_name = names[(n + 5 + repetition) % len(names)]
                    opposing = groups[opponent_name][repetition % len(groups[opponent_name])]
                    key = f"{n:02d}-{repetition}-{model}"
                    yield key, dict(id=key, plan=str(root / "plan.json"), actor=actor,
                        source_model=model, archetype=name,
                        deck_indices=[own, opposing] if actor == 0 else [opposing, own],
                        game_seed=plan["seed"] + n * 1000 + repetition * 10 + source_index,
                        sample_seed=plan["seed"] + 100000 + n * 100 + repetition * 2 + source_index)
    elif stage == "probe":
        collected = sorted(p for p in (root / "collect").glob("*.json")
            if not p.name.endswith(".failure.json"))
        for index, path in enumerate(collected):
            result = read(path)
            if result["status"] == "complete" and result.get("node") is not None:
                yield path.stem, dict(plan=str(root / "plan.json"), node=result["node"],
                    search_seed=plan["seed"] + 1000000 + index)


def run_stage(root, stage, workers, limit):
    plan = read(root / "plan.json")
    if (plan["simulations"], plan["worlds"], plan["batch"], plan["exploration_noise"]) != (320, 4, 4, 0):
        raise ValueError("Native diagnostic is fixed at 320 simulations, four worlds, batch four, no noise")
    for path, expected in plan["hashes"].items():
        if digest(path) != expected:
            raise ValueError(f"Frozen asset changed: {path}")
    jobs = list(requests(root, plan, stage))
    if limit:
        jobs = jobs[:limit]
    destination = root / stage
    destination.mkdir(exist_ok=True)
    request_dir = root / "requests"
    request_dir.mkdir(exist_ok=True)
    started = time.time()

    def worker(item):
        key, request = item
        target = destination / f"{key}.json"
        request_path = request_dir / f"{stage}-{key}.json"
        if request_path.exists() and read(request_path) != request:
            raise ValueError(f"Request changed: {key}")
        if target.with_suffix(".failure.json").exists():
            return key, "failed"
        if target.exists():
            return key, "complete" if read(target)["status"] == "complete" else "incomplete"
        write(request_path, request)
        begin = time.time()
        try:
            with target.with_suffix(".log").open("w", encoding="utf-8") as log:
                subprocess.run([plan["engine"], stage, str(request_path), str(target)],
                    stdout=log, stderr=subprocess.STDOUT, check=True, timeout=3600,
                    env={**os.environ, "OMP_NUM_THREADS": "1", "MKL_NUM_THREADS": "1",
                         "OPENBLAS_NUM_THREADS": "1", "RUST_MIN_STACK": "268435456"})
            return key, read(target)["status"]
        except (subprocess.SubprocessError, OSError) as error:
            failure = dict(status="failed", error=str(error), seconds=time.time()-begin,
                partial_result=str(target) if target.exists() else None)
            write(target.with_suffix(".failure.json"), failure)
            return key, "failed"

    statuses = []
    with ThreadPoolExecutor(max_workers=workers) as executor:
        futures = [executor.submit(worker, item) for item in jobs]
        for future in as_completed(futures):
            key, status = future.result()
            statuses.append(status)
            progress = dict(stage=stage, finished=len(statuses), total=len(jobs),
                complete=statuses.count("complete"), failed=statuses.count("failed"),
                incomplete=statuses.count("incomplete"), elapsed=time.time()-started,
                last=key, heartbeat=time.time())
            write(root / f"{stage}-progress.json", progress)
            print(json.dumps(progress), flush=True)
    if any(status != "complete" for status in statuses):
        raise RuntimeError("Diagnostic has failed or incomplete jobs; inspect preserved outputs before explicit recovery")
    return statuses


def paired_branches(result):
    """Only complete action pairs enter counterfactual strength summaries."""
    groups = defaultdict(dict)
    for branch in result["branches"]:
        key = (branch["model"], branch["opponent"])
        action = branch["first_action"]
        if action in groups[key]:
            raise ValueError("Duplicate continuation is not an independent sample")
        groups[key][action] = branch["outcome"]
    return {key: {a: group[a]["score"] for a in (61, 62)} for key, group in groups.items()
        if set(group) == {61, 62} and all(v["status"] == "complete" for v in group.values())}


def interval(rows, seed=28261028):
    """Stratified paired bootstrap, one independent source game per root."""
    if not rows:
        return dict(n=0, mean=None, ci95=None)
    groups = defaultdict(list)
    for archetype, value in rows:
        groups[archetype].append(value)
    rng = np.random.default_rng(seed)
    values = np.concatenate([np.asarray(v) for v in groups.values()])
    if any(len(v) < 2 for v in groups.values()):
        return dict(n=len(rows), mean=float(values.mean()), ci95=None,
            ci_note="Not estimated: at least one archetype has only one sampled root.")
    draws = sum(rng.choice(v, (4000, len(v)), replace=True).sum(axis=1)
                for v in groups.values()) / len(values)
    return dict(n=len(rows), mean=float(values.mean()),
        ci95=list(map(float, np.quantile(draws, [.025, .975]))))


def summarize(root):
    plan = read(root / "plan.json")
    collected = [read(p) for p in sorted((root / "collect").glob("*.json")) if not p.name.endswith(".failure.json")]
    results = [read(p) for p in sorted((root / "probe").glob("*.json")) if not p.name.endswith(".failure.json")]
    complete = [r for r in results if r["status"] == "complete"]
    rows, root_stats, cases = defaultdict(list), {}, []
    for model in plan["actors"]:
        chosen = [next(x for x in r["roots"] if x["model"] == model) for r in complete]
        root_stats[model] = dict(n=len(chosen), policy_promote=sum(x["policy_action"] == 61 for x in chosen),
            search_promote=sum(x["search_plan"][0] == 61 for x in chosen),
            policy_wait_search_promote=sum(x["policy_action"] == 62 and x["search_plan"][0] == 61 for x in chosen),
            policy_promote_search_wait=sum(x["policy_action"] == 61 and x["search_plan"][0] == 62 for x in chosen),
            mean_promote_probability=float(np.mean([x["promote_probability"] for x in chosen])) if chosen else None)
    for result in complete:
        pairs = paired_branches(result)
        name = result["node"]["archetype"]
        for prediction in result["roots"]:
            model = prediction["model"]
            for opponent in plan["opponents"]:
                pair = pairs.get((model, opponent))
                if pair is None:
                    continue
                base = f"model{model}/opponent{opponent}"
                delta = pair[61] - pair[62]
                for measure, value in (("promote", pair[61]), ("wait", pair[62]), ("promote_minus_wait", delta)):
                    rows[f"{base}/{measure}"].append((name, value))
                pa, sa = prediction["policy_action"], prediction["search_plan"][0]
                if pa in pair and sa in pair:
                    rows[f"{base}/search_choice_minus_policy_choice"].append((name, pair[sa]-pair[pa]))
                if pa == 62 and sa == 61:
                    rows[f"{base}/policy_wait_search_promote_delta"].append((name, delta))
                if pa == 62 and sa == 62:
                    rows[f"{base}/both_wait_promote_delta"].append((name, delta))
                cases.append(dict(node=result["node"]["id"], archetype=name, model=model, opponent=opponent,
                    policy=pa, search=sa, promote=pair[61], wait=pair[62], delta=delta))
            older, recent = pairs.get((model, 0)), pairs.get((model, 3))
            if older is not None and recent is not None:
                rows[f"model{model}/wait_advantage_recent_minus_old"].append(
                    (name, (recent[62]-recent[61])-(older[62]-older[61])))
    summary = dict(models=plan["models"], root_decisions=root_stats,
        analysis_driver_sha256=digest(__file__),
        source_games=len(collected), source_complete=sum(r["status"] == "complete" for r in collected),
        roots_available=sum(r.get("node") is not None for r in collected), roots_probed=len(complete),
        failed_jobs=len(list((root / "collect").glob("*.failure.json"))) + len(list((root / "probe").glob("*.failure.json"))),
        attempted_branches=sum(len(r["branches"]) for r in results),
        incomplete_branches=sum(b["outcome"]["status"] != "complete" for r in results for b in r["branches"]),
        paired_metrics={key: interval(values) for key, values in rows.items()}, cases=cases,
        bootstrap="4000 resamples of source-game roots within archetype; counterpart action outcomes stay paired",
        limitations=plan["limitations"])
    write(root / "summary.json", summary)
    print(json.dumps({k: summary[k] for k in ("source_games", "roots_available", "roots_probed", "attempted_branches", "incomplete_branches", "root_decisions")}))
    return summary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("stage", choices=["prepare", "collect", "probe", "summarize"])
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--engine", type=Path)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--limit", type=int)
    args = parser.parse_args()
    if not 1 <= args.workers <= 8:
        parser.error("workers must be between 1 and 8")
    root = args.root.resolve()
    if args.stage == "prepare":
        if args.engine is None:
            parser.error("prepare needs --engine")
        prepare(root, args.engine.resolve())
    elif args.stage == "summarize":
        summarize(root)
    else:
        run_stage(root, args.stage, args.workers, args.limit)


if __name__ == "__main__":
    main()
