"""Read-only search telemetry sidecar with durable backfill and optional W&B.

The experiment mount must be read-only. Sidecar state/spool lives elsewhere.
No engine, training, scheduler or network failures can alter gameplay here.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import time
import zipfile

from tools.selfplay_scheduler import atomic_json, controller_lock

STAGES = {name: i for i, name in enumerate(("pending", "input_validation", "batch_parity",
    "worker_scaling", "health_check", "generation", "pet_deck_evaluation", "complete", "failed"))}
EPOCH = re.compile(r"epoch (\d+): policy_loss ([\d.eE+-]+) value_loss ([\d.eE+-]+)")
ANCHOR = re.compile(r"vs\s+(\S+)\s+(\d+)-(\d+)-(\d+)\s+WR=\s*([\d.]+)%\s+\(n=(\d+)\)")


def read_json(path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (FileNotFoundError, json.JSONDecodeError):
        return None


def read_text(path):
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except FileNotFoundError:
        return ""


def recover_spool(path):
    """Repair only an interrupted final append; reject corruption in older rows."""
    if not path.exists():
        return []
    records = []
    offset = 0
    lines = path.read_bytes().splitlines(keepends=True)
    for index, line in enumerate(lines):
        try:
            record = json.loads(line)
        except (json.JSONDecodeError, UnicodeDecodeError):
            if index != len(lines) - 1:
                raise
            with path.open("r+b") as output:
                output.truncate(offset)
            break
        records.append(record)
        offset += len(line)
    if records and path.read_bytes()[-1:] != b"\n":
        with path.open("ab") as output:
            output.write(b"\n")
    return records


def active_generation(root):
    state = read_json(root / "experiment_state.json") or {}
    if "generation_index" in state:
        return int(state["generation_index"])
    return max((int(p.name[3:]) for p in (root / "generation").glob("gen*")
                if p.is_dir() and p.name[3:].isdigit()), default=0)


def collect(root, now):
    state = read_json(root / "experiment_state.json") or {"stage": "pending"}
    plan = read_json(root / "deployment-plan.json") or {}
    started = read_json(root / "started.json") or {}
    phase = state["stage"]
    gen = active_generation(root)
    gen_dir = root / "generation" / f"gen{gen}"
    metrics = {"monitor/unix_time": now, "pipeline/stage": phase,
        "pipeline/stage_code": STAGES.get(phase, -1), "pipeline/failed": int(phase == "failed"),
        "pipeline/complete": int(phase == "complete"), "pipeline/generation_index": gen,
        "pipeline/elapsed_hours": max(0, now - started.get("time_unix", now)) / 3600}
    metrics["pipeline/image"] = state.get("image", plan.get("image", "unknown"))
    queues = [("health160", root / "check-160/scheduler_progress.json"),
              ("health320", root / "check-320/scheduler_progress.json"),
              ("selfplay", gen_dir / "scheduler_progress.json")]
    for prefix, path in queues:
        progress = read_json(path)
        if not progress:
            continue
        for key in ("requested_games", "committed_games", "committed_rows", "truncated_games", "workers", "pending_chunks"):
            metrics[f"{prefix}/{key}"] = progress[key]
        games = progress["committed_games"]
        elapsed = progress["elapsed_seconds_this_invocation"]
        age = max(0, now - progress["heartbeat_unix"])
        running = progress["status"] == "running"
        metrics.update({f"{prefix}/status": progress["status"],
            f"{prefix}/active_workers": len(progress["active"]),
            f"{prefix}/heartbeat_age_seconds": age,
            f"{prefix}/heartbeat_stale": int(running and age > 120),
            f"{prefix}/fraction_complete": games / max(progress["requested_games"], 1),
            f"{prefix}/truncation_rate": progress["truncated_games"] / max(games, 1)})
        if elapsed > 0:
            # This experiment forbids implicit resume. These are current
            # invocation averages, including startup and uncommitted work.
            metrics[f"{prefix}/positions_per_second"] = progress["committed_rows"] / elapsed
            metrics[f"{prefix}/games_per_hour"] = games * 3600 / elapsed
            if games and running and age <= 120:
                metrics[f"{prefix}/eta_hours"] = max(0, progress["requested_games"] - games) * elapsed / games / 3600
        if prefix == "selfplay" and not running and progress["status"] == "complete" and phase == "generation":
            metrics["pipeline/substage"] = "fitting_or_evaluation"
    scaling = read_json(root / "diagnostics/scaling/benchmark.json") or {}
    grouped = {}
    for case in scaling.get("cases", []):
        grouped.setdefault(case["workers"], []).append(case["positions_generated_per_second"])
    for workers, rates in grouped.items():
        metrics[f"diagnostics/workers_{workers}_positions_per_second"] = sum(rates) / len(rates)
    if scaling:
        metrics["diagnostics/worker_target_parity"] = int(scaling["all_target_arrays_equal"])
    selected = read_json(root / "worker_selection.json")
    if selected:
        metrics["pipeline/selected_workers"] = selected["workers"]
    generation = read_json(gen_dir / "summary.json")
    if generation:
        for key, value in generation["timings_sec"].items():
            metrics[f"timings/{key}_seconds"] = value
    pet_path = root / "pet_anchored_eval.txt" if gen == 0 else gen_dir / "pet_anchored_eval.txt"
    for panel, path in (("meta_validation", gen_dir / "anchored_eval.txt"),
                        ("pet_same_list", pet_path)):
        for name, wins, losses, draws, wr, games in ANCHOR.findall(read_text(path)):
            for key, value in {"wins": int(wins), "losses": int(losses), "draws": int(draws),
                               "win_rate": float(wr) / 100, "games": int(games)}.items():
                metrics[f"evaluation/{panel}/{name}/{key}"] = value
    metrics["pipeline/target_games"] = plan.get("generation", {}).get("games", 0)
    if state.get("error"):
        metrics["pipeline/error"] = state["error"]
    return metrics


def epoch_events(root, log_path):
    # stdout is available during fitting; the final summary carries full precision.
    gen = active_generation(root)
    log = read_text(log_path)
    marker = f"=== GENERATION_MONITOR_INDEX={gen} ==="
    if marker in log:
        log = log.rsplit(marker, 1)[1]
    elif gen > 0:
        log = ""  # Previous invocation's stdout is not this generation's loss.
    found = {int(epoch): {"train/epoch": int(epoch), "train/policy_loss": float(policy),
             "train/value_loss": float(value), "train/source": "stdout"}
             for epoch, policy, value in EPOCH.findall(log)}
    summary = read_json(root / "generation" / f"gen{gen}" / "summary.json") or {}
    for row in summary.get("train", {}).get("epochs", []):
        found[row["epoch"]] = {"train/epoch": row["epoch"], "train/policy_loss": row["policy_loss"],
                               "train/value_loss": row["value_loss"], "train/source": "summary"}
    preceding_epochs = sum(len((read_json(root / 'generation' / f'gen{i}' / 'summary.json') or {})
                               .get('train', {}).get('epochs', [])) for i in range(gen))
    return [dict(event, **{"train/generation_index": gen,
                           "train/optimizer_epoch": preceding_epochs + event['train/epoch']})
            for event in found.values()]


def wandb_config(root):
    plan = read_json(root / "deployment-plan.json") or {}
    pool = read_json(root / "inputs/pool_report.json") or {}
    return {"generation": plan.get("generation"), "archetypes": pool.get("archetypes"),
        "distinct_training_lists": pool.get("distinct_training_lists"), "image": plan.get("image"),
        "image_source_commit": plan.get("image_source_commit"), "controller_commit": plan.get("controller_commit"),
        "input_hashes": plan.get("inputs"), "ci_run": plan.get("ci_run"),
        "evaluation_scope": "meta budget-validation panel and same-list pet panel; no independent promotion",
        "throughput_scope": "saved training positions; diagnostics logged separately",
        "automatic_promotion": False}


def file_signature(files):
    """Identity of a checkpoint file set by name, size and mtime (no reads)."""
    return hashlib.sha256(json.dumps(sorted(
        (p.name, p.stat().st_size, p.stat().st_mtime_ns) for p in files)).encode()).hexdigest()


def stable_checkpoint_files(root, now, stable_seconds=60, skip_signatures=()):
    directory = root / "generation" / f"gen{active_generation(root)}"
    model = directory / "model.zip"
    if not model.exists() or now - model.stat().st_mtime < stable_seconds:
        return []
    if file_signature([model]) in skip_signatures:
        return []
    try:
        with zipfile.ZipFile(model) as archive:
            if archive.testzip() is not None:
                return []
    except zipfile.BadZipFile:
        return []
    return [p for p in directory.iterdir() if p.is_file() and
            (p.name.startswith("model.") or p.name in ("summary.json", "anchored_eval.txt")) and
            now - p.stat().st_mtime >= stable_seconds]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("/experiment"))
    parser.add_argument("--state-dir", type=Path, default=Path("/monitor/state"))
    parser.add_argument("--training-log", type=Path, default=Path("/monitor/training.log"))
    parser.add_argument("--run-id", default="english-meta-pets-20261024")
    parser.add_argument("--entity", default=os.environ.get("WANDB_ENTITY"))
    parser.add_argument("--project", default=os.environ.get("WANDB_PROJECT"))
    parser.add_argument("--online", action="store_true", default=os.environ.get("WANDB_MODE") == "online")
    parser.add_argument("--once", action="store_true")
    parser.add_argument("--interval", type=int, default=30)
    parser.add_argument("--card-eval-root", type=Path, default=Path("/card-evaluation"))
    args = parser.parse_args()
    if args.online and not (args.entity and args.project):
        parser.error("Online logging requires an explicit W&B entity and project")
    args.state_dir.mkdir(parents=True, exist_ok=True)
    with controller_lock(args.state_dir / "monitor.lock"):
        state_path = args.state_dir / "publisher.json"
        local = read_json(state_path) or {"next_seq": 0, "epochs": {}, "uploaded": -1, "artifacts": []}
        spool = args.state_dir / "metrics.jsonl"
        records = recover_spool(spool)
        local["next_seq"] = max(local["next_seq"], max((r["seq"] + 1 for r in records), default=0))
        # Kept in memory: the spool is parsed once at startup, then read only
        # from the first not-yet-uploaded byte (`upload_offset`).
        queued = {r.get("card_report") for r in records}
        local.setdefault("upload_offset", 0)
        local.setdefault("model_signatures", [])
        if local["upload_offset"] > (spool.stat().st_size if spool.exists() else 0):
            local["upload_offset"] = 0
        run = None
        if args.online:
            import wandb
            run = wandb.init(entity=args.entity, project=args.project, id=args.run_id, resume="allow",
                name="English meta + pet decks | 3300 games/generation | 320/4", job_type="search-monitor",
                dir=str(args.state_dir), config=wandb_config(args.root),
                settings=wandb.Settings(disable_code=True, disable_git=True, x_disable_stats=True))
            run.define_metric("train/optimizer_epoch")
            run.define_metric("train/policy_loss", step_metric="train/optimizer_epoch")
            run.define_metric("train/value_loss", step_metric="train/optimizer_epoch")
            local["uploaded"] = max(local["uploaded"], run.step - 1)
            atomic_json(args.state_dir / "wandb_run.json", {"url": run.url, "id": run.id,
                "entity": args.entity, "project": args.project})
        print(json.dumps({"mode": "online" if run else "spooling", "run_id": args.run_id}), flush=True)
        while True:
            now = time.time()
            metrics = collect(args.root, now)
            card_status = read_json(args.card_eval_root / "status.json")
            if card_status:
                metrics["card_eval/worker_status"] = card_status["status"]
                metrics["card_eval/worker_failed"] = int(card_status["status"] == "failed" or
                    (card_status.get("last_result") or {}).get("status") == "failed")
            payloads = [metrics]
            for event in epoch_events(args.root, args.training_log):
                key = f"{event['train/generation_index']}:{event['train/epoch']}"
                if local["epochs"].get(key) != event:
                    payloads.append(event)
                    local["epochs"][key] = event
            with spool.open("a", encoding="utf-8") as out:
                for payload in payloads:
                    out.write(json.dumps({"seq": local["next_seq"], "payload": payload}) + "\n")
                    local["next_seq"] += 1
                for path in sorted(args.card_eval_root.glob("window-*/reports/checkpoint_*.json")):
                    if str(path) in queued:
                        continue
                    out.write(json.dumps({"seq": local["next_seq"], "card_report": str(path)}) + "\n")
                    queued.add(str(path))
                    local["next_seq"] += 1
                out.flush()
                os.fsync(out.fileno())
            atomic_json(args.state_dir / "latest_metrics.json", metrics)
            upload_ok = True
            if run:
                try:
                    with spool.open("rb") as pending:
                        pending.seek(local["upload_offset"])
                        for line in pending:
                            record = json.loads(line)
                            if record["seq"] > local["uploaded"]:
                                if "card_report" in record:
                                    from digimon_gym.agents.card_eval import publish_report
                                    path = Path(record["card_report"])
                                    report = read_json(path)
                                    seq = record["seq"]
                                    # A retry after a partial failure must not log
                                    # a second artifact version for the same report.
                                    if not report or not publish_report(report, path, run,
                                            step=seq, wait=True,
                                            skip_artifact=local.get("card_artifact_seq") == seq,
                                            on_artifact_logged=lambda: local.update(card_artifact_seq=seq)):
                                        raise RuntimeError(f"Card report not uploaded: {path.name}")
                                    local["card_reports_uploaded"] = local.get("card_reports_uploaded", 0) + 1
                                else:
                                    run.log(record["payload"], step=record["seq"])
                                local["uploaded"] = record["seq"]
                            local["upload_offset"] += len(line)
                    run.summary["pipeline_stage"] = metrics["pipeline/stage"]
                    run.summary["per_card_metrics_available"] = local.get("card_reports_uploaded", 0) > 0
                    run.summary["card_reports_uploaded"] = local.get("card_reports_uploaded", 0)
                    files = stable_checkpoint_files(args.root, now, skip_signatures=local["model_signatures"])
                    if files:
                        signature = file_signature(files)
                        if signature not in local["artifacts"]:
                            artifact = wandb.Artifact(f"checkpoint-{args.run_id}", type="model",
                                metadata={"generation": active_generation(args.root), "automatic_promotion": False})
                            for file in files:
                                artifact.add_file(str(file), name=file.name)
                            run.log_artifact(artifact, aliases=["latest", f"generation-{active_generation(args.root)}"]).wait()
                            local["artifacts"].append(signature)
                            local["model_signatures"].append(file_signature([args.root / "generation" / f"gen{active_generation(args.root)}" / "model.zip"]))
                except Exception as error:
                    upload_ok = False
                    print(f"W&B upload error; durable spool retained: {type(error).__name__}: {error}", flush=True)
            atomic_json(state_path, local)
            print(json.dumps({"stage": metrics["pipeline/stage"], "spooled_records": local["next_seq"],
                              "last_enqueued_wandb_record": local["uploaded"]}), flush=True)
            terminal = metrics["pipeline/stage"] in ("complete", "failed")
            cards_finished = not args.card_eval_root.exists() or (card_status or {}).get("status") == "stopped"
            if args.once or (terminal and cards_finished and upload_ok and now - (read_json(args.root / "experiment_state.json") or {}).get("time_unix", now) > 90):
                if run:
                    run.finish(exit_code=int(metrics["pipeline/stage"] == "failed"))
                return
            time.sleep(args.interval)


if __name__ == "__main__":
    main()
