"""Run the frozen open/closed-list comparison on existing local hardware.

No resource provisioning, model promotion, or tuning against held-out results.
Artifacts are resumable and each completed job binds its exact input hashes.
"""

from __future__ import annotations
import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from digimon_gym.agents.belief_evaluation import promotion, fingerprint
from tools.selfplay_scheduler import atomic_json, controller_lock, run_queue

ROOT = Path(__file__).resolve().parents[2]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def native_record(manifest):
    rows = manifest.get("opponent_games", [])
    if not rows or len(rows) != manifest["games"]:
        raise ValueError("native evaluation is missing per-game opponent records")
    result = {
        "wins": 0,
        "losses": 0,
        "draws": 0,
        "failed": 0,
        "truncated": 0,
        "games": len(rows),
        "first_seat_games": 0,
        "second_seat_games": 0,
    }
    for r in rows:
        if r["learner_seat"] is None:
            raise ValueError("evaluation unexpectedly used mirror self-play")
        result[
            (
                "first_seat_games"
                if r["learner_seat"] == r["first_player"]
                else "second_seat_games"
            )
        ] += 1
        if r["truncated"]:
            result["truncated"] += 1
        elif r["winner"] is None:
            result["draws"] += 1
        else:
            result["wins" if r["winner"] == r["learner_seat"] else "losses"] += 1
    return result


