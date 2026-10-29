"""Fresh, fixed-size 640/4 vs 320/4 evaluation with main's card telemetry.

No trainer imports, model fitting, promotion or external publishing.
"""
from __future__ import annotations

import argparse
from collections import Counter, defaultdict
import copy
import csv
import json
from pathlib import Path
import random
import signal
import subprocess
from types import SimpleNamespace

from digimon_gym.agents.card_eval import CardEvalCollector, save_report
from tools.search_quality_study import Jobs, cfg, score_interval
from tools.selfplay_scheduler import atomic_json, controller_lock, sha256


def confirmation_jobs(plan):
    """One fresh seat pair per ordered deck pairing, including mirrors."""
    n = len(plan["decks"])
    return [(f"confirm-{i:02d}-{j:02d}", "match", {
        "candidate": cfg(640, 4), "opponent": cfg(320, 4), "card_metrics": True,
        "deck_indices": [i, j], "game_seed": plan["match_seed"] + i * n + j,
        "search_seed": plan["search_seed"] + i * n + j,
    }) for i in range(n) for j in range(n)]


def normalized_event(event, flip=False):
    """Match the relevant event_to_pydict fields from the Rust binding.

    Input uses Rust seats 0/1; output uses collector roles 1/2.
    Unknown non-card event kinds are unmeasured, not fabricated counters.
    """
    kind = event["type"]
    if kind not in {"Play", "Digivolve", "EvalDigivolve", "Hatch", "BreedingMove", "Attack"}:
        return None
    pid = event["player"]
    if pid not in (0, 1):
        raise ValueError("Invalid native player")
    key = "top_card_id" if kind in ("Digivolve", "EvalDigivolve") else "attacker_card_id" if kind == "Attack" else "card_id"
    name = "attacker_card_name" if kind == "Attack" else "card_name"
    result = {"type": kind, "player": (1 - pid if flip else pid) + 1,
              "source_card_id": event[key], "source_card_name": event[name], "meta": {}}
    if kind == "Play":
        result["meta"] = {k: event[k] for k in ("cost_paid", "effect_initiated")}
    if kind in ("Digivolve", "EvalDigivolve"):
        result["source_slot"] = event["field_index"]
        result["meta"] = {k: event[k] for k in ("memory_paid", "was_dna")}
    return result


def role_ui(ui, flip):
    return {"pendingSelection": ui["pendingSelection"],
            "player1": ui["player2" if flip else "player1"],
            "player2": ui["player1" if flip else "player2"]}


def collect_game(collector, plan, record, game, budget):
    """Role-align a copy for the dashboard: player1 always the named budget.

    Two reports describe the same real games from opposite roles. They must not
    be added together as independent games.
    """
    telemetry = game["card_metrics"]
    if telemetry["schema"] != "native-card-eval-v1" or telemetry["event_version"] != 1:
        raise ValueError("Unsupported native telemetry")
    if len(telemetry["steps"]) != game["decisions"]:
        raise ValueError("Incomplete decision telemetry")
    seat = game["candidate_seat"] if budget == 640 else 1 - game["candidate_seat"]
    flip = seat == 1
    decks = [plan["decks"][index] for index in record["deck_indices"]]
    collector.context = {"agent_archetype": decks[seat]["name"],
        "opponent_archetype": decks[1 - seat]["name"], "budget": f"s{budget}-w4",
        "opponent_budget": f"s{960-budget}-w4", "native_agent_seat": seat,
        "game_seed": record["game_seed"], "candidate_seat": game["candidate_seat"]}
    ui = role_ui(telemetry["initial_ui"], flip)
    collector.start_game(ui, (decks[seat]["cards"], decks[1-seat]["cards"]), record["game_seed"])
    for step in telemetry["steps"]:
        mask = [False] * 2192
        for action in step["legal_actions"]:
            mask[action] = True
        if not mask[step["action"]]:
            raise ValueError("Recorded choice is illegal")
        pid = 3 - step["mover"] if flip else step["mover"]
        collector.before_step({"current_player_id": pid, "phase": step["phase"]}, ui, mask, step["action"])
        ui = role_ui(step["after_ui"], flip)
        events = [converted for event in step["events"] if (converted := normalized_event(event, flip))]
        collector.after_step({"turn_count": step["turn_count"]}, ui, events, False, False)
    winner = game["winner"]
    winner = None if winner is None else (1 - winner if flip else winner) + 1
    # Incomplete games are excluded from entered-card outcome correlations.
    collector.finish_game(None if game["truncated"] else winner,
                          "evaluation_end" if game["truncated"] else game["terminal_reason"])
    collector.games[-1]["native_terminal_reason"] = game["terminal_reason"]
    collector.games[-1]["truncated"] = game["truncated"]


