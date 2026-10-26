"""Frozen policy-only duel with equal deck cells, both player roles and openers."""
import argparse
from collections import Counter
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

from tools.selfplay_scheduler import atomic_json, controller_lock, sha256

ARMS = ("mixed", "development")


def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def make_schedule(pool, pairs_per_cell, seed):
    from digimon_gym.agents.anchored_eval import balanced_deck_pairs, _seat_balanced_seed
    if pairs_per_cell < 1:
        raise ValueError("pairs_per_cell must be positive")
    quota = 2 * len(pool.archetypes) * pairs_per_cell
    return [dict(index=i, seed=_seat_balanced_seed(seed, i),
                 agent_archetype=a.archetype_name, opponent_archetype=b.archetype_name,
                 agent_deck_id=a.deck_id, opponent_deck_id=b.deck_id)
            for i, (a, b) in enumerate(balanced_deck_pairs(pool, quota, seed))]


def summarize(games, schedule, samples=10000):
    """Normalize both engine roles to mixed; bootstrap shared four-game blocks."""
    import numpy as np
    indexed = {}
    expected = {row["seed"]: row for row in schedule}
    matrix, clusters = {}, {}
    for arm in ARMS:
        indexed[arm] = {}
        for game in games[arm]:
            seed = game["seed"]
            if seed not in expected or seed in indexed[arm]:
                raise ValueError("Unexpected or duplicate evaluation seed")
            reason = game.get("terminal_reason")
            if not reason or reason in {"evaluation_end", "reset_before_terminal", "step_limit", "truncated", "crash"}:
                raise ValueError("Incomplete evaluation game")
            row = expected[seed]
            if any(game["context"].get(k) != row[k] for k in (
                    "agent_archetype", "opponent_archetype", "agent_deck_id", "opponent_deck_id")):
                raise ValueError("Evaluation deck schedule changed")
            winner = game["winner_id"]
            if winner not in (None, 0, 1, 2):
                raise ValueError("Invalid winner")
            mixed_player = 1 if arm == "mixed" else 2
            score = .5 if winner in (None, 0) else float(winner == mixed_player)
            indexed[arm][seed] = score
            names = (row["agent_archetype"], row["opponent_archetype"])
            if arm == "development":
                names = names[::-1]
            cell = matrix.setdefault(names, Counter())
            cell["wins" if score == 1 else "losses" if score == 0 else "draws"] += 1
            clusters.setdefault(row["index"] // 2, []).append(score)
        if set(indexed[arm]) != set(expected):
            raise ValueError("Missing evaluation games")
    if any(len(v) != 4 for v in clusters.values()):
        raise ValueError("Incomplete player-role/opening-order block")
    means = np.asarray([sum(v) / 4 for v in clusters.values()])
    rng = np.random.default_rng(411)
    bootstrap = []
    for start in range(0, samples, 200):
        bootstrap.extend(means[rng.integers(len(means), size=(min(200, samples-start), len(means)))].mean(axis=1).tolist())
    totals = Counter()
    cells = []
    for (mixed, development), counts in sorted(matrix.items()):
        totals.update(counts)
        cells.append(dict(mixed_deck=mixed, development_deck=development, games=sum(counts.values()),
                          **{k: counts[k] for k in ("wins", "losses", "draws")}))
    return dict(status="complete", perspective="mixed", games=4*len(means),
                **{k: totals[k] for k in ("wins", "losses", "draws")},
                score=float(means.mean()), ci95=list(np.quantile(bootstrap, [.025, .975])),
                bootstrap_unit="four games: both player roles and both opening orders",
                matrix=cells, automatic_promotion=False)


def prepare(source, root, pairs, seed):
    from sb3_contrib import MaskablePPO
    from model_contract import validate_checkpoint
    import digimon_engine
    from digimon_gym.agents.gauntlet import GeneralistDeckPool
    if read(source / "comparison_state.json")["stage"] != "complete":
        raise ValueError("Source experiment must be complete")
    if (root / "plan.json").exists() or (root / "started.json").exists():
        raise ValueError("Refusing to overwrite an existing evaluation")
    pools = [GeneralistDeckPool.from_snapshot(source / "inputs" / name,
             implemented_card_ids=digimon_engine.load_implemented_card_ids())
             for name in ("heldout_snapshot.json", "pet_evaluation_snapshot.json")]
    if set(pools[0].archetypes) & set(pools[1].archetypes):
        raise ValueError("Meta and pet archetypes overlap")
    pool = GeneralistDeckPool({**pools[0].archetypes, **pools[1].archetypes})
    if len(pool.archetypes) != 13:
        raise ValueError("This experiment requires the frozen 13-archetype cohort")
    schedule = make_schedule(pool, pairs, seed)
    inputs = root / "inputs"
    inputs.mkdir(parents=True, exist_ok=True)
    pool.write_snapshot(inputs / "combined_snapshot.json")
    # Preserve cohort identity for later separate meta/pet interpretation.
    for name in ("heldout_snapshot.json", "pet_evaluation_snapshot.json"):
        shutil.copy2(source / "inputs" / name, inputs / name)
    contracts, hashes = {}, {}
    for arm in ARMS:
        checkpoint = source / "arms" / arm / "generation/gen1/model.zip"
        evaluated_image = read(source / "evaluations" / arm / "window-000001/context.json")["engine_image"]
        if evaluated_image != os.environ["DIGIMON_DEPLOYED_IMAGE"]:
            raise ValueError("Use the same immutable native image as the completed evaluation")
        contracts[arm] = validate_checkpoint(MaskablePPO.load(checkpoint, device="cpu"))
        hashes[arm] = sha256(checkpoint)
        shutil.copy2(checkpoint, inputs / f"{arm}.zip")
        if sha256(inputs / f"{arm}.zip") != hashes[arm]:
            raise ValueError("Model changed during copy")
        provenance = read(source / "corpora" / arm / "provenance.json")
        if sha256(Path('/app/data/cards.json')) != provenance["cards_sha256"]:
            raise ValueError("Runtime card database differs from the trained experiment")
        atomic_json(root / "corpora" / arm / "provenance.json", provenance)
    if contracts[ARMS[0]] != contracts[ARMS[1]]:
        raise ValueError("Model contracts differ")
    plan = dict(kind="mixed-development-head-to-head", seed=seed, pairs_per_cell=pairs,
                games_per_evaluation=len(schedule), total_games=2*len(schedule),
                games_per_archetype=len(schedule)//len(pool.archetypes),
                archetypes=pool.archetype_names, meta_archetypes=pools[0].archetype_names,
                pet_archetypes=pools[1].archetype_names, model_hashes=hashes,
                contract=contracts[ARMS[0]], snapshot_sha256=sha256(inputs / "combined_snapshot.json"),
                image=os.environ["DIGIMON_DEPLOYED_IMAGE"], controller_sha256=sha256(Path(__file__)),
                automatic_promotion=False, training=False)
    atomic_json(root / "schedule.json", schedule)
    plan["schedule_sha256"] = sha256(root / "schedule.json")
    atomic_json(root / "plan.json", plan)
    for arm in ARMS:
        opposite = next(a for a in ARMS if a != arm)
        atomic_json(inputs / f"{arm}-anchors.json", dict(version=1, champions=[dict(
            name=opposite, weights_path=str(inputs / f"{opposite}.zip"), algorithm="mlp",
            observation_profile=plan["contract"]["observation_profile"],
            tensor_layout_hash=plan["contract"]["tensor_layout_hash"], source="frozen checkpoint 1")]))
        arm_root = root / "arms" / arm
        checkpoint = arm_root / "generation/gen1/model.zip"
        checkpoint.parent.mkdir(parents=True, exist_ok=True)
        checkpoint.symlink_to(os.path.relpath(inputs / f"{arm}.zip", checkpoint.parent))
        atomic_json(arm_root / "deployment-plan.json", plan)
        atomic_json(arm_root / "experiment_state.json", dict(stage="prepared", generation_index=1))
    print(json.dumps(plan), flush=True)


def run(root):
    plan = read(root / "plan.json")
    inputs = root / "inputs"
    if sha256(inputs / "combined_snapshot.json") != plan["snapshot_sha256"] or sha256(root / "schedule.json") != plan["schedule_sha256"]:
        raise ValueError("Frozen cohort/schedule changed")
    if os.environ.get("DIGIMON_DEPLOYED_IMAGE") != plan["image"] or sha256(Path(__file__)) != plan["controller_sha256"]:
        raise ValueError("Runtime/controller changed since preflight")
    for arm, digest in plan["model_hashes"].items():
        if sha256(inputs / f"{arm}.zip") != digest:
            raise ValueError("Frozen model changed")
        anchor = read(inputs / f"{arm}-anchors.json")["champions"]
        opposite = next(a for a in ARMS if a != arm)
        if len(anchor) != 1 or anchor[0]["name"] != opposite or anchor[0]["weights_path"] != str(inputs / f"{opposite}.zip"):
            raise ValueError("Opponent registry changed")
    if (root / "started.json").exists():
        raise ValueError("Refusing implicit restart of an existing evaluation")
    atomic_json(root / "started.json", dict(time_unix=time.time(), plan=plan))
    atomic_json(root / "comparison_state.json", dict(stage="checkpoint_evaluation", arm="both", time_unix=time.time()))
    processes, logs, games = {}, [], {}
    try:
        for arm in ARMS:
            arm_root = root / "arms" / arm
            atomic_json(arm_root / "started.json", dict(time_unix=time.time(), plan=plan))
            atomic_json(arm_root / "experiment_state.json", dict(stage="checkpoint_evaluation", generation_index=1, time_unix=time.time()))
            out = root / "evaluations" / arm
            folder = out / "window-000001"
            folder.mkdir(parents=True, exist_ok=False)
            context = dict(evaluation_started_unix=time.time(), engine_image=plan["image"],
                           training_algorithm="search_selfplay", comparison="direct frozen mixed versus development",
                           checkpoint_index=1, schedule_sha256=plan["schedule_sha256"])
            atomic_json(folder / "context.json", context)
            atomic_json(out / "status.json", dict(status="running"))
            log = (arm_root / "evaluation.log").open("w", encoding="utf-8")
            logs.append(log)
            command = [sys.executable, "-m", "tools.anchored_eval_cli", "--candidate", str(inputs / f"{arm}.zip"),
                "--champions", str(inputs / f"{arm}-anchors.json"), "--no-greedy", "--device", "cpu",
                "--profile", plan["contract"]["observation_profile"], "--seed", str(plan["seed"]),
                "--deck-pool-snapshot", str(inputs / "combined_snapshot.json"),
                "--games-per-archetype", str(plan["games_per_archetype"]), "--card-eval-dir", str(folder / "reports"),
                "--checkpoint-step", "1", "--eval-window", "1", "--eval-panel", "combined_head_to_head",
                "--eval-context", str(folder / "context.json")]
            processes[arm] = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT,
                env={**os.environ, "DIGIMON_ONNX_OPPONENT":"0", "DIGIMON_EVAL_FAILURE_DIR":str(folder / "failures")})
        pending = set(ARMS)
        while pending:
            for arm in list(pending):
                code = processes[arm].poll()
                if code is None:
                    continue
                if code:
                    raise RuntimeError(f"{arm} evaluator exited {code}; preserve evaluation.log and failures")
                folder = root / "evaluations" / arm / "window-000001"
                reports = [read(p) for p in (folder / "reports").glob("checkpoint_*.json")]
                if len(reports) != 1 or reports[0]["provenance"]["status"] != "complete":
                    raise ValueError("Missing complete evaluation report")
                games[arm] = reports[0]["games"]
                atomic_json(folder / "result.json", dict(status="complete", time_unix=time.time()))
                atomic_json(root / "arms" / arm / "experiment_state.json", dict(stage="complete", generation_index=1, time_unix=time.time()))
                atomic_json(root / "evaluations" / arm / "status.json", dict(status="stopped"))
                pending.remove(arm)
            if pending:
                time.sleep(2)
        result = summarize(games, read(root / "schedule.json"))
        atomic_json(root / "head_to_head_result.json", result)
        atomic_json(root / "comparison_state.json", dict(stage="complete", automatic_promotion=False, time_unix=time.time()))
    except BaseException as error:
        atomic_json(root / "comparison_state.json", dict(stage="failed", error=str(error), time_unix=time.time()))
        for arm in ARMS:
            atomic_json(root / "arms" / arm / "experiment_state.json", dict(stage="failed", error=str(error), generation_index=1, time_unix=time.time()))
            atomic_json(root / "evaluations" / arm / "status.json", dict(status="stopped", error=str(error)))
        raise
    finally:
        for proc in processes.values():
            if proc.poll() is None:
                proc.terminate()
                try:
                    proc.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait()
        for log in logs:
            log.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("/experiment"))
    parser.add_argument("--source", type=Path, default=Path("/source"))
    parser.add_argument("--prepare", action="store_true")
    parser.add_argument("--pairs-per-cell", type=int, default=10)
    parser.add_argument("--seed", type=int, default=91261025)
    args = parser.parse_args()
    args.root.mkdir(parents=True, exist_ok=True)
    with controller_lock(args.root / "controller.lock"):
        if args.prepare:
            prepare(args.source, args.root, args.pairs_per_cell, args.seed)
        else:
            run(args.root)


if __name__ == "__main__":
    main()
