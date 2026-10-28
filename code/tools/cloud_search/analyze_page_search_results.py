"""Read-only analysis of exported, matched checkpoint evaluation games."""

from __future__ import annotations

import argparse
from collections import Counter, defaultdict
import csv
import gzip
import json
from pathlib import Path

import numpy as np


def score(game):
    return 0.5 if game["winner_id"] in (None, 0) else float(game["winner_id"] == 1)


def cohort_key(game):
    c = game["context"]
    return c["anchor"], c["agent_archetype"], game["seed"]


def paired(a, b, samples=10000):
    aa = {cohort_key(g): g for g in a}
    bb = {cohort_key(g): g for g in b}
    assert len(aa) == len(a) and len(bb) == len(b) and aa.keys() == bb.keys()
    groups = defaultdict(list)
    improved = worsened = 0
    for key in sorted(aa):
        assert aa[key]["context"] == bb[key]["context"]
        delta = score(bb[key]) - score(aa[key])
        groups[key[:2]].append((key[2], delta))
        improved += delta > 0
        worsened += delta < 0
    clusters = []
    for values in groups.values():
        assert len(values) % 2 == 0
        assert all(values[i][0] % 2 == 0 and values[i + 1][0] % 2 == 1
                   for i in range(0, len(values), 2))
        clusters.extend(np.array([v for _, v in values]).reshape(-1, 2).mean(axis=1))
    x = np.array(clusters)
    rng = np.random.default_rng(19)
    means = []
    for start in range(0, samples, 200):
        means.extend(x[rng.integers(len(x), size=(min(200, samples-start), len(x)))].mean(axis=1))
    lo, hi = np.quantile(means, [.025, .975])
    return dict(paired_games=len(a), delta_pp=float(x.mean() * 100),
                ci95_low_pp=float(lo * 100), ci95_high_pp=float(hi * 100),
                improved_games=improved, worsened_games=worsened,
                unchanged_games=len(a)-improved-worsened)


def behavior(games):
    counts = Counter()
    routes = defaultdict(Counter)
    sequences = Counter()
    dna_games = dna_sequence_games = 0
    dna_opportunity_games = 0
    for g in games:
        counts.update(g["counts"])
        dna_games += g["counts"]["dna_digivolves"] > 0
        for row in g["route_decisions"]:
            assert row["player"] == 1
            routes[row["route"]].update({k: row[k] for k in ("opportunities", "choices")})
        rows = [r for r in g["effect_sequences"] if r["card_id"] == "EX12-001"]
        dna_sequence_games += any(r.get("dna_completed", 0) > 0 for r in rows)
        dna_opportunity_games += any(r.get("opportunities", 0) > 0 for r in rows)
        for row in rows:
            assert row["player"] == 1
            sequences.update({k: v for k, v in row.items()
                              if k not in ("player", "card_id", "card_name") and isinstance(v, (int, float))})
    return dict(games=len(games), events=dict(counts),
                events_per_game={k: v/len(games) for k, v in counts.items()},
                games_with_dna=dna_games,
                routes={k: dict(v, choice_pct=100*v["choices"]/v["opportunities"]
                                if v["opportunities"] else None) for k, v in routes.items()},
                nyaromon=dict(sequences, games_with_completed_dna=dna_sequence_games,
                             games_with_opportunity=dna_opportunity_games),
                steps=dict(mean=float(np.mean([g["steps"] for g in games])),
                           max=max(g["steps"] for g in games),
                           p95=float(np.quantile([g["steps"] for g in games], .95))))