def summarize(plan, directory):
    records = [json.loads((directory / "results" / f"{name}.json").read_text())
               for name, _, _ in confirmation_jobs(plan)]
    pairs, blocks, games = [], [], []
    for record in records:
        pair = record["games"]
        if len(pair) != 2 or {g["candidate_seat"] for g in pair} != {0, 1}:
            raise ValueError("Invalid seat pair")
        if sum(g["candidate_seat"] == g["first_player"] for g in pair) != 1:
            raise ValueError("Pair is not first-player balanced")
        pairs.append(sum(g["score"] for g in pair) / 2)
        blocks.append(tuple(sorted(record["deck_indices"])))
        games.extend(pair)
    metrics = defaultdict(lambda: {"search_calls": 0, "selection_seconds": 0.0, "decisions": 0})
    for g in games:
        for step in g["card_metrics"]["steps"]:
            budget = "640/4" if step["mover"] - 1 == g["candidate_seat"] else "320/4"
            metrics[budget]["decisions"] += 1
            if len(step["legal_actions"]) > 1:
                metrics[budget]["search_calls"] += 1
                metrics[budget]["selection_seconds"] += step["selection_seconds"]
    return {"games": len(games), "seat_pairs": len(pairs), "matchup_blocks": len(set(blocks)),
        "score_640": sum(pairs)/len(pairs), "score_ci95": score_interval(pairs, blocks),
        "wins": sum(g["score"] == 1 for g in games), "losses": sum(g["score"] == 0 for g in games),
        "draws_or_caps": sum(g["score"] == .5 for g in games),
        "truncated": sum(g["truncated"] for g in games), "workload": dict(metrics),
        "first_player_score": sum(g["score"] for g in games if g["candidate_seat"] == g["first_player"])/len(pairs),
        "second_player_score": sum(g["score"] for g in games if g["candidate_seat"] != g["first_player"])/len(pairs),
        "inference": "fixed-size panel; approximate matchup-block bootstrap widened by pair-level fractional Wilson; no optional stopping or old-build pooling"}, records


