"""Offline belief scores and conservative, preregistered promotion gates.

No game, sampler, or action selector imports this module.
"""

from __future__ import annotations
import argparse
import hashlib
import json
import math
from pathlib import Path


def fingerprint(value: dict) -> str:
    return hashlib.sha256(
        json.dumps(
            value, sort_keys=True, separators=(",", ":"), allow_nan=False
        ).encode()
    ).hexdigest()


def score_cards(prediction: dict[str, float], truth: list[str]) -> dict:
    if (
        not truth
        or not prediction
        or any(not math.isfinite(p) or p < 0 for p in prediction.values())
    ):
        raise ValueError("invalid probability distribution or empty scoring hand")
    if not math.isclose(sum(prediction.values()), 1, abs_tol=1e-6):
        raise ValueError("card probabilities must sum to one")
    support = sum(prediction.get(c, 0) > 0 for c in truth) / len(truth)
    log_loss = -sum(math.log(max(prediction.get(c, 0), 1e-15)) for c in truth) / len(
        truth
    )
    brier = sum(
        sum(p * p for p in prediction.values()) - 2 * prediction.get(c, 0) + 1
        for c in truth
    ) / len(truth)
    return {
        "cards": len(truth),
        "support_coverage": support,
        "log_loss": log_loss,
        "brier": brier,
    }


def interval(wins: int, games: int) -> tuple[float, float]:
    """97.5% Wilson component interval for a nominal 95% two-interval difference."""
    if (
        type(wins) is not int
        or type(games) is not int
        or not 0 <= wins <= games
        or games <= 0
    ):
        raise ValueError("invalid completed-game count")
    z = 2.241402727604947
    p = wins / games
    den = 1 + z * z / games
    mid = (p + z * z / (2 * games)) / den
    radius = z * math.sqrt((p * (1 - p) + z * z / (4 * games)) / games) / den
    return max(0, mid - radius), min(1, mid + radius)