def write_csv(path, rows):
    with path.open("w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


def render_outcomes(output, rows):
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    selected = [r for r in rows if r["panel"] == "combined_diagnostic" and r["archetype"] != "ALL"]
    names = sorted({r["archetype"] for r in selected})
    fig, axes = plt.subplots(1, 2, figsize=(12, 8), sharey=True)
    fig.patch.set_facecolor("#f7f8fb")
    titles = (("parent-checkpoint4", "Earlier campaign model · stronger anchor"),
              ("development-fixed", "Frozen development model"))
    for ax, (anchor, title) in zip(axes, titles):
        values = np.array([[next(r["win_pct"] for r in selected if r["anchor"] == anchor
                                 and r["archetype"] == name and r["checkpoint"] == cp)
                            for cp in (2, 3, 4)] for name in names])
        ax.imshow(values, cmap="YlGnBu", vmin=30, vmax=90, aspect="auto")
        ax.set_xticks(range(3), ["Checkpoint 2", "Checkpoint 3", "Checkpoint 4"])
        ax.set_yticks(range(len(names)), names)
        ax.tick_params(axis="both", length=0, pad=8)
        ax.set_title(title, fontsize=12, loc="left", pad=16, weight="bold")
        for y in range(len(names)):
            for x in range(3):
                ax.text(x, y, f"{values[y,x]:.1f}%", ha="center", va="center", fontsize=11,
                        color="white" if values[y,x] >= 68 else "#142338")
        ax.set_xticks(np.arange(-.5, 3), minor=True)
        ax.set_yticks(np.arange(-.5, len(names)), minor=True)
        ax.grid(which="minor", color="white", linewidth=2)
        ax.tick_params(which="minor", bottom=False, left=False)
        for spine in ax.spines.values():
            spine.set_visible(False)
    fig.suptitle("Did the two new updates improve deck piloting?", x=.03, ha="left", fontsize=19, weight="bold")
    fig.text(.03, .92, "Policy-only win rates · fixed EX13 diagnostic cohort · 200 games per cell", fontsize=12, color="#475569")
    fig.text(.03, .045, "Same seeds, lists and opponent weights across checkpoints. Per-deck results are exploratory.", fontsize=10, color="#475569")
    fig.text(.03, .022, "These compare model-plus-deck performance against fixed anchors; they are not a real-world meta ranking.", fontsize=10, color="#475569")
    fig.subplots_adjust(left=.23, right=.97, top=.84, bottom=.11, wspace=.12)
    fig.savefig(output / "archetype-comparison.png", dpi=180, facecolor=fig.get_facecolor())
    plt.close(fig)


def analyze(source, output):
    with gzip.open(source, "rt", encoding="utf-8") as f:
        reports = json.load(f)
    assert len(reports) == 6
    index = {(r["checkpoint"], r["panel"]): r for r in reports}
    panels = ("combined_diagnostic", "original_pet_regression")
    anchors = ("greedy", "parent-checkpoint4", "development-fixed")
    invariant_keys = ("runtime_cards_sha256", "observation_profile", "tensor_version",
                      "feature_schema_version", "tensor_size", "tensor_layout_hash",
                      "action_space_size", "action_schema_version", "evaluation_mode",
                      "panel", "card_eval_event_version", "base_seed", "games_per_archetype",
                      "requested_games_per_anchor", "cohort_scheme", "snapshot_sha256",
                      "anchors_sha256", "anchor_model_hashes", "deterministic", "status")
    outcomes, comparisons, behaviors, matchups = [], [], [], []
    for panel in panels:
        base = index[2, panel]["provenance"]
        for cp in (2, 3, 4):
            report = index[cp, panel]
            p = report["provenance"]
            assert all(p[k] == base[k] for k in invariant_keys)
            assert p["status"] == "complete" and p["evaluation_mode"] == "policy_only"
            assert p["checkpoint_index"] == cp
            assert p["candidate_sha256"] == p["checkpoint_sha256"]
            games = report["games"]
            assert len(games) == (7800 if panel == panels[0] else 3000)
            assert len({cohort_key(g) for g in games}) == len(games)
            assert set(g["terminal_reason"] for g in games) <= {"security_attack", "deck_out"}
            quotas = Counter((g["context"]["anchor"], g["context"]["agent_archetype"]) for g in games)
            assert set(quotas.values()) == {200}
            archetypes = sorted({g["context"]["agent_archetype"] for g in games})
            for anchor in (*anchors, "learned_only"):
                selected = [g for g in games if (g["context"]["anchor"] != "greedy"
                            if anchor == "learned_only" else g["context"]["anchor"] == anchor)]
                for archetype in ("ALL", *archetypes):
                    subset = [g for g in selected if archetype == "ALL" or g["context"]["agent_archetype"] == archetype]
                    wins = sum(g["winner_id"] == 1 for g in subset)
                    losses = sum(g["winner_id"] == 2 for g in subset)
                    outcomes.append(dict(checkpoint=cp, panel=panel, anchor=anchor,
                                         archetype=archetype, games=len(subset), wins=wins, losses=losses,
                                         draws=len(subset)-wins-losses, win_pct=100*wins/len(subset)))
                    behaviors.append(dict(checkpoint=cp, panel=panel, anchor=anchor,
                                          archetype=archetype, **behavior(subset)))
                if anchor != "learned_only":
                    cells = defaultdict(list)
                    for g in selected:
                        cells[g["context"]["agent_archetype"], g["context"]["opponent_archetype"]].append(g)
                    for (a, b), cell in sorted(cells.items()):
                        matchups.append(dict(checkpoint=cp, panel=panel, anchor=anchor,
                                             agent_archetype=a, opponent_archetype=b,
                                             games=len(cell), wins=sum(g["winner_id"] == 1 for g in cell),
                                             win_pct=100*np.mean([score(g) for g in cell])))
        for old, new in ((2, 3), (3, 4), (2, 4)):
            for anchor in anchors:
                a = [g for g in index[old, panel]["games"] if g["context"]["anchor"] == anchor]
                b = [g for g in index[new, panel]["games"] if g["context"]["anchor"] == anchor]
                for archetype in ("ALL", *archetypes):
                    aa = [g for g in a if archetype == "ALL" or g["context"]["agent_archetype"] == archetype]
                    bb = [g for g in b if archetype == "ALL" or g["context"]["agent_archetype"] == archetype]
                    comparisons.append(dict(old=old, new=new, panel=panel, anchor=anchor,
                                            archetype=archetype, **paired(aa, bb)))
    output.mkdir(parents=True, exist_ok=True)
    write_csv(output / "checkpoint-outcomes.csv", outcomes)
    write_csv(output / "paired-comparisons.csv", comparisons)
    write_csv(output / "matchups.csv", matchups)
    route_rows = [dict(checkpoint=b["checkpoint"], panel=b["panel"], anchor=b["anchor"],
                       archetype=b["archetype"], games=b["games"], route=route, **values)
                  for b in behaviors for route, values in b["routes"].items()]
    write_csv(output / "route-choice-rates.csv", route_rows)
    result = dict(method=dict(evaluation_mode="policy_only", bootstrap_samples=10000,
                              bootstrap_seed=19, bootstrap_unit="consecutive opening-order pair within anchor/archetype",
                              inference="paired checkpoint scores against fixed anchors; not head-to-head checkpoint play",
                              per_archetype_intervals="exploratory; unadjusted for multiple comparisons",
                              panels="separate; overlapping archetypes use different original-pet lists",
                              cp2_engine="reused evaluation, policy runtime equivalence validated in deployment receipt",
                              dna="EX12-001 opportunities deduplicated per physical egg/global turn; only Virus Busters interpreted"),
                  sources=[{k: r[k] for k in ("checkpoint", "panel", "provenance", "source", "source_sha256")} for r in reports],
                  outcomes=outcomes, paired_comparisons=comparisons, behavior=behaviors)
    (output / "analysis.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
    render_outcomes(output, outcomes)
    print(json.dumps(dict(reports=len(reports), games=sum(len(r["games"]) for r in reports),
                          validated=True, output=str(output))))
    print("HEADLINE OUTCOMES")
    for r in outcomes:
        if r["archetype"] == "ALL" and r["anchor"] != "learned_only":
            print(json.dumps(r))
    print("HEADLINE PAIRED COMPARISONS")
    for r in comparisons:
        if r["archetype"] == "ALL":
            print(json.dumps(r))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    analyze(args.source, args.output)
