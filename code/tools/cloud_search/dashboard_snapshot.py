"""Read-only, stdlib-only experiment summary; safe to run over SSH stdin.

Large immutable reports are reduced once, then reused by file signature.
Only canonical evaluation windows are included, never resume archives.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import time

INCOMPLETE = {"evaluation_end", "reset_before_terminal", "step_limit", "truncated", "crash"}


def read(path, default=None):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return default


def scheduler_progress(generation):
    progress = read(generation / "scheduler_progress.json", {})
    if not progress:
        return progress
    # Total committed games can include an earlier invocation. Measure only
    # commits during this invocation, using the scheduler's own event times.
    start = progress.get("heartbeat_unix", 0) - progress.get("elapsed_seconds_this_invocation", 0)
    try:
        events = [json.loads(line) for line in (generation / "scheduler_events.jsonl").read_text().splitlines() if line]
        progress["measured_games_this_invocation"] = sum(
            event["games"] for event in events
            if event.get("event") == "chunk_committed" and start <= event.get("unix", 0) <= progress["heartbeat_unix"])
    except (OSError, ValueError, KeyError):
        progress["measured_games_this_invocation"] = None
    return progress


def summarize_report(report, identity, arm, signature, dna_lists=None):
    outcomes = {}
    dna = {}
    for game in report.get("games", []):
        context = game.get("context", {})
        key = (context.get("anchor", "unknown"), context.get("agent_archetype", "unknown"))
        row = outcomes.setdefault(key, dict(anchor=key[0], archetype=key[1], wins=0,
                                            losses=0, draws=0, incomplete=0, decisions=0))
        reason, winner = game.get("terminal_reason"), game.get("winner_id")
        outcome = "incomplete" if reason in INCOMPLETE or not reason else (
            "draws" if winner in (None, 0) else "wins" if winner == 1 else "losses")
        row[outcome] += 1
        row["decisions"] += game.get("steps", 0)
        deck = context.get("agent_deck_id")
        if deck in (dna_lists or {}) and outcome != "incomplete":
            eligible = dna.setdefault((key, deck), dict(anchor=key[0], archetype=key[1], deck_id=deck,
                games=0, opportunities=0, choices=0, entries=0, targets=dna_lists[deck]))
            eligible["games"] += 1
            for route in game.get("routes", []):
                if route.get("player") == 1 and route.get("route") == "dna_digivolve":
                    eligible["opportunities"] += route["opportunities"]
                    eligible["choices"] += route["choices"]
            eligible["entries"] += sum(c.get("dna_digivolves", 0) for c in game.get("cards", []) if c.get("player") == 1)
    provenance = report.get("provenance", {})
    return dict(id=identity, signature=signature, arm=arm, window_id=report.get("window_id", identity),
                checkpoint=report.get("checkpoint_step", 0), panel=provenance.get("panel", "unknown"),
                status=provenance.get("status", "unknown"), provenance=provenance,
                outcomes=list(outcomes.values()), dna=list(dna.values()), coverage=report.get("coverage", {}),
                cards=[r for r in report.get("cards", []) if r.get("player") == 1],
                routes=[r for r in report.get("routes", []) if r.get("player") == 1])


def collect(root, known=None, now=None, dna_catalog=None):
    root = Path(root)
    known, now = known or {}, time.time() if now is None else now
    if not (root / "started.json").exists():
        raise ValueError("Not an experiment root: started.json is missing")
    started = read(root / "started.json", {})
    plan = started.get("plan", read(root / "deployment-plan.json", {}))
    state = read(root / "comparison_state.json", read(root / "experiment_state.json", {}))
    arms = sorted(p for p in (root / "arms").glob("*") if p.is_dir()) if (root / "arms").exists() else [root]
    result = dict(schema_version=1, collected_at=now, root=str(root), state=state, plan=plan,
                  arms=[], report_index={}, reports=[], comparisons={}, errors=[])
    for path in arms:
        name = path.name if path != root else "pilot"
        bootstrap_provenance = read(root / "corpora" / name / "provenance.json", {})
        dna_lists = dict(bootstrap_provenance.get("dna_capable_lists", {}))
        catalog_valid = bool(dna_catalog and bootstrap_provenance.get("cards_sha256") in
                             dna_catalog.get("cards_sha256_candidates", [dna_catalog.get("cards_sha256")]))
        if catalog_valid:
            dna_ids = set(dna_catalog["card_ids"])
            for filename in ("train_snapshot.json", "heldout_snapshot.json", "pet_evaluation_snapshot.json"):
                for deck in read(root / "inputs" / filename, {}).get("decks", []):
                    targets = {cid: count for cid, count in deck.get("card_counts", {}).items() if cid in dna_ids}
                    if targets:
                        dna_lists[deck["deck_id"]] = targets
        evaluation = root / "evaluations" / name if path != root else root / "evaluations"
        windows = []
        for window in sorted(evaluation.glob("window-*")):
            if not window.is_dir():
                continue
            windows.append(dict(name=window.name, context=read(window / "context.json", {}),
                result=read(window / "result.json", {}),
                progress=[read(p, {}) for p in sorted(window.glob("*-progress.json"))]))
            for file in sorted((window / "reports").glob("checkpoint_*.json")):
                identity = file.relative_to(root).as_posix()
                stat = file.stat()
                signature = [stat.st_size, stat.st_mtime_ns]
                result["report_index"][identity] = signature
                if known.get(identity) == signature:
                    continue
                try:
                    report = json.loads(file.read_text(encoding="utf-8"))
                    if report.get("schema_version") != 1:
                        raise ValueError("unsupported card report schema")
                    result["reports"].append(summarize_report(report, identity, name, signature, dna_lists))
                except (OSError, ValueError, KeyError, TypeError) as error:
                    result["errors"].append(dict(path=identity, error=str(error)))
        generations = []
        for generation in sorted((path / "generation").glob("gen*")):
            if not generation.is_dir() or not generation.name[3:].isdigit():
                continue
            model = generation / "model.zip"
            generations.append(dict(index=int(generation.name[3:]),
                model_saved_at=model.stat().st_mtime if model.exists() else None,
                progress=scheduler_progress(generation),
                coverage=read(generation / "archetype_coverage.json", {})))
        monitor = root / "monitors" / name / "state"
        publisher = monitor / "publisher.json"
        result["arms"].append(dict(name=name, state=read(path / "experiment_state.json", {}),
            deployment=read(path / "deployment-plan.json", {}), generations=generations, windows=windows,
            wandb=read(monitor / "wandb_run.json", {}),
            publisher_at=publisher.stat().st_mtime if publisher.exists() else None,
            dna_classification="all frozen lists" if catalog_valid else "bootstrap lists only; held-out eligibility unmeasured",
            bootstrap=read(root / "corpora" / name / "progress.json", {})))
    for file in sorted(root.glob("paired-checkpoint-*.json")):
        result["comparisons"][file.stem.removeprefix("paired-checkpoint-")] = read(file, {})
    result["head_to_head"] = read(root / "head_to_head_result.json", {})
    return result


def merge_snapshot(previous, incoming):
    """Drop archived/deleted reports; never retain changed-but-unreadable data."""
    cache = {r["id"]: r for r in (previous or {}).get("reports", [])}
    cache.update({r["id"]: r for r in incoming["reports"]})
    merged = dict(incoming)
    merged["reports"] = [cache[key] for key, sig in incoming["report_index"].items()
                         if key in cache and cache[key]["signature"] == sig]
    return merged


def search_progress(progress, now):
    completed = progress.get("committed_games", 0)
    total = progress.get("requested_games", 0)
    elapsed = progress.get("elapsed_seconds_this_invocation", 0)
    age = max(0, now - progress.get("heartbeat_unix", 0))
    fresh = age <= 120
    measured = progress.get("measured_games_this_invocation")
    rate = measured * 3600 / elapsed if elapsed > 0 and measured else None
    eta = ((total - completed) * 3600 / rate if rate and fresh and
           progress.get("status") == "running" and measured >= 8 else None)
    return dict(completed=completed, total=total, games_per_hour=rate, heartbeat_age=age,
                stale=not fresh and progress.get("status") == "running",
                eta_seconds=max(0, eta) if eta is not None else None)


def latest_reports(reports, include_incomplete=False):
    """One window per arm/checkpoint/panel, so resumed samples aren't pooled."""
    latest = {}
    for report in reports:
        if report.get("status") != "complete" and not include_incomplete:
            continue
        key = (report["arm"], report["checkpoint"], report["panel"])
        previous = latest.get(key)
        timestamp = report.get("provenance", {}).get("evaluation_started_unix", 0)
        if previous is None or timestamp >= previous.get("provenance", {}).get("evaluation_started_unix", 0):
            latest[key] = report
    return list(latest.values())


