"""Render completed frozen-model study evidence; never runs games or training."""
from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("study", type=Path)
    args = parser.parse_args()
    root = args.study
    plan = json.loads((root / "plan.json").read_text())
    sweep = json.loads((root / "sweep_summary.json").read_text())
    path = root / "evaluation_extra_summary.json"
    if not path.exists():
        path = root / "evaluation_summary.json"
    evaluation = json.loads(path.read_text())
    baseline = plan["baseline"]["id"]
    metrics = {m["config"]["id"]: m for m in sweep["metrics"]}
    corpus = json.loads((root / "corpus.json").read_text())
    completed = [p for p in (root / "results").glob("match-*.json")
                 if not p.name.endswith((".done.json", ".progress.json")) and p.with_suffix(".done.json").exists()]
    records = [json.loads(file.read_text()) for file in completed]
    raw_games = [g for record in records for g in record["games"]]
    expected = ((len(evaluation["panels"]) - 1) * plan["head_pairs"] * 2
                + len(evaluation["panels"]) * 2 * plan["anchor_pairs"] * 2)
    if len(raw_games) != expected:
        raise ValueError(f"Incomplete result population: {len(raw_games)}/{expected} games")
    selected = evaluation["recommended_pilot_config"]["id"]
    lines = ["# Frozen-model search budget results", "",
        f"Recommended pilot setting: **{selected}**. Training remains stopped.", "",
        f"Population: {len(plan['decks'])} held-out English lists; {len(corpus['nodes'])} fixed roots "
        f"({sum(n['pending'] for n in corpus['nodes'])} pending selections); "
        f"{len(raw_games)} seat-balanced evaluation games, {sum(g['truncated'] for g in raw_games)} truncated.", "",
        "These are conditional simulator measurements with the frozen seed model and exact-composition open-decklist PIMC. "
        "No model was trained or promoted. The position panel is small and pending-choice-heavy. "
        "Higher-budget agreement is a proxy; synthetic controls do not certify real card interactions.", "",
        "## Fixed-root sweep", "",
        "| Simulations/worlds | Seconds/root | Reference agreement | Seed agreement | Visit JS bits | Tactics |",
        "|---|---:|---:|---:|---:|---:|"]
    with (root / "sweep.csv").open("w", newline="", encoding="utf-8") as out:
        writer = csv.writer(out)
        writer.writerow(["config", "seconds_per_root", "reference_agreement", "seed_agreement", "reference_js_bits", "tactical_passes", "tactical_total"])
        for m in sweep["metrics"]:
            c = m["config"]
            writer.writerow([c["id"], m["seconds_per_position"], m["mean_reference_agreement"], m["seed_action_agreement"], m["mean_reference_js_bits"], m["tactical_passes"], m["tactical_total"]])
            lines.append(f"| {c['simulations']}/{c['worlds']} | {m['seconds_per_position']:.3f} | "
                         f"{m['mean_reference_agreement']:.1%} | {m['seed_action_agreement']:.1%} | "
                         f"{m['mean_reference_js_bits']:.4f} | {m['tactical_passes']}/{m['tactical_total']} |")
    lines += ["", "## Paired full-game panels", "",
        "Scores count a win as 1, draw/cap as 0.5 and loss as 0. Intervals are descriptive block-bootstrap "
        "intervals widened by an approximate fractional-score Wilson interval; they are not exact or multiplicity-adjusted guarantees.", "",
        "| Candidate | Fixed opponent | Games | Mean score | Approx. 95% interval | Truncated |",
        "|---|---|---:|---:|---|---:|"]
    for candidate, panels in evaluation["panels"].items():
        for opponent, panel in panels.items():
            lo, hi = panel["score_ci95"]
            lines.append(f"| {candidate} | {opponent} | {panel['games']} | {panel['mean_score']:.1%} | "
                         f"{lo:.1%}–{hi:.1%} | {panel['truncated_games']} |")
    lines += ["", "## Full-game workload", "",
        "Worker-seconds sum elapsed game time across up to four concurrent workers; they are not CPU-seconds or "
        "end-to-end wall time. Different settings can change game length and the number of decisions. "
        "Search calls exclude forced single-action decisions and include both players when both use search.", "",
        "| Candidate | Fixed opponent | Decisions | Search calls | Worker-seconds/game | Worker-seconds/search call |",
        "|---|---|---:|---:|---:|---:|"]
    for candidate, panels in evaluation["panels"].items():
        for opponent in panels:
            games = [g for record in records
                     if record["candidate"]["id"] == candidate and record["opponent"]["id"] == opponent
                     for g in record["games"]]
            seconds = sum(g["seconds"] for g in games)
            searches = sum(g["searched"] for g in games)
            per_search = f"{seconds / searches:.3f}" if searches else "n/a"
            lines.append(f"| {candidate} | {opponent} | {sum(g['decisions'] for g in games)} | "
                         f"{searches} | {seconds / len(games):.2f} | {per_search} |")
    lines += ["", "## Recommendation gates", ""]
    for gate in evaluation["candidate_gates"]:
        lines.append(f"- {gate['config']['id']}: pilot gate {'passed' if gate['qualifies_pilot_gate'] else 'not cleared'}; "
                     f"{'cheaper' if gate['cheaper_than_baseline'] else 'more expensive'} than baseline.")
        for anchor, delta in gate["anchor_differences"].items():
            lo, hi = delta["ci95"]
            lines.append(f"  - Paired score difference versus {anchor}: {delta['mean_delta']:+.1%} "
                         f"(approx. 95% interval {lo:+.1%} to {hi:+.1%}).")
    lines += ["", evaluation["recommendation_status"], "",
        "All timing measurements are from the occupied Windows development host. Repeat on the actual Linux cloud host "
        "before sizing production runs. A fixed-model budget choice does not establish the best equal-compute training budget. "
        "The original two-alternative result and the preregistered cheaper-search supplement are retained separately.", ""]
    (root / "report.md").write_text("\n".join(lines), encoding="utf-8")
    # Standard scientific plotting, kept optional for environments without matplotlib.
    try:
        import matplotlib
        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
    except ImportError:
        print(f"Report and CSV written at {root}; matplotlib unavailable")
        return
    fig, axes = plt.subplots(1, 2, figsize=(12, 4.5), layout="constrained")
    colors = {4: "#2474a6", 8: "#b05d24", 16: "#6b4c9a"}
    for worlds in (4, 8, 16):
        rows = [m for m in sweep["metrics"] if m["config"]["worlds"] == worlds]
        axes[0].plot([m["seconds_per_position"] for m in rows],
                     [m["mean_reference_agreement"] * 100 for m in rows], "o-", color=colors[worlds], label=f"{worlds} worlds")
    base_cost = metrics[baseline]["seconds_per_position"]
    axes[0].annotate("320/4", (base_cost, metrics[baseline]["mean_reference_agreement"] * 100),
                     xytext=(8, -18), textcoords="offset points")
    axes[0].set(xlabel="Seconds per fixed root", ylabel="Agreement with references (%)",
                title="Position proxy, not ground truth", ylim=(80, 103))
    axes[0].legend()
    axes[0].grid(alpha=.2)
    axes[1].axhline(50, color="gray", linestyle="--", linewidth=1)
    for gate in evaluation["candidate_gates"]:
        key = gate["config"]["id"]
        panel = evaluation["panels"][key][baseline]
        value = panel["mean_score"] * 100
        lo, hi = [x * 100 for x in panel["score_ci95"]]
        x = metrics[key]["seconds_per_position"] / base_cost
        axes[1].errorbar(x, value, yerr=[[value - lo], [hi - value]], fmt="o", capsize=4,
                         color=colors[gate["config"]["worlds"]])
        axes[1].annotate(key, (x, value), xytext=(-6 if x > 3.5 else 5, 6),
                         ha="right" if x > 3.5 else "left", textcoords="offset points", fontsize=8)
    axes[1].set(xlabel="Fixed-root compute relative to 320/4", ylabel="Score against 320/4 (%)",
                title="Seat-balanced full games", ylim=(0, 100))
    axes[1].grid(alpha=.2)
    fig.suptitle("Frozen seed model • eight held-out English lists • exploratory study", fontsize=11)
    fig.savefig(root / "quality_compute.png", dpi=160)
    plt.close(fig)
    print(f"Report, CSV and figure written at {root}")


if __name__ == "__main__":
    main()
