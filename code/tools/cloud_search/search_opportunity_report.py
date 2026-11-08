"""Descriptive opportunity/choice counts from new native telemetry, never scores.

No inference, replay, or reconstruction of missing historical probabilities.
"""
from __future__ import annotations
import argparse
from collections import defaultdict
import hashlib
import json
import math
from pathlib import Path


def entropy(weights):
    if any(w is None for w in weights):
        return None
    total = sum(weights)
    return -sum((w / total) * math.log(w / total) for w in weights if w > 0) if total > 0 else None


def summarize(rows):
    rows = list(rows)
    counts = defaultdict(int)
    samples = defaultdict(list)
    for row in rows:
        if row.get("schema") != "search-opportunities-v1":
            raise ValueError("missing or unsupported opportunity telemetry")
        plans = row["plans"]
        selected = [p for p in plans if p["selected"]]
        if len(selected) != 1 or selected[0]["plan"] != row["selected_plan"]:
            raise ValueError("selected plan identity mismatch")
        selected = selected[0]
        executed = row["executed_actions"]
        if row["selected_plan"][:len(executed)] != executed or row["commitment_executed"] != (executed == row["selected_plan"]):
            raise ValueError("execution receipt is not the declared plan prefix")
        total = sum(p["visits"] for p in plans)
        if total != row["total_visits"]:
            raise ValueError("visit total mismatch")
        for p in plans:
            if p["visits"] < 0 or any(p[k] is not None and (not math.isfinite(p[k]) or p[k] < 0) for k in ("prior_before_noise", "prior_after_noise", "selection_probability")):
                raise ValueError("invalid nonnegative telemetry")
        counts["searched_roots"] += 1
        counts["committed_roots"] += bool(row["commitment_executed"])
        counts["selected_below_max_visits"] += selected["visits"] < max(p["visits"] for p in plans)
        if all(p["prior_before_noise"] is not None for p in plans):
            counts["roots_with_priors"] += 1
            counts["selected_below_max_prior"] += selected["prior_before_noise"] < max(p["prior_before_noise"] for p in plans)
        for key in ("prior_before_noise", "prior_after_noise", "selection_probability", "visits"):
            value = entropy([p[key] for p in plans])
            if value is not None:
                samples[key + "_entropy_nats"].append(value)
        counts["plans"] += len(plans)
        counts["zero_visit_plans"] += sum(p["visits"] == 0 for p in plans)
        if total:
            samples["selected_visit_share"].append(selected["visits"] / total)
        category = selected["category"]
        counts["selected_" + category] += 1
        counts["executed_" + category] += bool(row["commitment_executed"])
        tamers = [p for p in plans if p["category"] == "tamer_digivolve"]
        if not tamers:
            continue
        counts["tamer_opportunity_roots"] += 1
        counts["tamer_plans"] += len(tamers)
        counts["tamer_zero_visit_plans"] += sum(p["visits"] == 0 for p in tamers)
        counts["tamer_selected_at_opportunity"] += category == "tamer_digivolve"
        counts["tamer_executed_at_opportunity"] += category == "tamer_digivolve" and row["commitment_executed"]
        counts["hard_play_selected_at_tamer_opportunity"] += category == "hard_play"
        counts["hard_play_selected_with_same_card_tamer_route"] += category == "hard_play" and any(p["action"]["card_id"] == selected["action"]["card_id"] for p in tamers)
        for key in ("prior_before_noise", "prior_after_noise", "selection_probability"):
            values = [p[key] for p in tamers]
            if all(v is not None for v in values):
                samples["tamer_" + key + "_mass"].append(sum(values))
        if total:
            samples["tamer_visit_share"].append(sum(p["visits"] for p in tamers) / total)
    return {"counts": dict(counts), "means": {k: {"value": sum(v) / len(v), "measured_roots": len(v)} for k, v in samples.items()},
            "scope": "Descriptive searched-root counts; correlated decisions, not game scores or proof a move was good. Missing priors/zero-total visit shares remain unmeasured. Effect-sequence opportunities unmeasured."}


def load_rows(path):
    if path.suffix == ".jsonl":
        for line in path.read_text().splitlines():
            if line.strip():
                yield json.loads(line)
    else:
        data = json.loads(path.read_text())
        # New two-model native driver; original full-game reports lack this field.
        for game in data["games"]:
            for search in game["search_diagnostics"]:
                row = dict(search["opportunity"])
                start = search["decision"]
                executed = row["executed_actions"]
                if game["actions"][start:start + len(executed)] != executed:
                    raise ValueError("opportunity execution differs from game trace")
                row["context"] = {"candidate_seat": game["candidate_seat"], "request": data.get("request"), "decision": search["decision"],
                    "model": data["model_sha256"] if row["actor"] == game["candidate_seat"] else data["opponent_model_sha256"],
                    "information_regime": search["diagnostics"]["information"]["regime"],
                    "decks": data["request"]["deck_indices"]}
                yield row


def report(paths):
    rows = []
    sources = []
    for path in paths:
        rows.extend(load_rows(path))
        sources.append({"path": str(path.resolve()), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()})
    groups = defaultdict(list)
    for row in rows:
        context = row.get("context", {})
        key = json.dumps({"actor": row["actor"], "sampling": row["sampling"],
            "full_search": context.get("full_search"), "model": context.get("historical_opponent") if context.get("policy_training_actor") is False else context.get("model"),
            "policy_training_actor": context.get("policy_training_actor"),
            "historical_opponent": context.get("historical_opponent"),
            "information_regime": context.get("information_regime"),
            "decks": context.get("decks"), "phase": row["phase"]}, sort_keys=True)
        groups[key].append(row)
    return {"schema": "search-opportunity-report-v1", "sources": sources, "overall": summarize(rows),
            "groups": [{"group": json.loads(k), **summarize(v)} for k, v in sorted(groups.items())]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inputs", nargs="+", type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    result = report(args.inputs)
    args.out.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result["overall"]["counts"]))


if __name__ == "__main__":
    main()
