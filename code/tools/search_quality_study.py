"""Frozen-model budget study. No training imports or training-run mutations.

Run with PYTHONPATH=code. This is an exploratory, covered-pool development study,
not a promotion gate or proof that an imperfect-information search is optimal.
"""
from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor, wait, FIRST_COMPLETED
import hashlib
import json
import math
import os
from pathlib import Path
import random
import signal
import subprocess
import threading
import time

from tools.selfplay_scheduler import atomic_json, controller_lock, sha256


def cfg(simulations, worlds):
    return {"id": f"s{simulations}-w{worlds}", "simulations": simulations, "worlds": worlds}


def normalized(visits):
    total = sum(v for _, v in visits)
    if total <= 0:
        raise ValueError("No search visits")
    return {int(a): v / total for a, v in visits}


def best(distribution):
    return max(distribution, key=distribution.get)


def js_distance(a, b):
    """Jensen-Shannon divergence in bits, bounded 0..1; not value regret."""
    score = 0.0
    for action in a.keys() | b.keys():
        x, y = a.get(action, 0.0), b.get(action, 0.0)
        mid = (x + y) / 2
        if x:
            score += 0.5 * x * math.log2(x / mid)
        if y:
            score += 0.5 * y * math.log2(y / mid)
    return score


def cluster_interval(values, groups, seed=997, samples=10000):
    """Percentile bootstrap over matchup/archetype blocks, retaining seat pairs."""
    if not values:
        return None
    buckets = {}
    for value, group in zip(values, groups, strict=True):
        buckets.setdefault(group, []).append(value)
    labels = list(buckets)
    rng = random.Random(seed)
    draws = []
    for _ in range(samples):
        chosen = [x for _ in labels for x in buckets[rng.choice(labels)]]
        draws.append(sum(chosen) / len(chosen))
    draws.sort()
    return [draws[int(samples * 0.025)], draws[min(samples - 1, int(samples * 0.975))]]


def score_interval(values, groups):
    # Keep a finite-sample uncertainty floor when a small panel is all ties.
    # Fractional-score Wilson is approximate; report it with the block bootstrap.
    estimate, n, z = sum(values) / len(values), len(values), 1.95996398454
    denominator = 1 + z * z / n
    center = (estimate + z * z / (2 * n)) / denominator
    half = z * math.sqrt(estimate * (1 - estimate) / n + z * z / (4 * n * n)) / denominator
    boot = cluster_interval(values, groups)
    return [min(boot[0], center - half), max(boot[1], center + half)]


def average_policy(rows):
    result = {}
    for row in rows:
        for action, p in normalized(row["visits"]).items():
            result[action] = result.get(action, 0) + p / len(rows)
    return result


def analyze_sweep(plan, directory):
    corpus = json.loads((directory / "corpus.json").read_text())
    nodes = [node["id"] for node in corpus["nodes"]]
    records = {}
    for settings in plan["grid"] + plan["references"]:
        by_node = {node: [] for node in nodes}
        for seed in plan["search_seeds"]:
            r = json.loads((directory / "results" / f"sweep-{settings['id']}-{seed}.json").read_text())
            for row in r["rows"]:
                by_node[row["node"]].append(row)
        records[settings["id"]] = by_node
    references = {c["id"]: {node: average_policy(rows) for node, rows in records[c["id"]].items()}
                  for c in plan["references"]}
    reference_ids = list(references)
    consensus = [n for n in nodes if len({best(references[c][n]) for c in reference_ids}) == 1]
    tactical = json.loads((directory / "results/tactics.json").read_text())["rows"]
    metrics = []
    for settings in plan["grid"]:
        by_node = records[settings["id"]]
        policies = {n: average_policy(rows) for n, rows in by_node.items()}
        tests = [r for r in tactical if r["config"]["id"] == settings["id"]]
        stability = sum(len({r["action"] for r in rows}) == 1 for rows in by_node.values()) / len(nodes)
        agreement = {c: sum(best(policies[n]) == best(references[c][n]) for n in nodes) / len(nodes)
                     for c in reference_ids}
        mean_seconds = sum(r["seconds"] for rows in by_node.values() for r in rows) / (len(nodes) * len(plan["search_seeds"]))
        metric = {"config": settings, "seconds_per_position": mean_seconds,
                  "seed_action_agreement": stability, "reference_action_agreement": agreement,
                  "mean_reference_agreement": sum(agreement.values()) / len(agreement),
                  "consensus_reference_positions": len(consensus),
                  "consensus_action_agreement": sum(best(policies[n]) == best(references[reference_ids[0]][n]) for n in consensus) / len(consensus) if consensus else None,
                  "mean_reference_js_bits": sum(js_distance(policies[n], references[c][n]) for n in nodes for c in references) / (len(nodes) * len(references)),
                  "tactical_passes": sum(t["passed"] for t in tests), "tactical_total": len(tests),
                  "tactical_all_pass": bool(tests) and all(t["passed"] for t in tests)}
        metrics.append(metric)
    eligible = [m for m in metrics if m["tactical_all_pass"]]
    ranked = sorted(eligible or metrics, key=lambda m: (-m["mean_reference_agreement"], -m["seed_action_agreement"], m["mean_reference_js_bits"], m["seconds_per_position"]))
    quality = ranked[0]
    cheap_pool = [m for m in eligible if m["mean_reference_agreement"] >= quality["mean_reference_agreement"] - 0.10]
    cheap = min(cheap_pool or ranked, key=lambda m: m["seconds_per_position"])
    finalists = []
    for item in [cheap, quality, *ranked]:
        c = item["config"]
        if c["id"] != plan["baseline"]["id"] and c not in finalists:
            finalists.append(c)
        if len(finalists) == 2:
            break
    return {"scope": "reference agreement is a proxy, not ground truth; two seeds and sixteen correlated roots",
            "positions": len(nodes), "metrics": metrics, "finalists": finalists}