def compact_status(snapshot, now=None):
    now = time.time() if now is None else now
    age = max(0, now - snapshot["collected_at"])
    reports = latest_reports(snapshot["reports"])
    arms = []
    for arm in snapshot["arms"]:
        generations = arm.get("generations", [])
        current = next((g for g in generations if g["index"] == arm["state"].get("generation_index", 0)), {})
        arms.append(dict(name=arm["name"], state=arm["state"], wandb=arm.get("wandb", {}),
            search=search_progress(current["progress"], now) if current.get("progress") else None,
            saved_checkpoints=[g["index"] for g in generations if g["model_saved_at"]]))
    results = []
    for report in reports:
        anchors = {}
        for row in report["outcomes"]:
            values = anchors.setdefault(row["anchor"], dict(wins=0, losses=0, draws=0, incomplete=0))
            for key in values:
                values[key] += row[key]
        for anchor, counts in anchors.items():
            total = counts["wins"] + counts["losses"] + counts["draws"]
            results.append(dict(arm=report["arm"], checkpoint=report["checkpoint"], panel=report["panel"],
                anchor=anchor, **counts, completed_games=total,
                win_rate=counts["wins"]/total if total else None))
    return dict(collected_at=snapshot["collected_at"], source_age_seconds=age, fresh=age <= 150,
                state=snapshot["state"], arms=arms, results=results,
                head_to_head={k:v for k,v in snapshot.get("head_to_head", {}).items() if k != "matrix"},
                paired={step:{arm:value.get("aggregate") for arm,value in comparisons.items()}
                        for step,comparisons in snapshot.get("comparisons", {}).items()}, errors=snapshot.get("errors", []))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--root", type=Path)
    source.add_argument("--cache", type=Path)
    parser.add_argument("--summary", action="store_true")
    parser.add_argument("--known-reports", default="{}")
    parser.add_argument("--dna-catalog", default="{}")
    args = parser.parse_args()
    snapshot = read(args.cache / "snapshot.json") if args.cache else collect(args.root, json.loads(args.known_reports), dna_catalog=json.loads(args.dna_catalog))
    if not snapshot:
        parser.error("No snapshot available")
    print(json.dumps(compact_status(snapshot) if args.summary else snapshot, separators=(",", ":")))


if __name__ == "__main__":
    main()