def promotion(protocol: dict, result: dict) -> dict:
    """Missing/failed/truncated/inconclusive evidence never promotes."""
    failures = []
    if protocol.get("schema") != "fair-belief-protocol-v1":
        raise ValueError("unknown protocol")
    if result.get("protocol_hash") != fingerprint(protocol):
        failures.append("protocol hash mismatch")
    if result.get("evaluation_information_regime") != "closed_decklist":
        failures.append("evaluation is not closed-decklist")
    expected = set(protocol["seeds"])
    if set(result.get("seeds", [])) != expected:
        failures.append("missing preregistered training seeds")
    if result.get("budgets") != protocol["budgets"]:
        failures.append("comparison budgets mismatch")
    summaries = {}
    for name, rule in protocol["panels"].items():
        records = result.get("panels", {}).get(name, [])
        if (
            len(records) != len(expected)
            or {r.get("seed") for r in records} != expected
        ):
            failures.append(f"{name}: missing or duplicate seed results")
            continue
        if any(
            r.get("evaluation_mode") != rule["evaluation_mode"]
            or r.get("status") != "completed"
            for r in records
        ):
            failures.append(f"{name}: missing, failed, or wrong-mode evaluation")
            continue
        passed = True
        comparisons = []
        for r in records:
            try:
                for label in ("candidate", "baseline"):
                    row = r[label]
                    counts = [
                        row[k]
                        for k in ("wins", "losses", "draws", "failed", "truncated")
                    ]
                    if (
                        any(type(n) is not int or n < 0 for n in counts)
                        or sum(counts) != row["games"]
                    ):
                        raise ValueError(
                            "outcomes do not conserve the requested game denominator"
                        )
                    if row.get("failed", 0) or row.get("truncated", 0):
                        raise ValueError(
                            "failed/truncated games are not promotion evidence"
                        )
                    if row["games"] < rule["minimum_games_per_seed"]:
                        raise ValueError("insufficient games")
                    if (
                        row.get("first_seat_games") != row["games"] // 2
                        or row.get("second_seat_games")
                        != row["games"] - row["games"] // 2
                    ):
                        raise ValueError("evaluation seats are not balanced")
                c = interval(r["candidate"]["wins"], r["candidate"]["games"])
                b = interval(r["baseline"]["wins"], r["baseline"]["games"])
                delta = (c[0] - b[1], c[1] - b[0])
                ok = (
                    (delta[0] >= rule["margin"])
                    if rule["direction"] == "higher"
                    else (delta[1] <= rule["margin"])
                )
                comparisons.append(
                    {"seed": r["seed"], "delta_95_conservative": delta, "passed": ok}
                )
                passed &= ok
            except (KeyError, ValueError, TypeError) as error:
                failures.append(f"{name}: invalid denominator/evidence: {error}")
                passed = False
        summaries[name] = comparisons
        if not passed:
            failures.append(f"{name}: criterion failed or inconclusive")
    return {
        "eligible": not failures,
        "failures": failures,
        "panels": summaries,
        "exploitability_scope": "fresh finite-budget best-response estimate; no equilibrium certificate",
    }


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--protocol", type=Path)
    p.add_argument("--results", type=Path)
    p.add_argument("--score-run", type=Path)
    p.add_argument("--out", type=Path, required=True)
    a = p.parse_args(argv)
    if a.score_run:
        root = a.score_run
        manifest = json.loads((root / "manifest.json").read_text())
        if manifest.get("information", {}).get("regime") != "closed_decklist":
            raise ValueError("belief scoring requires closed-list provenance")

        def rows(name):
            result = {}
            for line in (root / name).read_text().splitlines():
                row = json.loads(line)
                key = (row["game"], row["decision"], row["viewer"])
                if key in result:
                    raise ValueError("duplicate scoring decision")
                result[key] = row
            return result

        predictions = rows("belief_predictions.jsonl")
        truth = rows("belief_scoring_truth.jsonl")
        if predictions.keys() != truth.keys():
            raise ValueError("prediction/truth decision coverage differs")
        completed = {
            k: v
            for k, v in predictions.items()
            if v["completed"] and truth[k]["completed"]
        }
        scores = [
            score_cards(v["probabilities"], truth[k]["actual_hand"])
            for k, v in completed.items()
        ]
        bins = [
            {"count": 0, "probability_sum": 0.0, "frequency_sum": 0.0}
            for _ in range(10)
        ]
        for k, v in completed.items():
            hand = truth[k]["actual_hand"]
            for card, probability in v["probabilities"].items():
                bucket = bins[min(9, int(probability * 10))]
                bucket["count"] += 1
                bucket["probability_sum"] += probability
                bucket["frequency_sum"] += hand.count(card) / len(hand)
        count = sum(s["cards"] for s in scores)
        report = {
            "completed_decisions": len(scores),
            "uncompleted_decisions": len(predictions) - len(scores),
            "cards": count,
            "manifest_sha256": hashlib.sha256(
                (root / "manifest.json").read_bytes()
            ).hexdigest(),
            "information": manifest["information"],
            "sampled_worlds": sum(
                v.get("sampled_worlds", 0) for v in completed.values()
            ),
            "fallback_worlds": sum(
                v.get("fallback_worlds", 0) for v in completed.values()
            ),
            "sampling_seconds": sum(
                v.get("sampling_seconds", 0) for v in completed.values()
            ),
            "fallback_mass_mean": (
                sum(v["fallback_mass"] for v in completed.values()) / len(completed)
                if completed
                else None
            ),
            "all_candidates_eliminated_decisions": sum(
                v["compatible_candidates"] == 0 for v in completed.values()
            ),
            "calibration_bins": bins,
            **{
                name: (
                    sum(s[name] * s["cards"] for s in scores) / count if count else None
                )
                for name in ("support_coverage", "log_loss", "brier")
            },
        }
        a.out.write_text(json.dumps(report, indent=2))
        return 0
    if not a.protocol or not a.results:
        p.error("gate requires --protocol and --results")
    report = promotion(
        json.loads(a.protocol.read_text()), json.loads(a.results.read_text())
    )
    a.out.write_text(json.dumps(report, indent=2))
    return 0 if report["eligible"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
