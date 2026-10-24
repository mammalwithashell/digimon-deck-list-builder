"""Fail-closed launch sequence for one approved CPU search experiment.

Mounted deployment controller; engine, trainer and dependencies come from the
CI image. Diagnostics and health checks are never generation training data.
"""
from pathlib import Path
import hashlib
import json
import os
import subprocess
import sys
import time

import digimon_engine
from digimon_gym.agents.gauntlet import GeneralistDeckPool, load_playable_card_ids
from model_contract import validate_contract, validate_checkpoint
from tools.run_selfplay_generation import run_selfplay_stage
from tools.selfplay_scheduler import atomic_json

ROOT = Path("/experiment")
INPUTS = ROOT / "inputs"
APP = Path("/app")
CLI = "/usr/local/bin/digimon-engine-cli"


def stage(name, **details):
    atomic_json(ROOT / "experiment_state.json", {"stage": name, "time_unix": time.time(), **details})
    print(json.dumps({"stage": name, **details}), flush=True)


def run(*arguments):
    subprocess.run([sys.executable, *map(str, arguments)], check=True)


def main():
    if (ROOT / "started.json").exists():
        raise RuntimeError("Experiment already started; investigate and resume explicitly")
    atomic_json(ROOT / "started.json", {"time_unix": time.time()})
    stage("input_validation")
    plan = json.loads((ROOT / "deployment-plan.json").read_text())
    for name, digest in plan["inputs"].items():
        actual = hashlib.sha256((INPUTS / name).read_bytes()).hexdigest()
        if actual != digest:
            raise ValueError(f"Input hash mismatch: {name}")
    pool = GeneralistDeckPool.from_snapshot(INPUTS / "train_snapshot.json", implemented_card_ids=load_playable_card_ids())
    for decks in pool.archetypes.values():
        for deck in decks:
            if not digimon_engine.validate_deck(deck.card_ids).is_valid:
                raise ValueError(f"Illegal deck: {deck.deck_id}")
    meta = json.loads((INPUTS / "seed.onnx.meta.json").read_text())
    validate_contract(meta, "standard_lite_v2")
    if meta.get("value_semantics") != "terminal_outcome":
        raise ValueError("Seed lacks terminal outcome value semantics")
    from sb3_contrib import MaskablePPO
    validate_checkpoint(MaskablePPO.load(INPUTS / "seed.zip", device="cpu"), "standard_lite_v2")
    diag = ROOT / "diagnostics"
    common = ["--cli", CLI, "--model", INPUTS / "seed.onnx", "--decks", INPUTS / "decks.json", "--cards-json", APP / "data/cards.json"]
    stage("batch_parity")
    run(APP / "code/tools/benchmark_search_batching.py", *common, "--out", diag / "batch-parity", "--seeds", "24261024", "24261025")
    stage("worker_scaling")
    run(APP / "code/tools/benchmark_search_workers.py", "scale", *common, "--out", diag / "scaling", "--workers", "2", "4", "6", "8", "--games", "16", "--repeats", "2", "--chunk-games", "2", "--decision-cap", "40")
    report = json.loads((diag / "scaling/benchmark.json").read_text())
    if not report["all_target_arrays_equal"]:
        raise ValueError("Worker-count target parity failed")
    throughput = {}
    for case in report["cases"]:
        throughput.setdefault(case["workers"], []).append(case["positions_generated_per_second"])
    workers = max(throughput, key=lambda n: sum(throughput[n]) / len(throughput[n]))
    atomic_json(ROOT / "worker_selection.json", {"workers": workers, "prefix_positions_per_second": throughput, "complete_game_eta_pending": True})
    stage("health_check", workers=workers)
    health = []
    for sims in (160, 320):
        folder = ROOT / f"check-{sims}"
        started = time.monotonic()
        directories = run_selfplay_stage(CLI, folder, str(INPUTS / "seed.onnx"), False,
            INPUTS / "decks.json", 100, workers, sims, 4, "pimc", 25261024, 16, 1200, 4096,
            APP / "data/cards.json", "implemented", eval_batch_size=4, chunk_games=4,
            worker_timeout_seconds=7200)
        manifests = [json.loads((p / "manifest.json").read_text()) for p in directories]
        elapsed = time.monotonic() - started
        summary = {"simulations": sims, "games": sum(m["games"] for m in manifests),
            "truncated_games": sum(m["truncated_games"] for m in manifests),
            "rows": sum(m["rows"] for m in manifests), "elapsed_seconds": elapsed,
            "games_per_hour": sum(m["games"] for m in manifests) * 3600 / elapsed}
        atomic_json(folder / "summary.json", summary)
        health.append(summary)
        if summary["games"] != 100 or summary["truncated_games"] > 5:
            raise RuntimeError(f"Expanded-pool health gate failed: {summary}")
    atomic_json(ROOT / "health_summary.json", {"runs": health, "games": 200,
        "generation_eta_hours": plan["generation"]["games"] / health[-1]["games_per_hour"]})
    stage("generation", workers=workers, games=plan["generation"]["games"])
    run(APP / "code/tools/run_selfplay_generation.py",
        "--run-dir", ROOT / "generation", "--seed-model", INPUTS / "seed.zip",
        "--seed-model-onnx", INPUTS / "seed.onnx", "--decks", INPUTS / "decks.json",
        "--cli-bin", CLI, "--cards-json", APP / "data/cards.json",
        "--generations", "1", "--games", str(plan["generation"]["games"]),
        "--procs", str(workers), "--chunk-games", "4", "--worker-timeout-seconds", "7200",
        "--sims", "320", "--worlds", "4", "--eval-batch-size", "4", "--decision-cap", "1200",
        "--epochs", "2", "--batch-size", "512", "--lr", "0.0001", "--train-device", "cpu",
        "--eval-n", "300", "--deck-pool-snapshot", INPUTS / "heldout_snapshot.json",
        "--champions", INPUTS / "anchors.json", "--seed", "26261024")
    stage("pet_deck_evaluation")
    with (ROOT / "pet_anchored_eval.txt").open("w") as log:
        subprocess.run([sys.executable, str(APP / "code/tools/anchored_eval_cli.py"),
            "--candidate", str(ROOT / "generation/gen0/model.zip"), "--champions", str(INPUTS / "anchors.json"),
            "--n", "300", "--seed", "27261024", "--device", "cpu", "--profile", "standard_lite_v2",
            "--deck-pool-snapshot", str(INPUTS / "pet_evaluation_snapshot.json")],
            stdout=log, stderr=subprocess.STDOUT, check=True)
    stage("complete", automatic_promotion=False)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        stage("failed", error=str(error), automatic_restart=False)
        raise
