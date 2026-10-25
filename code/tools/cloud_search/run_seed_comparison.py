"""Run isolated, matched bootstrap arms, then one search update per arm.

No promotion or automatic recovery. All corpora must qualify before any fitting.
The calling deployment must provide frozen inputs and a tested immutable image.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
from types import SimpleNamespace

from tools.cloud_search.mixed_bootstrap import ARMS, PROFILE, collect, fit, write_json
from tools.cloud_search.card_eval_worker import evaluate_window
from tools.selfplay_scheduler import controller_lock, sha256

INPUTS = ("decks.json", "train_snapshot.json", "heldout_snapshot.json", "pet_evaluation_snapshot.json")


def compare_reports(control, candidate, *, seed=19, samples=10000):
    """Paired bootstrap over consecutive opening-seat pairs. Candidate is P1
    in the evaluation wrapper; the opening player alternates by episode seed.
    """
    import numpy as np
    def indexed(games):
        result = {}
        for game in games:
            context = game["context"]
            key = (context["panel"], context["anchor"], context["agent_archetype"], game["seed"])
            if key in result or game["terminal_reason"] in ("truncated", "step_limit", "crash", "evaluation_end", "reset_before_terminal"):
                raise ValueError("duplicate or incomplete paired evaluation game")
            result[key] = game
        return result
    a, b = indexed(control), indexed(candidate)
    if set(a) != set(b) or not a:
        raise ValueError("evaluation cohorts differ or are empty")
    grouped = {}
    score = lambda g: 0.5 if g["winner_id"] in (None, 0) else float(g["winner_id"] == 1)
    for key in sorted(a):
        if any(a[key]["context"][field] != b[key]["context"][field]
               for field in ("opponent_archetype", "opponent_deck_id", "agent_deck_id")):
            raise ValueError("paired opponent deck cohort differs")
        grouped.setdefault(key[:3], []).append(score(b[key]) - score(a[key]))
    clusters, panels, archetypes = [], {}, {}
    for (panel, anchor, archetype), values in grouped.items():
        if len(values) % 2:
            raise ValueError("incomplete opening-seat pair")
        pairs = np.asarray(values).reshape(-1, 2).mean(axis=1)
        clusters.extend(pairs.tolist())
        panels.setdefault(panel, []).extend(pairs.tolist())
        archetypes.setdefault(archetype, []).extend(pairs.tolist())
    def estimate(values):
        rng = np.random.default_rng(seed)
        x = np.asarray(values)
        means = []
        for start in range(0, samples, 200):
            means.extend(x[rng.integers(len(x), size=(min(200, samples-start), len(x)))].mean(axis=1).tolist())
        lo, hi = np.quantile(means, [.025, .975])
        return dict(paired_games=len(x)*2, difference=float(x.mean()), ci95=[float(lo), float(hi)],
                    noninferior_at_five_points=bool(lo > -.05))
    return dict(aggregate=estimate(clusters), panels={k: estimate(v) for k,v in panels.items()},
        archetypes={k: estimate(v) for k,v in archetypes.items()},
        automatic_promotion=False, interpretation="Noninferiority is an outcome gate, not proof that extra evolution improves strength")


def read_games(directory):
    games = []
    for path in sorted((directory / "reports").glob("*.json")):
        games.extend(json.loads(path.read_text())["games"])
    if len(games) != 5200:
        raise ValueError(f"expected complete 5200-game cohort, got {len(games)}")
    return games


def run(root, *, seed, image):
    source = root / "inputs"
    if not image or "@sha256:" not in image:
        raise ValueError("immutable tested image required")
    plan = dict(seed=seed, image=image, arms=list(ARMS), bootstrap_games=416,
        search_games_per_arm=1100, simulations=320, worlds=4, evaluation_batch_size=4,
        evaluations_per_arm=2, games_per_evaluation=5200, automatic_restart=False, automatic_promotion=False,
        inputs={name: sha256(source / name) for name in INPUTS})
    with controller_lock(root / "comparison.lock"):
        with (root / "started.json").open("x") as f:
            json.dump(dict(time_unix=time.time(), plan=plan), f, indent=2)
        def stage(name, arm=None, **extra):
            write_json(root / "comparison_state.json", dict(stage=name, arm=arm, time_unix=time.time(), **extra))
            if arm:
                arm_stage(root / "arms" / arm, name, extra.get("checkpoint", 0), **extra)
        def arm_stage(path, name, index, **extra):
            write_json(path / "experiment_state.json", dict(stage=name, generation_index=index,
                time_unix=time.time(), **extra))
        try:
            for arm in ARMS:
                path = root / "arms" / arm
                write_json(path / "deployment-plan.json", dict(image=image, bootstrap_arm=arm, comparison=plan,
                    bootstrap_corpus=str(root / "corpora" / arm), inputs=plan["inputs"],
                    generation=dict(games=1100, generations=1, simulations=320, worlds=4, evaluation_batch_size=4, workers=8)))
                write_json(path / "started.json", dict(time_unix=time.time()))
                arm_stage(path, "awaiting_bootstrap", 0)
            for arm in ARMS:
                stage("collecting_bootstrap", arm)
                collect(source / "train_snapshot.json", root / "corpora" / arm, arm=arm, seed=seed)
            for arm in ARMS:
                stage("fitting_seed", arm)
                fit(root / "corpora" / arm, seed)
            from model_contract import validate_checkpoint
            from sb3_contrib import MaskablePPO
            anchor = root / "corpora/greedy/seed.zip"
            for arm in ARMS:
                path = root / "arms" / arm
                inputs, parent = path / "inputs", path / "generation/gen0"
                inputs.mkdir(parents=True, exist_ok=False)
                parent.mkdir(parents=True, exist_ok=False)
                for name in INPUTS:
                    shutil.copy2(source / name, inputs / name)
                for suffix in (".zip", ".onnx", ".onnx.meta.json", ".calibration.json"):
                    shutil.copy2(root / "corpora" / arm / f"seed{suffix}", inputs / f"seed{suffix}")
                    shutil.copy2(inputs / f"seed{suffix}", parent / f"model{suffix}")
                contract = validate_checkpoint(MaskablePPO.load(inputs / "seed.zip", device="cpu"), PROFILE)
                write_json(inputs / "anchors.json", dict(version=1, champions=[dict(name="frozen-greedy-bootstrap",
                    weights_path=str(anchor), algorithm="mlp", observation_profile=PROFILE,
                    tensor_layout_hash=contract["tensor_layout_hash"], source="common matched greedy-only control")]))
                write_json(path / "generation/state.json", dict(next_gen=1, prev=dict(
                    model_zip=str(parent / "model.zip"), model_onnx=str(parent / "model.onnx"))))
                arm_stage(path, "prepared", 0)
            for index in (0, 1):
                for arm in ARMS:
                    path = root / "arms" / arm
                    if index:
                        stage("search_update", arm)
                        arm_stage(path, "generation", 1)
                        command = [sys.executable, "-m", "tools.run_selfplay_generation", "--run-dir", str(path / "generation"),
                            "--seed-model", str(path / "inputs/seed.zip"), "--seed-model-onnx", str(path / "inputs/seed.onnx"),
                            "--decks", str(source / "decks.json"), "--cli-bin", "/usr/local/bin/digimon-engine-cli",
                            "--cards-json", "/app/data/cards.json", "--generations", "1", "--games", "1100", "--procs", "8",
                            "--chunk-games", "4", "--worker-timeout-seconds", "7200", "--sims", "320", "--worlds", "4",
                            "--eval-batch-size", "4", "--decision-cap", "1200", "--fail-on-truncation", "--window", "1",
                            "--epochs", "2", "--batch-size", "512", "--lr", "0.0001", "--train-device", "cpu",
                            "--seed", str(seed+1000000), "--skip-eval"]
                        with (path / "training.log").open("a", encoding="utf-8") as log:
                            log.write("=== GENERATION_MONITOR_INDEX=1 ===\n")
                            log.flush()
                            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True)
                    stage("checkpoint_evaluation", arm, checkpoint=index)
                    arm_stage(path, "checkpoint_evaluation", index)
                    checkpoint = path / "generation" / f"gen{index}" / "model.zip"
                    out = root / "evaluations" / arm
                    out.mkdir(parents=True, exist_ok=True)
                    result = evaluate_window(path, out, dict(path=str(checkpoint), sha256=sha256(checkpoint), step=index),
                        index, "new_checkpoint", SimpleNamespace(panels=[("meta_validation", "heldout_snapshot.json"),
                            ("pet_same_list", "pet_evaluation_snapshot.json")], games=200, games_per_archetype=200,
                            seed=28261025, profile=PROFILE, timeout=10800))
                    write_json(out / "status.json", dict(result, worker_status="waiting"))
                    if result["status"] != "complete":
                        raise RuntimeError(f"{arm} checkpoint {index} evaluation failed")
                    for panel, target in (("meta_validation", "anchored_eval.txt"), ("pet_same_list", "pet_anchored_eval.txt")):
                        shutil.copy2(out / f"window-{index:06d}" / f"{panel}.log", checkpoint.parent / target)
                    if index:
                        arm_stage(path, "complete", 1)
                        write_json(out / "status.json", dict(status="stopped", last_result=result))
                control = read_games(root / "evaluations/greedy" / f"window-{index:06d}")
                write_json(root / f"paired-checkpoint-{index}.json", {arm: compare_reports(control,
                    read_games(root / "evaluations" / arm / f"window-{index:06d}")) for arm in ARMS if arm != "greedy"})
            stage("complete", automatic_promotion=False)
        except BaseException as error:
            stage("failed", error=repr(error))
            for path in (root / "arms").glob("*"):
                state = json.loads((path / "experiment_state.json").read_text()) if (path / "experiment_state.json").exists() else {}
                if state.get("stage") != "complete":
                    arm_stage(path, "failed", state.get("generation_index", 0), error=repr(error))
                    write_json(root / "evaluations" / path.name / "status.json", dict(status="stopped", error=repr(error)))
            raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("/experiment"))
    parser.add_argument("--seed", type=int, default=60261025)
    args = parser.parse_args()
    run(args.root, seed=args.seed, image=os.environ.get("DIGIMON_DEPLOYED_IMAGE"))


if __name__ == "__main__":
    main()