def run(args):
    import torch

    torch.set_num_threads(1)
    out = args.out
    out.mkdir(parents=True, exist_ok=True)
    experiment = args.experiment
    protocol = read(experiment / "protocol.json")
    assets = {
        k: sha(experiment / f"{k}.json")
        for k in ("protocol", "population", "prior", "train", "validation", "test")
    }
    import digimon_engine

    assets.update(
        engine=sha(args.engine),
        seed_model=sha(args.seed_model),
        binding=sha(digimon_engine.__file__),
        runner=sha(__file__),
        trainer=sha(ROOT / "code/digimon_gym/agents/azero_trainer.py"),
        generation_runner=sha(ROOT / "code/tools/run_selfplay_generation.py"),
        mechanics=sha(ROOT / "data/mechanics_readiness.json"),
        admission=sha(ROOT / "data/card_admissibility.json"),
    )
    for name in (
        "code/digimon_gym/agents/belief_contract.py",
        "code/digimon_gym/agents/belief_evaluation.py",
        "code/tools/export_onnx.py",
        "code/tools/selfplay_scheduler.py",
        "code/tools/belief_policy_panel.py",
        "data/cards.json",
        "data/deck_formats.json",
    ):
        assets[name] = sha(ROOT / name)
    identity = {"schema": "belief-comparison-v1", "assets": assets}
    identity_path = out / "identity.json"
    if identity_path.exists() and read(identity_path) != identity:
        raise ValueError("comparison resume changed frozen assets")
    if not identity_path.exists():
        atomic_json(identity_path, identity)
    inputs = out / "inputs"
    inputs.mkdir(exist_ok=True)
    engine = inputs / args.engine.name
    if not engine.exists():
        shutil.copy2(args.engine, engine)
    for dll in args.engine.parent.glob("*.dll"):
        if not (inputs / dll.name).exists():
            shutil.copy2(dll, inputs / dll.name)
    if sha(engine) != assets["engine"]:
        raise ValueError("frozen engine changed")
    progress = {"status": "running", "completed_jobs": []}
    env = os.environ.copy()
    env.update(PYTHONPATH=str(ROOT / "code"), OMP_NUM_THREADS="1", MKL_NUM_THREADS="1")

    def job(name, files, fn):
        dest = out / name
        dest.parent.mkdir(parents=True, exist_ok=True)
        marker = dest.with_suffix(".job.json")
        signature = {str(p): sha(p) for p in files}
        if marker.exists():
            old = read(marker)
            if old["inputs"] != signature:
                raise ValueError(f"{name}: changed job inputs")
            for p, h in old["outputs"].items():
                if sha(p) != h:
                    raise ValueError(f"{name}: changed committed output")
            return old["result"]
        progress["active_job"] = name
        atomic_json(out / "progress.json", progress)
        result, outputs = fn(dest)
        atomic_json(
            marker,
            {
                "inputs": signature,
                "outputs": {str(p): sha(p) for p in outputs},
                "result": result,
            },
        )
        progress["completed_jobs"].append(name)
        atomic_json(out / "progress.json", progress)
        return result

    def command(cmd, log):
        import psutil

        started = time.monotonic()
        peak_rss = 0
        with log.open("w", encoding="utf-8") as f:
            proc = subprocess.Popen(
                [str(c) for c in cmd],
                cwd=ROOT,
                env=env,
                stdout=f,
                stderr=subprocess.STDOUT,
            )
            watched = psutil.Process(proc.pid)
            while proc.poll() is None:
                try:
                    peak_rss = max(
                        peak_rss,
                        sum(
                            p.memory_info().rss
                            for p in [watched] + watched.children(recursive=True)
                            if p.is_running()
                        ),
                    )
                except (psutil.NoSuchProcess, psutil.AccessDenied):
                    pass
                try:
                    proc.wait(timeout=0.5)
                except subprocess.TimeoutExpired:
                    pass
        atomic_json(
            log.with_suffix(".resources.json"),
            {
                "elapsed_seconds": time.monotonic() - started,
                "peak_process_tree_rss_bytes": peak_rss,
                "sampling_interval_seconds": 0.5,
                "returncode": proc.returncode,
            },
        )
        if proc.returncode:
            raise RuntimeError(f"stage failed ({proc.returncode}); preserved {log}")

    trained = {}
    for seed in protocol["seeds"]:
        for regime in ("open_decklist", "closed_decklist"):

            def train(dest, seed=seed, regime=regime):
                state = (
                    read(dest / "state.json")
                    if (dest / "state.json").exists()
                    else {"next_gen": 0}
                )
                remaining = protocol["budgets"]["generations"] - state["next_gen"]
                if remaining > 0:
                    cmd = [
                        sys.executable,
                        ROOT / "code/tools/run_selfplay_generation.py",
                        "--run-dir",
                        dest,
                        "--seed-model",
                        args.seed_model,
                        "--decks",
                        experiment / "train.json",
                        "--cli-bin",
                        engine,
                        "--information-regime",
                        regime,
                        "--games",
                        protocol["budgets"]["training_games_per_seed"],
                        "--generations",
                        remaining,
                        "--procs",
                        args.workers,
                        "--chunk-games",
                        4,
                        "--sims",
                        protocol["budgets"]["search_simulations"],
                        "--worlds",
                        protocol["budgets"]["worlds"],
                        "--eval-batch-size",
                        protocol["budgets"]["worlds"],
                        "--uniform",
                        "--full-search",
                        "--skip-eval",
                        "--epochs",
                        2,
                        "--batch-size",
                        64,
                        "--train-device",
                        "cpu",
                        "--seed",
                        seed,
                    ]
                    if regime == "closed_decklist":
                        cmd += ["--belief-prior", experiment / "prior.json"]
                    pending = dest / f"gen{state['next_gen']}"
                    if pending.exists():
                        cmd += ["--resume-selfplay"]
                    command(cmd, dest.parent / f"{dest.name}.log")
                state = read(dest / "state.json")
                final = Path(state["prev"]["model_zip"])
                onnx = Path(state["prev"]["model_onnx"])
                return {"zip": str(final), "onnx": str(onnx)}, [
                    final,
                    onnx,
                    Path(str(onnx) + ".meta.json"),
                ]

            trained[seed, regime] = job(
                f"train/{seed}/{regime}",
                [
                    engine,
                    args.seed_model,
                    experiment / "prior.json",
                    experiment / "train.json",
                    experiment / "protocol.json",
                ],
                train,
            )

    # One fixed champion for every seed/panel; never a moving self-play anchor.
    first = protocol["seeds"][0]
    anchor_zip = out / f"train/{first}/open_decklist/gen0/model.zip"
    anchor_onnx = anchor_zip.with_suffix(".onnx")

    def native_eval(dest, model, decks, seed, games, policy_only):
        dest.mkdir(parents=True, exist_ok=True)
        pool = dest / "opponent.json"
        atomic_json(
            pool,
            {
                "schema": "search-opponent-pool-v1",
                "selfplay_probability": 0.0,
                "opponents": [
                    {
                        "id": "fixed-anchor",
                        "path": str(anchor_onnx.resolve()),
                        "sha256": sha(anchor_onnx),
                        "metadata_sha256": sha(str(anchor_onnx) + ".meta.json"),
                        "weight": 1.0,
                    }
                ],
            },
        )
        cmd = [
            engine,
            "selfplay",
            "--model",
            model,
            "--opponent-pool",
            pool,
            "--decks",
            decks,
            "--information-regime",
            "closed_decklist",
            "--belief-prior",
            experiment / "prior.json",
            "--sims",
            protocol["budgets"]["search_simulations"],
            "--worlds",
            protocol["budgets"]["worlds"],
            "--eval-batch-size",
            protocol["budgets"]["worlds"],
        ]
        if policy_only:
            cmd += ["--policy-only"]
        run_dir = dest / "games"
        directories = run_queue(
            command=[str(c) for c in cmd],
            gen_dir=run_dir,
            games=games,
            workers=args.workers,
            chunk_games=16,
            seed=seed,
            env=env,
            identity={
                "comparison": identity,
                "model": sha(model),
                "anchor": sha(anchor_onnx),
                "mode": "policy_only" if policy_only else "search_assisted",
            },
            resume=(run_dir / "schedule.json").exists(),
        )
        manifests = [d / "manifest.json" for d in directories]
        records = [read(m) for m in manifests]
        combined = {
            "games": games,
            "opponent_games": [row for m in records for row in m["opponent_games"]],
        }
        return native_record(combined), manifests

    def policy_eval(dest, candidate, opponent, seed, games):
        from concurrent.futures import ThreadPoolExecutor

        dest.mkdir(parents=True, exist_ok=True)

        def chunk(start):
            path = dest / f"chunk-{start:06d}.json"
            # Partial panels are rerun with identical indexed games. A completed
            # panel is reused only through the outer immutable job marker.
            command(
                [
                    sys.executable,
                    ROOT / "code/tools/belief_policy_panel.py",
                    "--candidate",
                    candidate,
                    "--opponent",
                    opponent,
                    "--decks",
                    experiment / "test.json",
                    "--seed",
                    seed,
                    "--games",
                    min(32, games - start),
                    "--game-start",
                    start,
                    "--out",
                    path,
                ],
                dest / f"chunk-{start:06d}.log",
            )
            return path

        with ThreadPoolExecutor(max_workers=args.workers) as workers:
            paths = list(workers.map(chunk, range(0, games, 32)))
        rows = [read(path) for path in paths]
        keys = (
            "wins",
            "losses",
            "draws",
            "failed",
            "truncated",
            "games",
            "first_seat_games",
            "second_seat_games",
        )
        result = {key: sum(row[key] for row in rows) for key in keys}
        result["errors"] = [e for row in rows for e in row.get("errors", [])]
        path = dest.with_suffix(".evaluation.json")
        atomic_json(path, result)
        return result, [path] + paths

    def train_exploiter(dest, target, seed):
        import numpy as np
        from sb3_contrib import MaskablePPO
        from sb3_contrib.common.wrappers import ActionMasker
        from digimon_gym.agents.pilot_training import (
            OpponentWrapper,
            make_agent_opponent_fn,
            _unwrap_to_digimon_env,
        )
        from digimon_gym.digimon_gym import DigimonEnv
        from model_contract import current_contract

        decks = read(experiment / "train.json")
        frozen = MaskablePPO.load(str(target), device="cpu")
        profile = frozen.digimon_contract["observation_profile"]

        class VaryDecks(OpponentWrapper):
            def reset(self, *, seed=None, options=None):
                r = int(rng.integers(len(decks)))
                s = int(rng.integers(len(decks)))
                return super().reset(
                    seed=seed,
                    options={"deck1": decks[r]["cards"], "deck2": decks[s]["cards"]},
                )

        rng = np.random.default_rng(seed)
        env = ActionMasker(
            VaryDecks(
                DigimonEnv(
                    deck1=decks[0]["cards"],
                    deck2=decks[0]["cards"],
                    tensor_profile=profile,
                ),
                opponent_fn=make_agent_opponent_fn(str(target)),
            ),
            lambda e: _unwrap_to_digimon_env(e).action_mask(),
        )
        model = MaskablePPO(
            "MlpPolicy",
            env,
            n_steps=128,
            batch_size=64,
            n_epochs=4,
            seed=seed,
            policy_kwargs={"net_arch": {"pi": [32], "vf": [32]}},
            device="cpu",
        )
        model.digimon_contract = current_contract(profile)
        model.learn(total_timesteps=protocol["budgets"]["exploiter_steps"])
        dest.parent.mkdir(parents=True, exist_ok=True)
        path = dest.with_suffix(".zip")
        model.save(str(path))
        env.close()
        info = dest.with_suffix(".training.json")
        atomic_json(
            info,
            {
                "fresh": True,
                "seed": seed,
                "target_sha256": sha(target),
                "timesteps": model.num_timesteps,
                "information_regime": "closed_decklist",
            },
        )
        return {"zip": str(path), "training": str(info)}, [path, info]

    initializer = {}
    for seed in protocol["seeds"]:
        initializer[seed] = job(
            f"evaluate/{seed}/unchanged_seed",
            [args.seed_model, experiment / "test.json"],
            lambda dest: policy_eval(
                dest,
                args.seed_model,
                "greedy",
                seed + 4000000,
                protocol["panels"]["anchors"]["minimum_games_per_seed"],
            ),
        )
    panels = {k: [] for k in protocol["panels"]}
    for seed in protocol["seeds"]:
        results = {}
        for label, regime in (
            ("baseline", "open_decklist"),
            ("candidate", "closed_decklist"),
        ):
            model = trained[seed, regime]
            files = [
                Path(model["onnx"]),
                Path(model["onnx"] + ".meta.json"),
                anchor_onnx,
                Path(str(anchor_onnx) + ".meta.json"),
                experiment / "prior.json",
                experiment / "test.json",
                experiment / "protocol.json",
            ]
            for panel in ("heldout_policy", "heldout_search"):
                count = protocol["panels"][panel]["minimum_games_per_seed"]
                results[label, panel] = job(
                    f"evaluate/{seed}/{label}/{panel}",
                    files,
                    lambda dest, model=model, panel=panel, count=count: native_eval(
                        dest,
                        model["onnx"],
                        experiment / "test.json",
                        seed + 1000000,
                        count,
                        panel == "heldout_policy",
                    ),
                )
            count = protocol["panels"]["anchors"]["minimum_games_per_seed"]
            results[label, "anchors"] = job(
                f"evaluate/{seed}/{label}/anchors",
                [Path(model["zip"]), anchor_zip, experiment / "test.json"],
                lambda dest, model=model: policy_eval(
                    dest, model["zip"], "greedy", seed + 2000000, count
                ),
            )
            exploiter = job(
                f"exploiters/{seed}/{label}",
                [
                    Path(model["zip"]),
                    experiment / "train.json",
                    experiment / "protocol.json",
                ],
                lambda dest, model=model: train_exploiter(dest, model["zip"], seed),
            )
            results[label, "exploiter"] = job(
                f"evaluate/{seed}/{label}/exploiter",
                [Path(exploiter["zip"]), Path(model["zip"]), experiment / "test.json"],
                lambda dest, model=model, exploiter=exploiter: policy_eval(
                    dest,
                    exploiter["zip"],
                    model["zip"],
                    seed + 3000000,
                    protocol["panels"]["exploiter"]["minimum_games_per_seed"],
                ),
            )
        for panel, rule in protocol["panels"].items():
            panels[panel].append(
                {
                    "seed": seed,
                    "status": "completed",
                    "evaluation_mode": rule["evaluation_mode"],
                    "candidate": results["candidate", panel],
                    "baseline": results["baseline", panel],
                }
            )
        atomic_json(out / "partial-results.json", panels)
    result = {
        "protocol_hash": fingerprint(protocol),
        "evaluation_information_regime": "closed_decklist",
        "seeds": protocol["seeds"],
        "budgets": protocol["budgets"],
        "panels": panels,
        "training_models": {
            f"{seed}/{regime}": value for (seed, regime), value in trained.items()
        },
        "unchanged_seed": initializer,
        "fixed_anchor_sha256": sha(anchor_onnx),
    }
    atomic_json(out / "results.json", result)
    atomic_json(out / "promotion.json", promotion(protocol, result))
    progress["status"] = "completed"
    progress["active_job"] = None
    atomic_json(out / "progress.json", progress)


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--experiment", type=Path, required=True)
    p.add_argument("--seed-model", type=Path, required=True)
    p.add_argument("--engine", type=Path, required=True)
    p.add_argument("--out", type=Path, required=True)
    p.add_argument("--workers", type=int, default=2)
    args = p.parse_args(argv)
    args.out.mkdir(parents=True, exist_ok=True)
    with controller_lock(args.out / "comparison.lock"):
        try:
            run(args)
        except BaseException as error:
            atomic_json(
                args.out / "failure.json",
                {"error": str(error), "promotion_eligible": False},
            )
            raise
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
