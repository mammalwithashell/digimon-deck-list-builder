"""Prepare and analyze a telemetry replay without modifying the source run.

Gameplay uses final_policy_eval unchanged. A new image is explicitly recorded;
the frozen schedule is replayed, never counted as new independent evidence.
"""
import argparse
from collections import Counter, defaultdict
import csv
import html
import json
import os
from pathlib import Path
import shutil
import time

from tools.cloud_search import final_policy_eval
from tools.selfplay_scheduler import atomic_json, sha256


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def checked_source(source):
    plan = read(source / "plan.json")
    if read(source / "state.json")["stage"] != "complete":
        raise ValueError("Source evaluation must be complete")
    files = ["pool.json", "checkpoint3.zip", "checkpoint4.zip", "adjacent-schedule.json"]
    for name in files:
        if sha256(source / name) != plan["files"][name]:
            raise ValueError(f"Source input changed: {name}")
    schedule = read(source / "adjacent-schedule.json")
    outcomes = [game for shard in range(4)
                for game in read(source / "adjacent" / f"shard{shard}" / "outcomes.json")]
    result = final_policy_eval.summarize(schedule, outcomes, "adjacent")
    if result["games"] != 6760 or len({r["deck1"] for r in schedule}) != 13:
        raise ValueError("Expected the completed 13-deck adjacent comparison")
    return plan, files, outcomes


def prepare(source, root, source_commit, ci_run, rules_fix=False):
    import digimon_engine
    from sb3_contrib import MaskablePPO
    from model_contract import validate_checkpoint
    from digimon_gym.agents.gauntlet import GeneralistDeckPool

    image = os.environ["DIGIMON_DEPLOYED_IMAGE"]
    if "@sha256:" not in image or digimon_engine.CARD_EVAL_EVENT_VERSION < 2:
        raise ValueError("An immutable telemetry-enabled image is required")
    old, files, _ = checked_source(source)
    if sha256(Path("/app/data/cards.json")) != old["cards_sha256"]:
        raise ValueError("Card data changed from the original evaluation")
    if root.exists():
        raise ValueError("Refusing to overwrite an evaluation directory")
    pool = GeneralistDeckPool.from_snapshot(source / "pool.json",
        implemented_card_ids=digimon_engine.load_implemented_card_ids())
    if len(pool.archetypes) != 13 or any(len(v) != 1 for v in pool.archetypes.values()):
        raise ValueError("Expected 13 admitted frozen lists")
    for checkpoint in (3, 4):
        model = source / f"checkpoint{checkpoint}.zip"
        if validate_checkpoint(MaskablePPO.load(model, device="cpu")) != old["contract"]:
            raise ValueError("Model contract changed")
        if sha256(model) != old["models"][str(checkpoint)]:
            raise ValueError("Checkpoint hash mismatch")
    root.mkdir()
    for name in files:
        shutil.copy2(source / name, root / name)
        if sha256(root / name) != old["files"][name]:
            raise ValueError("Input changed during copy")
    plan = dict(old, image=image, source_commit=source_commit, ci_run=ci_run,
        files={name: old["files"][name] for name in files}, panels=["adjacent"],
        self_games=0, mirror_games=0, adjacent_games=6760, workers_per_panel=4,
        controller_sha256=sha256(Path(final_policy_eval.__file__)),
        preparation_sha256=sha256(Path(__file__)), card_eval_event_version=2,
        source_evaluation=str(source), source_image=old["image"],
        source_plan_sha256=sha256(source / "plan.json"),
        source_outcomes_sha256={str(i): sha256(source / "adjacent" / f"shard{i}" / "outcomes.json")
                               for i in range(4)},
        replay=True, independent_strength_sample=False,
        comparison_type="rules_fix" if rules_fix else "telemetry_only",
        rules_fix="EX12-001 attack requires this effect's successful DNA" if rules_fix else None,
        training=False, search=False, automatic_promotion=False)
    atomic_json(root / "plan.json", plan)
    print(json.dumps(plan), flush=True)