class Jobs:
    def __init__(self, plan, directory, workers, timeout):
        self.plan, self.directory, self.workers, self.timeout = plan, directory, workers, timeout
        self.lock = threading.Lock()
        self.active = {}
        self.stopping = False
        for name in ("requests", "results", "logs"):
            (directory / name).mkdir(exist_ok=True)

    def one(self, name, mode, request):
        if self.plan.get("training_state") and sha256(Path(self.plan["training_state"])) != self.plan["training_state_sha256"]:
            raise RuntimeError("Training stop-state changed; halt isolated study")
        request = {"plan": str((self.directory / "plan.json").resolve()), **request}
        req_path = self.directory / "requests" / f"{name}.json"
        output = self.directory / "results" / f"{name}.json"
        done = output.with_suffix(".done.json")
        if req_path.exists() and json.loads(req_path.read_text()) != request:
            raise ValueError(f"Changed request: {name}")
        atomic_json(req_path, request)
        if done.exists():
            record = json.loads(done.read_text())
            if record != {"request_sha256": sha256(req_path), "output_sha256": sha256(output)}:
                raise ValueError(f"Changed completed result: {name}")
            return json.loads(output.read_text())
        if output.exists():
            raise FileExistsError(f"Uncommitted result preserved: {output}")
        with self.lock:
            if self.stopping:
                raise InterruptedError("Study stopped")
            log = (self.directory / "logs" / f"{name}.log").open("w", encoding="utf-8")
            env = dict(os.environ, DIGIMON_SEARCH_PROFILE="0", DIGIMON_SEARCH_EVAL_CACHE_ENTRIES="0")
            proc = subprocess.Popen([self.plan["engine"], mode, str(req_path), str(output)],
                                    stdout=log, stderr=subprocess.STDOUT, env=env)
            self.active[name] = (proc, time.monotonic())
        try:
            rc = proc.wait(timeout=self.timeout)
            if rc:
                raise RuntimeError(f"{name} failed with exit {rc}; see its log")
            result = json.loads(output.read_text())
            atomic_json(done, {"request_sha256": sha256(req_path), "output_sha256": sha256(output)})
            print(f"completed {name}", flush=True)
            return result
        finally:
            if proc.poll() is None:
                proc.kill()
                proc.wait()
            log.close()
            with self.lock:
                self.active.pop(name, None)

    def run(self, jobs):
        executor = ThreadPoolExecutor(max_workers=self.workers)
        futures = set()
        try:
            futures = {executor.submit(self.one, *job) for job in jobs}
            while futures:
                completed, futures = wait(futures, timeout=15, return_when=FIRST_COMPLETED)
                for f in completed:
                    f.result()
                with self.lock:
                    atomic_json(self.directory / "study_progress.json", {"remaining_jobs": len(futures),
                        "heartbeat_unix": time.time(), "active": {name: {"pid": proc.pid,
                            "elapsed_seconds": time.monotonic() - started} for name, (proc, started) in self.active.items()}})
        except BaseException:
            with self.lock:
                self.stopping = True
                for proc, _ in self.active.values():
                    if proc.poll() is None:
                        proc.terminate()
            for f in futures:
                f.cancel()
            raise
        finally:
            executor.shutdown(wait=True, cancel_futures=True)


