"""Search-budget quality experiments on the frozen seed model. Diagnostic only.

Reuses the budget study's job runner (`tools.search_quality_study.Jobs`) and
its frozen 16-root corpus + 2560-simulation reference sweeps, but drives the
branch's `search_quality` example so per-root budgets (`reduced_simulations`)
and the Gumbel root (`root: gumbel`) can be evaluated.

Stages (run with PYTHONPATH=code):

  prepare  --parent-study DIR --engine EXE --out DIR
           Write a new plan.json next to the parent's frozen inputs (model,
           cards, decks, corpus); candidates come from --candidates JSON.
  references --out DIR [--workers N]
           Regenerate the reference sweeps when the model or engine differs
           from the parent study (prepare --model sets this up).
  panel    --out DIR [--workers N]
           Replay the 16 frozen roots under every candidate's EFFECTIVE budget
           (two search seeds) and compare the preferred action with the parent
           study's two reference sweeps, split into Main and pending roots.
  matches  --out DIR [--workers N] [--pairs 16]
           Seat-balanced games: each candidate vs the baseline on identical
           initial states, both seats, no root noise. Scores with the study's
           block-bootstrap + Wilson interval.

See docs/superpowers/research/2026-10-27-search-budget-experiments.md.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

from tools.search_quality_study import (Jobs, average_policy, best, js_distance, pair_job,
                                        score_interval)
from tools.selfplay_scheduler import atomic_json, sha256

DEFAULT_CANDIDATES = [
    {"id": "s320-w4", "simulations": 320, "worlds": 4},
    {"id": "c1-main320-other64", "simulations": 320, "worlds": 4, "reduced_simulations": 64},
]


def load_plan(out: Path) -> dict:
    return json.loads((out / "plan.json").read_text(encoding="utf-8"))


def prepare(args) -> int:
    parent = json.loads((args.parent_study / "plan.json").read_text(encoding="utf-8"))
    candidates = json.loads(args.candidates) if args.candidates else DEFAULT_CANDIDATES
    ids = [c["id"] for c in candidates]
    if len(set(ids)) != len(ids) or any("id" not in c or "simulations" not in c or "worlds" not in c for c in candidates):
        raise SystemExit("candidates need unique ids plus simulations/worlds")
    args.out.mkdir(parents=True, exist_ok=False)
    model = str(args.model.resolve()) if args.model else parent["model"]
    plan = {
        "schema": "search-budget-experiment-v1",
        "diagnostic_only": True,
        "engine": str(args.engine.resolve()),
        "engine_sha256": sha256(args.engine),
        "parent_study": str(args.parent_study.resolve()),
        "model": model,
        # The parent's reference sweeps are only reusable when model and engine
        # are unchanged; otherwise `references` regenerates them here.
        "references_from_parent": args.model is None,
        "cards_json": parent["cards_json"],
        "decks": parent["decks"],
        "corpus": str((args.parent_study / "corpus.json").resolve()),
        "references": parent["references"],
        "search_seeds": parent["search_seeds"],
        "baseline": parent["baseline"],
        "candidates": candidates,
        "match_seed": args.match_seed,
        "head_pairs": args.pairs,
        "decision_cap": parent["decision_cap"],
        "regime": parent.get("regime"),
        "evaluation_root_noise": 0,
        "batch_size": 4,
        "hashes": {"model": sha256(Path(model)), "cards_json": sha256(Path(parent["cards_json"]))},
        "corpus_sha256": sha256(args.parent_study / "corpus.json"),
    }
    atomic_json(args.out / "plan.json", plan)
    print(json.dumps({"plan": str(args.out / "plan.json"), "candidates": ids}), flush=True)
    return 0


def references(args) -> int:
    """Regenerate the 2560-simulation reference sweeps with THIS plan's model
    and engine (needed whenever either differs from the parent study)."""
    plan = load_plan(args.out)
    jobs = Jobs(plan, args.out, args.workers, args.timeout)
    work = [(f"sweep-{c['id']}-{seed}", "sweep",
             {"config": c, "seed": seed, "noise": 0, "corpus": plan["corpus"]})
            for c in plan["references"] for seed in plan["search_seeds"]]
    jobs.run(work)
    return 0


def _reference_policies(plan: dict, nodes: list[str], out: Path) -> dict[str, dict[str, dict]]:
    source = Path(plan["parent_study"]) if plan.get("references_from_parent", True) else out
    references = {}
    for settings in plan["references"]:
        by_node = {node: [] for node in nodes}
        for seed in plan["search_seeds"]:
            path = source / "results" / f"sweep-{settings['id']}-{seed}.json"
            if not path.exists():
                raise FileNotFoundError(f"{path}: run the `references` stage first")
            r = json.loads(path.read_text())
            for row in r["rows"]:
                by_node[row["node"]].append(row)
        references[settings["id"]] = {node: average_policy(rows) for node, rows in by_node.items()}
    return references


def _policy_rows(rows: list[dict]) -> list[dict]:
    """Rows whose `visits` are the distribution to average: a Gumbel root's
    completed-Q improved policy when the sweep recorded one, else visits."""
    return [{**row, "visits": row["policy"]} if row.get("policy") else row for row in rows]


def _candidate_action(rows: list[dict]) -> int:
    """Preferred (deterministic) action of a candidate over its seeds: the
    argmax of the mean improved policy (Gumbel) or mean visit policy (PUCT)."""
    return best(average_policy(_policy_rows(rows)))


def panel(args) -> int:
    plan = load_plan(args.out)
    corpus = json.loads(Path(plan["corpus"]).read_text(encoding="utf-8"))
    nodes = [node["id"] for node in corpus["nodes"]]
    pending = {node["id"]: bool(node["pending"]) or node["phase"] != "Main" for node in corpus["nodes"]}
    jobs = Jobs(plan, args.out, args.workers, args.timeout)
    work = [(f"sweep-{c['id']}-{seed}", "sweep",
             {"config": c, "seed": seed, "noise": 0, "corpus": plan["corpus"]})
            for c in plan["candidates"] for seed in plan["search_seeds"]]
    jobs.run(work)
    references = _reference_policies(plan, nodes, args.out)
    reference_ids = list(references)
    summary = []
    for c in plan["candidates"]:
        by_node = {node: [] for node in nodes}
        for seed in plan["search_seeds"]:
            r = json.loads((args.out / "results" / f"sweep-{c['id']}-{seed}.json").read_text())
            for row in r["rows"]:
                row["gumbel"] = c.get("root") == "gumbel"
                by_node[row["node"]].append(row)
        preferred = {n: _candidate_action(rows) for n, rows in by_node.items()}
        policies = {n: average_policy(_policy_rows(rows)) for n, rows in by_node.items()}

        def agreement(subset):
            if not subset:
                return None
            return {ref: sum(preferred[n] == best(references[ref][n]) for n in subset) / len(subset)
                    for ref in reference_ids}

        main_nodes = [n for n in nodes if not pending[n]]
        pending_nodes = [n for n in nodes if pending[n]]
        metric = {
            "config": c,
            "seconds_per_root": sum(r["seconds"] for rows in by_node.values() for r in rows)
            / (len(nodes) * len(plan["search_seeds"])),
            "seconds_per_main_root": (sum(r["seconds"] for n in main_nodes for r in by_node[n])
                                      / (len(main_nodes) * len(plan["search_seeds"]))) if main_nodes else None,
            "seconds_per_pending_root": (sum(r["seconds"] for n in pending_nodes for r in by_node[n])
                                         / (len(pending_nodes) * len(plan["search_seeds"]))) if pending_nodes else None,
            "simulations_run": sorted({r["simulations_run"] for rows in by_node.values() for r in rows}),
            "seed_action_agreement": sum(len({r["action"] for r in rows}) == 1 for rows in by_node.values()) / len(nodes),
            "reference_action_agreement": agreement(nodes),
            "main_root_agreement": agreement(main_nodes),
            "pending_root_agreement": agreement(pending_nodes),
            "mean_reference_js_bits": sum(js_distance(policies[n], references[ref][n]) for n in nodes for ref in references)
            / (len(nodes) * len(references)),
            "disagreements": {ref: [n for n in nodes if preferred[n] != best(references[ref][n])] for ref in reference_ids},
        }
        summary.append(metric)
        print(json.dumps({"config": c["id"], "agreement": metric["reference_action_agreement"],
                          "main": metric["main_root_agreement"], "pending": metric["pending_root_agreement"],
                          "seconds_per_root": round(metric["seconds_per_root"], 3)}), flush=True)
    atomic_json(args.out / "panel_summary.json", {
        "scope": "reference agreement is a proxy, not ground truth; two seeds and sixteen correlated roots "
                 "(three Main, thirteen pending); Gumbel candidates report the survivor action",
        "positions": len(nodes), "main_roots": len([n for n in nodes if not pending[n]]),
        "metrics": summary})
    return 0


def matches(args) -> int:
    plan = load_plan(args.out)
    baseline = plan["baseline"]
    candidates = [c for c in plan["candidates"] if c["id"] != baseline["id"]]
    if args.candidates:
        wanted = set(args.candidates.split(","))
        missing = wanted - {c["id"] for c in candidates}
        if missing:
            raise SystemExit(f"unknown match candidates: {sorted(missing)}")
        candidates = [c for c in candidates if c["id"] in wanted]
    jobs = Jobs(plan, args.out, args.workers, args.timeout)
    work = [pair_job(plan, c, baseline, i) for c in candidates for i in range(plan["head_pairs"])]
    jobs.run(work)
    n_decks = len(plan["decks"])
    panels = {}
    for c in candidates:
        pairs, truncations, seconds, searched, candidate_seconds, opponent_seconds = [], 0, 0.0, 0, 0.0, 0.0
        for i in range(plan["head_pairs"]):
            name, _, _ = pair_job(plan, c, baseline, i)
            r = json.loads((args.out / "results" / f"{name}.json").read_text())
            assert {g["candidate_seat"] for g in r["games"]} == {0, 1}
            assert sum(g["candidate_seat"] == g["first_player"] for g in r["games"]) == 1
            pairs.append(sum(g["score"] for g in r["games"]) / 2)
            truncations += sum(g["truncated"] for g in r["games"])
            seconds += r["seconds"]
            searched += sum(g["searched"] for g in r["games"])
        panels[c["id"]] = {
            "pairs": plan["head_pairs"], "games": 2 * plan["head_pairs"],
            "mean_score": sum(pairs) / len(pairs),
            "score_ci95": score_interval(pairs, [i % n_decks for i in range(len(pairs))]),
            "pair_scores": pairs, "truncated_games": truncations, "process_seconds": seconds,
            "searched_decisions_both_seats": searched,
        }
        print(json.dumps({"config": c["id"], **{k: panels[c["id"]][k] for k in ("mean_score", "score_ci95", "truncated_games")}}), flush=True)
    atomic_json(args.out / "match_summary.json", {
        "scope": "seat-balanced pairs vs the baseline on identical initial states; "
                 "block bootstrap over matchup blocks widened by a pair-level Wilson interval; "
                 "a lower bound at or above 0.45 for a cheaper candidate means no large regression detected, nothing stronger",
        "baseline": baseline, "panels": panels})
    return 0


def main(argv=None) -> int:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="stage", required=True)
    prep = sub.add_parser("prepare")
    prep.add_argument("--parent-study", type=Path, required=True)
    prep.add_argument("--engine", type=Path, required=True)
    prep.add_argument("--out", type=Path, required=True)
    prep.add_argument("--candidates", help="JSON list of candidate settings; defaults to the documented set")
    prep.add_argument("--model", type=Path, help="policy+value ONNX export; defaults to the parent study's model")
    prep.add_argument("--match-seed", type=int, default=2026102700)
    prep.add_argument("--pairs", type=int, default=16)
    prep.set_defaults(func=prepare)
    for name, func in (("references", references), ("panel", panel), ("matches", matches)):
        s = sub.add_parser(name)
        s.add_argument("--out", type=Path, required=True)
        s.add_argument("--workers", type=int, default=4)
        s.add_argument("--timeout", type=float, default=3 * 3600)
        if name == "matches":
            s.add_argument("--candidates", help="comma-separated candidate ids to play (default: all non-baseline)")
        s.set_defaults(func=func)
    args = p.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