def behavior_rows(reports, expected):
    """Attribute each player to its actual model, including swapped model roles."""
    routes, effects, appearances = defaultdict(Counter), defaultdict(Counter), Counter()
    observed = set()
    unexplained_attacks = []
    for report in reports:
        if report.get("coverage", {}).get("effect_sequences", "unmeasured") == "unmeasured":
            raise ValueError("Missing native effect telemetry coverage")
        for game in report["games"]:
            context = game["context"]
            index = context["index"]
            if index in observed or index not in expected:
                raise ValueError("Duplicate or unexpected telemetry game")
            observed.add(index)
            outcome = expected[index]
            if any(context.get(k) != outcome[k] for k in
                   ("seed", "block", "deck1", "deck2", "model1", "model2")):
                raise ValueError("Telemetry differs from its scheduled game")
            if game["winner_id"] != outcome["winner"] or game["terminal_reason"] != outcome["terminal_reason"]:
                raise ValueError("Telemetry outcome mismatch")
            for player in (1, 2):
                appearances[context[f"model{player}"], context[f"deck{player}"]] += 1
            if "route_decisions" not in game or "effect_sequences" not in game:
                raise ValueError("Missing decision counters")
            for row in game["route_decisions"]:
                player = row["player"]
                key = context[f"model{player}"], context[f"deck{player}"], row["route"]
                routes[key].update({k: row[k] for k in ("opportunities", "choices")})
            for row in game["effect_sequences"]:
                player = row["player"]
                key = context[f"model{player}"], context[f"deck{player}"], row["card_id"]
                effects[key].update({k: v for k, v in row.items() if k not in ("player", "card_id")})
                if row["card_id"] == "EX12-001" and row.get("attacks_declared", 0):
                    dna = sum(card["dna_digivolves"] for card in game["cards"] if card["player"] == player)
                    if dna == 0:
                        unexplained_attacks.append(dict(index=index, seed=context["seed"],
                            player=player, checkpoint=context[f"model{player}"],
                            archetype=context[f"deck{player}"], declarations=row["attacks_declared"]))
    if observed != set(expected):
        raise ValueError("Missing telemetry games")
    def rows(groups, dimension):
        return [dict(checkpoint=model, archetype=deck, **{dimension: item},
                     appearances=appearances[model, deck], **counts)
                for (model, deck, item), counts in sorted(groups.items())]
    route_rows = rows(routes, "route")
    for row in route_rows:
        row["choice_rate"] = row["choices"] / row["opportunities"] if row["opportunities"] else None
    return dict(route_decisions=route_rows, effect_sequences=rows(effects, "card_id"),
                nyaromon_attacks_without_player_dna=unexplained_attacks,
                appearances=[dict(checkpoint=model, archetype=deck, games=n)
                             for (model, deck), n in sorted(appearances.items())])


def validate_comparison(plan, result):
    if result["changed_outcomes"] and plan.get("comparison_type", "telemetry_only") == "telemetry_only":
        raise ValueError("Replay outcomes changed: inspect before claiming observer equivalence")
    if plan.get("comparison_type") == "rules_fix" and result["nyaromon_attacks_without_player_dna"]:
        raise ValueError("Nyaromon attacks without DNA remain after the rules fix")


def analyze(source, root):
    plan = final_policy_eval.validate(root)
    if read(root / "state.json")["stage"] != "complete":
        raise ValueError("Replay evaluation is incomplete")
    if sha256(source / "plan.json") != plan["source_plan_sha256"]:
        raise ValueError("Source plan changed")
    for shard, digest in plan["source_outcomes_sha256"].items():
        if sha256(source / "adjacent" / f"shard{shard}" / "outcomes.json") != digest:
            raise ValueError("Source outcomes changed")
    _, _, old = checked_source(source)
    new = [g for shard in range(4) for g in read(root / "adjacent" / f"shard{shard}" / "outcomes.json")]
    final_policy_eval.summarize(read(root / "adjacent-schedule.json"), new, "adjacent")
    old = {g["index"]: g for g in old}
    expected = {g["index"]: g for g in new}
    differences = [dict(index=i, before=old[i], after=g) for i, g in expected.items() if g != old[i]]
    paths = [next((root / "adjacent" / f"shard{i}" / "reports").glob("checkpoint_*.json")) for i in range(4)]
    result = behavior_rows((read(path) for path in paths), expected)
    result.update(status="complete", games=len(new), changed_outcomes=len(differences),
                  analysis_sha256=sha256(Path(__file__)),
                  replay=True, independent_strength_sample=False, automatic_promotion=False,
                  interpretation="Choice rates are conditional on the states each model reaches, not causal skill estimates.")
    out = root / "report"
    out.mkdir(exist_ok=True)
    atomic_json(out / "decision-telemetry.json", result)
    atomic_json(out / "replay-outcome-differences.json", differences)
    for name in ("route_decisions", "effect_sequences", "appearances"):
        rows = result[name]
        if rows:
            with (out / f"{name}.csv").open("w", newline="", encoding="utf-8") as stream:
                writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
                writer.writeheader()
                writer.writerows(rows)
    print(json.dumps(result), flush=True)
    validate_comparison(plan, result)
    return result


