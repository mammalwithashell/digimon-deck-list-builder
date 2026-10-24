"""Periodic policy-only card evaluations, isolated from the search experiment.

Reads a cloud-search experiment; writes only its own output directory. No W&B
credentials or training controls belong in this process.
"""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys
import time
import zipfile

from tools.selfplay_scheduler import atomic_json, controller_lock, sha256


def read_json(path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (FileNotFoundError, json.JSONDecodeError):
        return {}


def checkpoints(root, now, stable_seconds=60):
    result = [(0, root / "inputs/seed.zip")]
    for directory in (root / "generation").glob("gen*"):
        if directory.name[3:].isdigit():
            result.append((int(directory.name[3:]) + 1, directory / "model.zip"))
    ready = []
    for step, path in sorted(result):
        if not path.exists() or now - path.stat().st_mtime < stable_seconds:
            continue
        try:
            with zipfile.ZipFile(path) as archive:
                if archive.testzip() is not None:
                    continue
        except (zipfile.BadZipFile, OSError):
            continue
        ready.append({"step": step, "path": str(path), "sha256": sha256(path)})
    return ready


def due_checkpoint(ready, state, now, interval, terminal=False):
    if not ready:
        return None
    attempted = state.get("attempted", [])
    # Baseline first, then every available checkpoint, even if fitting was fast.
    for checkpoint in ready:
        if checkpoint["sha256"] not in attempted:
            return checkpoint, "new_checkpoint"
    latest = ready[-1]
    if terminal and not state.get("final_attempted"):
        return latest, "final"
    if not terminal and now >= state.get("next_due_unix", 0):
        return latest, "interval"
    return None


def evaluate_window(root, out, checkpoint, window, reason, args):
    folder = out / f"window-{window:06d}"
    folder.mkdir(parents=True, exist_ok=False)
    frozen = folder / "candidate.zip"
    shutil.copyfile(checkpoint["path"], frozen)
    if sha256(frozen) != checkpoint["sha256"]:
        raise RuntimeError("Checkpoint changed during snapshot; evaluation refused")
    training_games = 0
    for path in (root / "generation").glob("gen*/scheduler_progress.json"):
        training_games += read_json(path).get("committed_games", 0)
    context = dict(checkpoint_sha256=checkpoint["sha256"], checkpoint_index=checkpoint["step"],
        training_games_collected=training_games, evaluation_started_unix=time.time(),
        trigger=reason, frozen_during_collection=True, training_algorithm="search_selfplay",
        comparison="fixed seeds and deck snapshots; repeated same-hash windows are not independent samples")
    atomic_json(folder / "context.json", context)
    failures = []
    for panel, snapshot in args.panels:
        try:
            command = [sys.executable, "-m", "tools.anchored_eval_cli", "--candidate", str(frozen),
                "--champions", str(root / "inputs/anchors.json"), "--n", str(args.games),
                "--seed", str(args.seed), "--device", "cpu", "--profile", args.profile,
                "--deck-pool-snapshot", str(root / "inputs" / snapshot),
                "--card-eval-dir", str(folder / "reports"), "--checkpoint-step", str(checkpoint["step"]),
                "--eval-window", str(window), "--eval-panel", panel,
                "--eval-context", str(folder / "context.json")]
            with (folder / f"{panel}.log").open("w", encoding="utf-8") as log:
                subprocess.run(command, stdout=log, stderr=subprocess.STDOUT,
                               timeout=args.timeout, check=True)
        except (subprocess.SubprocessError, OSError) as error:
            failures.append({"panel": panel, "error": str(error)})
    result = dict(status="failed" if failures else "complete", window=window,
        checkpoint=checkpoint, reason=reason, failures=failures, time_unix=time.time())
    atomic_json(folder / "result.json", result)
    # Reports carry the hash; duplicate checkpoint bytes need not accumulate.
    frozen.unlink()
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("/experiment"))
    parser.add_argument("--out", type=Path, default=Path("/card-evaluation"))
    parser.add_argument("--interval-seconds", type=int, default=7200)
    parser.add_argument("--games", type=int, default=24, help="even number of games per anchor per panel")
    parser.add_argument("--seed", type=int, default=28261024)
    parser.add_argument("--profile", default="standard_lite_v2")
    parser.add_argument("--timeout", type=int, default=1800, help="maximum seconds per panel")
    parser.add_argument("--panel", action="append", help="LABEL=SNAPSHOT_FILENAME under experiment inputs")
    parser.add_argument("--once", action="store_true")
    args = parser.parse_args()
    if args.games <= 0 or args.games % 2 or args.interval_seconds < 60 or args.timeout <= 0:
        parser.error("Use positive even --games, interval >= 60 seconds and positive timeout")
    args.panels = [p.split("=", 1) for p in (args.panel or [
        "meta_validation=heldout_snapshot.json", "pet_same_list=pet_evaluation_snapshot.json"])]
    for panel in args.panels:
        if len(panel) != 2 or not all(p and Path(p).name == p and "/" not in p and "\\" not in p for p in panel):
            parser.error("Panels must be LABEL=SNAPSHOT_FILENAME without directories")
    args.out.mkdir(parents=True, exist_ok=True)
    if args.out.resolve() == args.root.resolve() or args.root.resolve() in args.out.resolve().parents:
        parser.error("Evaluation output must be separate from the experiment")
    with controller_lock(args.out / "worker.lock"):
        state_path = args.out / "schedule.json"
        state = read_json(state_path)
        # Reserve window IDs before launching, so interrupted runs are retained.
        while True:
            now = time.time()
            pipeline = read_json(args.root / "experiment_state.json").get("stage")
            terminal = pipeline in ("complete", "failed")
            due = due_checkpoint(checkpoints(args.root, now), state, now, args.interval_seconds, terminal)
            if due:
                checkpoint, reason = due
                window = state.get("next_window", 0)
                state.update(next_window=window + 1, next_due_unix=now + args.interval_seconds)
                atomic_json(state_path, state)
                atomic_json(args.out / "status.json", dict(status="running", window=window,
                    checkpoint=checkpoint, reason=reason, time_unix=now))
                try:
                    result = evaluate_window(args.root, args.out, checkpoint, window, reason, args)
                except Exception as error:
                    result = dict(status="failed", window=window, error=str(error), time_unix=time.time())
                state.setdefault("attempted", []).append(checkpoint["sha256"])
                if reason == "final":
                    state["final_attempted"] = True
                state.update(last_result=result, next_due_unix=time.time() + args.interval_seconds)
                atomic_json(state_path, state)
                atomic_json(args.out / "status.json", dict(result, worker_status="waiting"))
                print(json.dumps(result), flush=True)
            elif terminal:
                atomic_json(args.out / "status.json", dict(status="stopped", pipeline=pipeline,
                    time_unix=now, last_result=state.get("last_result")))
                return
            if args.once:
                return
            time.sleep(30)


if __name__ == "__main__":
    main()