def pair_job(plan, candidate, opponent, index):
    n = len(plan["decks"])
    opponent_id = opponent.get("id", opponent["kind"] if "kind" in opponent else "search")
    name = f"match-{candidate['id']}-vs-{opponent_id}-{index:03d}"
    return name, "match", {"candidate": candidate, "opponent": opponent,
        "deck_indices": [index % n, (index * 3 + 1) % n],
        "game_seed": plan["match_seed"] + index, "search_seed": plan["match_seed"] + 100000 + index}


def summarize_matches(plan, directory, finalists, sweep):
    panels = {}
    for candidate in [plan["baseline"], *finalists]:
        candidate_panels = {}
        opponents = [plan["baseline"]] if candidate != plan["baseline"] else []
        opponents += [{"id": "greedy", "kind": "greedy"}, {"id": "policy", "kind": "policy"}]
        for opponent in opponents:
            count = plan["head_pairs"] if opponent == plan["baseline"] else plan["anchor_pairs"]
            pairs, truncations, seconds = [], 0, 0.0
            for i in range(count):
                name, _, _ = pair_job(plan, candidate, opponent, i)
                r = json.loads((directory / "results" / f"{name}.json").read_text())
                assert {g["candidate_seat"] for g in r["games"]} == {0, 1}
                assert sum(g["candidate_seat"] == g["first_player"] for g in r["games"]) == 1
                pairs.append(sum(g["score"] for g in r["games"]) / 2)
                truncations += sum(g["truncated"] for g in r["games"])
                seconds += r["seconds"]
            candidate_panels[opponent["id"]] = {"pairs": count, "games": 2 * count,
                "mean_score": sum(pairs) / count, "score_ci95": score_interval(pairs, [i % len(plan["decks"]) for i in range(count)]),
                "pair_scores": pairs, "truncated_games": truncations, "process_seconds": seconds}
        panels[candidate["id"]] = candidate_panels
    baseline = plan["baseline"]["id"]
    metrics = {m["config"]["id"]: m for m in sweep["metrics"]}
    choices = []
    for candidate in finalists:
        key = candidate["id"]
        head = panels[key][baseline]
        differences = {}
        for anchor in ("greedy", "policy"):
            delta = [a - b for a, b in zip(panels[key][anchor]["pair_scores"], panels[baseline][anchor]["pair_scores"], strict=True)]
            differences[anchor] = {"mean_delta": sum(delta) / len(delta),
                "ci95": cluster_interval(delta, list(range(len(delta))))}
        healthy = all(p["truncated_games"] == 0 for p in panels[key].values())
        anchors_ok = all(d["ci95"][0] >= -0.05 for d in differences.values())
        tactical_ok = metrics[key]["tactical_all_pass"]
        cheaper = metrics[key]["seconds_per_position"] < metrics[baseline]["seconds_per_position"]
        head_ok = head["score_ci95"][0] >= 0.45 if cheaper else head["score_ci95"][0] > 0.50
        qualifies = healthy and anchors_ok and tactical_ok and head_ok
        choices.append({"config": candidate, "cheaper_than_baseline": cheaper,
                        "anchor_differences": differences, "qualifies_pilot_gate": qualifies})
    qualified = [c for c in choices if c["qualifies_pilot_gate"]]
    selected = min(qualified, key=lambda c: metrics[c["config"]["id"]]["seconds_per_position"])["config"] if qualified else plan["baseline"]
    return {"panels": panels, "candidate_gates": choices, "recommended_pilot_config": selected,
        "recommendation_status": "provisional covered-pool choice; no globally optimal or promotion claim",
        "scope": "fixed seed model; eight held-out lists; exact-composition open-decklist PIMC; confidence intervals cluster by eight matchup blocks; exploratory multiple comparisons"}


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--engine", type=Path, required=True)
    p.add_argument("--model", type=Path, required=True)
    p.add_argument("--snapshot", type=Path, required=True)
    p.add_argument("--cards-json", type=Path, required=True)
    p.add_argument("--out", type=Path, required=True)
    p.add_argument("--workers", type=int, default=4)
    p.add_argument("--head-pairs", type=int, default=16)
    p.add_argument("--anchor-pairs", type=int, default=8)
    p.add_argument("--job-timeout", type=float, default=1800)
    p.add_argument("--stage", choices=["sweep", "evaluate", "all", "extra"], default="all")
    p.add_argument("--extra-config", nargs="+", default=["s160-w4"], help="predeclared supplemental compute challengers")
    p.add_argument("--resume", action="store_true")
    args = p.parse_args(argv)
    if min(args.workers, args.head_pairs, args.anchor_pairs) < 1:
        p.error("worker and pair counts must be positive")
    snapshot = json.loads(args.snapshot.read_text())
    plan = {"schema": "search-quality-study-v1", "diagnostic_only": True,
        "engine": str(args.engine.resolve()), "model": str(args.model.resolve()), "cards_json": str(args.cards_json.resolve()),
        "decks": [{"name": d["archetype"], "cards": d["card_ids"], "source_deck_id": d["deck_id"]} for d in snapshot["decks"]],
        "grid": [cfg(s, w) for s in (160, 320, 640, 1280) for w in (4, 8, 16)],
        "references": [cfg(2560, 8), cfg(2560, 32)], "baseline": cfg(320, 4),
        "search_seeds": [77101, 77102], "tactical_seeds": [77101, 77102, 77103],
        "corpus_seed": 2026102300, "match_seed": 2026102400, "decision_cap": 1200,
        "head_pairs": args.head_pairs, "anchor_pairs": args.anchor_pairs,
        "regime": "open_decklist_exact_composition", "evaluation_root_noise": 0,
        "batch_size": 4, "source_snapshot_hash": snapshot["snapshot_hash"],
        "selection": "quality leader by reference agreement/stability/JS; cheap leader within 10pp agreement; two finalists excluding baseline; predeclared noninferiority pilot margins"}
    assets = {"engine": args.engine, "model": args.model, "metadata": Path(str(args.model) + ".meta.json"),
              "snapshot": args.snapshot, "cards": args.cards_json,
              "mechanics": args.cards_json.parent / "mechanics_readiness.json",
              "admission": args.cards_json.parent / "card_admissibility.json"}
    plan["hashes"] = {key: sha256(path) for key, path in assets.items()}
    training_state = args.model.parent / "pipeline_state.json"
    if training_state.exists():
        state = json.loads(training_state.read_text())
        if state.get("status") != "stopped_by_user" or state.get("auto_resume") is not False:
            raise ValueError("This study requires the existing training run to remain stopped")
        plan["training_state"] = str(training_state.resolve())
        plan["training_state_sha256"] = sha256(training_state)
    if args.out.exists() and not args.resume:
        raise FileExistsError("Preserving study outputs; explicit --resume required")
    args.out.mkdir(parents=True, exist_ok=True)
    with controller_lock(args.out / "study.lock"):
        plan_path = args.out / "plan.json"
        if plan_path.exists() and json.loads(plan_path.read_text()) != plan:
            raise ValueError("Frozen study plan/inputs changed")
        atomic_json(plan_path, plan)
        jobs = Jobs(plan, args.out, args.workers, args.job_timeout)
        if args.stage in ("sweep", "all"):
            jobs.run([("corpus", "corpus", {})])
            corpus_result = args.out / "results/corpus.json"
            corpus = args.out / "corpus.json"
            if not corpus.exists():
                atomic_json(corpus, json.loads(corpus_result.read_text()))
            if json.loads(corpus.read_text()) != json.loads(corpus_result.read_text()):
                raise ValueError("Corpus changed")
            work = [(f"sweep-{c['id']}-{seed}", "sweep", {"config": c, "seed": seed,
                "noise": 0, "corpus": str(corpus.resolve())}) for c in plan["grid"] + plan["references"] for seed in plan["search_seeds"]]
            random.Random(997).shuffle(work)
            jobs.run([("tactics", "tactics", {}), *work])
            atomic_json(args.out / "sweep_summary.json", analyze_sweep(plan, args.out))
        if args.stage in ("evaluate", "all", "extra"):
            sweep = json.loads((args.out / "sweep_summary.json").read_text())
            finalists = list(sweep["finalists"])
            if args.stage == "extra":
                supplement = json.loads((args.out / "supplement_plan.json").read_text())
                if supplement["base_plan_sha256"] != sha256(plan_path) or supplement["extra_configs"] != args.extra_config:
                    raise ValueError("Supplement does not match its preregistered plan")
                for key in args.extra_config:
                    candidate = next(c for c in plan["grid"] if c["id"] == key)
                    if candidate != plan["baseline"] and candidate not in finalists:
                        finalists.append(candidate)
            work = []
            for candidate in [plan["baseline"], *finalists]:
                if candidate != plan["baseline"]:
                    work += [pair_job(plan, candidate, plan["baseline"], i) for i in range(args.head_pairs)]
                for anchor in ("greedy", "policy"):
                    work += [pair_job(plan, candidate, {"id": anchor, "kind": anchor}, i) for i in range(args.anchor_pairs)]
            random.Random(1001).shuffle(work)
            jobs.run(work)
            summary_name = "evaluation_extra_summary.json" if args.stage == "extra" else "evaluation_summary.json"
            atomic_json(args.out / summary_name, summarize_matches(plan, args.out, finalists, sweep))
        print("Study stage complete; training untouched", flush=True)
    return 0


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, lambda *_: (_ for _ in ()).throw(KeyboardInterrupt()))
    raise SystemExit(main())