def publish(source, root, run_id):
    from tools.cloud_search.final_policy_report import render, publish as publish_report
    result = analyze(source, root)
    plan = read(root / "plan.json")
    results, out = render(root)
    sections = ["<h2>Decision telemetry: checkpoint 3 versus 4</h2>"
                "<p>This replays the previous schedule with new telemetry. "
                "It is not another independent strength sample. "
                f"{result['changed_outcomes']} of 6,760 outcomes changed from the original evaluation. "
                "Rates are conditional on the states "
                "each policy reaches; they do not prove causal skill.</p>"]
    if plan.get("comparison_type") == "rules_fix":
        sections.append("<p>Nyaromon's DNA-success gate is fixed in this runtime. "
                        "Changed outcomes are expected and are retained in the difference report.</p>")
    if result["nyaromon_attacks_without_player_dna"]:
        count = len(result["nyaromon_attacks_without_player_dna"])
        sections.append(f"<p><strong>Rules-fidelity warning:</strong> {count} player-games "
            "contain Nyaromon attack declarations but no DNA evolution by that player. "
            "These are not successful DNA follow-ups. The card requires the DNA-digivolved "
            "Digimon; investigate this existing engine behavior before further training "
            "or interpreting Virus Busters performance as rules-faithful.</p>")
    for title, rows in [
        ("Raising evolution when available", [r for r in result["route_decisions"] if r["route"] == "digivolve_raising"]),
        ("Nyaromon DNA sequence", result["effect_sequences"]),
    ]:
        sections.append("<h3>" + title + "</h3>")
        if not rows:
            sections.append("<p>No observed opportunities in this cohort.</p>")
            continue
        columns = list(rows[0])
        sections.append("<div class=scroll><table><tr>" + "".join("<th>" + html.escape(c) + "</th>" for c in columns) + "</tr>")
        for row in rows:
            sections.append("<tr>" + "".join("<td>" + html.escape(str(row[c])) + "</td>" for c in columns) + "</tr>")
        sections.append("</table></div>")
    path = out / "index.html"
    path.write_text(path.read_text(encoding="utf-8").replace("</html>", "".join(sections) + "</html>"), encoding="utf-8")
    publish_report(root, results, out, run_id, run_name="Checkpoint 3 vs 4: " +
                   ("Nyaromon rules fix" if plan.get("comparison_type") == "rules_fix" else "evolution diagnostics"))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--prepare", action="store_true")
    parser.add_argument("--source-commit")
    parser.add_argument("--ci-run")
    parser.add_argument("--rules-fix", action="store_true")
    parser.add_argument("--publish", action="store_true")
    parser.add_argument("--wait", action="store_true")
    parser.add_argument("--run-id", default="decision-telemetry-eval-20261026")
    args = parser.parse_args()
    if args.prepare:
        prepare(args.source, args.root, args.source_commit, args.ci_run, args.rules_fix)
    elif args.publish:
        while args.wait:
            state = read(args.root / "state.json") if (args.root / "state.json").exists() else {"stage": "preparing"}
            if state["stage"] == "complete":
                break
            if state["stage"] == "failed":
                raise RuntimeError(f"Evaluation failed: {state}")
            time.sleep(10)
        publish(args.source, args.root, args.run_id)
    else:
        analyze(args.source, args.root)