def write_reports(plan, directory):
    summary, records = summarize(plan, directory)
    card_data = json.loads(Path(plan["cards_json"]).read_text(encoding="utf-8"))
    lookup = lambda card: SimpleNamespace(card_name=card_data.get(card, {}).get("card_name_eng", card))
    csv_rows = {"cards": [], "routes": []}
    totals = {}
    for budget in (640, 320):
        collector = CardEvalCollector(0, 0, lookup)
        collector.identity["window_id"] = f"budget-confirmation-{budget}-{sha256(directory/'plan.json')[:12]}"
        collector.provenance = {"study_plan_sha256": sha256(directory / "plan.json"),
            "budget": f"{budget}/4", "main_revision": plan["main_revision"],
            "event_version": 1, "role_mapping": "player1 is this budget; both budget reports reuse the same games",
            "information_regime": plan["regime"], "hashes": plan["hashes"],
            "model_contract": json.loads(Path(plan["model"] + ".meta.json").read_text())}
        for record in records:
            for game in record["games"]:
                collect_game(collector, plan, record, game, budget)
        report = collector.report()
        save_report(report, directory / "eval_cards" / f"s{budget}-w4")
        totals[str(budget)] = Counter()
        for kind in csv_rows:
            for row in report[kind]:
                if row["player"] == 1:
                    csv_rows[kind].append({"budget": budget, **row})
                    for key, value in row.items():
                        if isinstance(value, (float, int)) and key != "player":
                            totals[str(budget)][key] += value
    summary["card_totals_by_budget"] = dict(totals)
    atomic_json(directory / "summary.json", summary)
    for kind, rows in csv_rows.items():
        with (directory / f"{kind}.csv").open("w", newline="", encoding="utf-8") as out:
            writer = csv.DictWriter(out, fieldnames=list(rows[0]))
            writer.writeheader()
            writer.writerows(rows)
    lo, hi = summary["score_ci95"]
    old = json.loads((Path(plan["parent_study"]) / "evaluation_extra_summary.json").read_text())
    old_head = old["panels"]["s640-w4"]["s320-w4"]
    lines = ["# 640/4 versus 320/4: expanded post-main evaluation", "",
        f"{summary['games']} games / {summary['seat_pairs']} seat pairs / {summary['matchup_blocks']} unordered matchup blocks; "
        f"{summary['truncated']} truncated. Training remains stopped.", "",
        f"640/4 score: **{summary['score_640']:.1%}**, approximate 95% interval **{lo:.1%}–{hi:.1%}** "
        f"({summary['wins']} wins, {summary['losses']} losses, {summary['draws_or_caps']} draws/caps).", "",
        f"First-player score: {summary['first_player_score']:.1%}; second-player score: {summary['second_player_score']:.1%}.", "",
        f"Earlier engine panel: {old_head['mean_score']:.1%} across {old_head['games']} games "
        f"({old_head['score_ci95'][0]:.1%}–{old_head['score_ci95'][1]:.1%}). "
        "The earlier and post-main populations are reported separately, not pooled.", "",
        "## Workload", "", "| Budget | Decisions | Non-forced searches | Selection seconds/search |",
        "|---|---:|---:|---:|"]
    for budget, row in summary["workload"].items():
        lines.append(f"| {budget} | {row['decisions']} | {row['search_calls']} | {row['selection_seconds']/row['search_calls']:.3f} |")
    lines += ["", "## Per-card metrics", "",
        "The dashboard-compatible reports under eval_cards/s640-w4 and eval_cards/s320-w4 role-align each budget "
        "to player1. They describe the same real games twice from opposite roles; do not add their game counts. "
        "cards.csv and routes.csv include only each budget's own player1 rows, preserving matchup/card grouping.", "",
        "| Counter | 640/4 | 320/4 |", "|---|---:|---:|"]
    for key in ("plays_direct", "plays_effect", "hatches", "promotions", "digivolves_raising", "digivolves_battle", "dna_digivolves", "attacks", "memory_paid", "opportunities", "choices"):
        lines.append(f"| {key} | {totals['640'][key]} | {totals['320'][key]} |")
    lines += ["", "Opportunity means a legal decision exposure, deduplicated by card/route per decision. "
        "Pending selections are excluded because action IDs are reused. Choice counts are independent of resolved entries. "
        "Entry routes can overlap (raising evolution then promotion); missing opportunity routes are unmeasured. "
        "Win-when-entered is correlation, not causal card strength. Hand-seen is not a draw counter. "
        "Only actual game steps contribute; search rollouts do not.", "",
        "Scope: eight held-out English lists, all 64 ordered deck pairings including mirrors, exact-composition open-decklist "
        "PIMC, fixed seed network, batch four, zero root noise, no depth/quality changes. "
        "Intervals are approximate and conditional on this pool. New engines/models or other meta lists may differ. "
        "This head-to-head follow-up adds no new greedy/policy anchors and does not independently satisfy a model-promotion gate. "
        "Timing is from the Windows development host, not a Linux cloud sizing result.", ""]
    (directory / "report.md").write_text("\n".join(lines), encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", type=Path, required=True)
    parser.add_argument("--parent-study", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--resume", action="store_true")
    parser.add_argument("--analyze-only", action="store_true")
    args = parser.parse_args()
    base = json.loads((args.parent_study / "plan.json").read_text())
    plan = copy.deepcopy(base)
    plan.update(schema="search-confirmation-v1", engine=str(args.engine.resolve()),
        parent_study=str(args.parent_study.resolve()),
        main_revision=subprocess.check_output(["git", "rev-parse", "origin/main"], text=True).strip(),
        merge_revision=subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(), candidate=cfg(640, 4), opponent=cfg(320, 4),
        match_seed=2026102600, search_seed=2026102700,
        protocol="all ordered pairs once; fixed 64 seat pairs; no old-build pooling; canonical unordered matchup bootstrap; actual-game card metrics only")
    plan["hashes"]["engine"] = sha256(args.engine)
    for name, path in {"collector": Path("code/digimon_gym/agents/card_eval.py"),
                       "confirmation_tool": Path(__file__),
                       "job_controller": Path("code/tools/search_quality_study.py"),
                       "scheduler": Path("code/tools/selfplay_scheduler.py"),
                       "parent_plan": args.parent_study / "plan.json"}.items():
        plan["hashes"][name] = sha256(path)
    # Revalidate inherited assets as well as the existing stop-state.
    model, cards = Path(plan["model"]), Path(plan["cards_json"])
    paths = {"model": model, "metadata": Path(str(model)+".meta.json"),
        "snapshot": model.parent / "heldout_snapshot.json", "cards": cards,
        "mechanics": cards.parent / "mechanics_readiness.json", "admission": cards.parent / "card_admissibility.json"}
    for key, path in paths.items():
        if sha256(path) != base["hashes"][key]:
            raise ValueError(f"Changed parent asset: {key}")
    state = json.loads(Path(plan["training_state"]).read_text())
    if state["status"] != "stopped_by_user" or state["auto_resume"] is not False:
        raise ValueError("Training must remain stopped")
    if args.workers < 1:
        parser.error("workers must be positive")
    if args.out.exists() and not args.resume:
        raise FileExistsError("Use explicit --resume")
    args.out.mkdir(parents=True, exist_ok=True)
    with controller_lock(args.out / "study.lock"):
        path = args.out / "plan.json"
        if path.exists() and json.loads(path.read_text()) != plan:
            raise ValueError("Frozen confirmation plan changed")
        atomic_json(path, plan)
        work = confirmation_jobs(plan)
        random.Random(6729).shuffle(work)
        if not args.analyze_only:
            Jobs(plan, args.out, args.workers, 3600).run(work)
        else:
            for name, _, _ in work:
                committed = json.loads((args.out / "results" / f"{name}.done.json").read_text())
                if (committed["output_sha256"] != sha256(args.out / "results" / f"{name}.json")
                    or committed["request_sha256"] != sha256(args.out / "requests" / f"{name}.json")):
                    raise ValueError("Changed result")
        write_reports(plan, args.out)
    print("Confirmation complete; training remains stopped", flush=True)


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, lambda *_: (_ for _ in ()).throw(KeyboardInterrupt()))
    main()
