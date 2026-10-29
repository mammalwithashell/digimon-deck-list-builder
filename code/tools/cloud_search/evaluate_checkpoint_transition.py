"""Re-evaluate two saved checkpoints on one immutable, repaired engine image.

Mount the old campaign read-only at --source and use a new --out directory.
Old reports/replay are never reused. Selection recommends continuation weights;
it does not promote a champion, start training, or alter the source campaign.
"""
from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess
import sys
import time
from types import SimpleNamespace
import zipfile

from tools.selfplay_scheduler import atomic_json, sha256

PANEL = "combined_validation"
PROFILE = "standard_lite_v2"
ANCHORS = ("greedy", "parent-checkpoint4", "development-fixed")
INCOMPLETE = {"truncated", "step_limit", "crash", "evaluation_end", "reset_before_terminal"}
CONTRACT = ("observation_profile", "tensor_version", "feature_schema_version", "tensor_size",
            "tensor_layout_hash", "action_space_size", "action_schema_version", "card_eval_event_version")
SELECTION_POLICY = dict(learned_anchor_lower_bound=-.02, greedy_lower_bound=-.01,
                        confidence=.95, fallback_checkpoint=3)
RUNTIME_FILES = ("tools/cloud_search/evaluate_checkpoint_transition.py", "tools/anchored_eval_cli.py",
    "digimon_gym/agents/anchored_eval.py", "digimon_gym/agents/card_eval.py",
    "digimon_gym/agents/gauntlet.py", "model_contract.py", "data_paths.py",
    "tools/cloud_search/run_seed_comparison.py")


def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def expected_cohort(snapshot, seed, games_per_archetype):
    from digimon_gym.agents.anchored_eval import (
        BALANCED_COHORT_SCHEME, balanced_deck_pairs, _seat_balanced_seed,
    )
    if BALANCED_COHORT_SCHEME != "archetype-paired-v2":
        raise ValueError("Review the transition protocol before changing the cohort version")
    archetypes = {}
    for deck in snapshot["decks"]:
        archetypes.setdefault(deck["archetype"], []).append(SimpleNamespace(
            archetype_name=deck["archetype"], deck_id=deck["deck_id"]))
    if set(archetypes) != set(snapshot["eligible_archetypes"]):
        raise ValueError("Snapshot archetype coverage differs from its declared cohort")
    for decks in archetypes.values():
        if len({d.deck_id for d in decks}) != len(decks):
            raise ValueError("Duplicate deck variant in snapshot")
    pairs = list(balanced_deck_pairs(SimpleNamespace(archetypes=archetypes), games_per_archetype, seed))
    cohort = []
    for anchor in ANCHORS:
        for i, (first, second) in enumerate(pairs):
            game_seed = _seat_balanced_seed(seed, i)
            cohort.append(dict(anchor=anchor, panel=PANEL, evaluation_mode="policy_only", seed=game_seed,
                agent_archetype=first.archetype_name, opponent_archetype=second.archetype_name,
                agent_deck_id=first.deck_id, opponent_deck_id=second.deck_id,
                opening_player=1 + game_seed % 2))
    return BALANCED_COHORT_SCHEME, cohort


def prepare(source, out, *, image, source_plan_sha256, checkpoint_hashes,
            seed=22261026, games_per_archetype=200):
    from data_paths import CARDS_JSON
    source, out = Path(source).resolve(), Path(out).resolve()
    if out == source or out.is_relative_to(source) or source.is_relative_to(out):
        raise ValueError("Assessment output must be separate from the archived campaign")
    if out.exists():
        raise FileExistsError("Use a new assessment directory; existing evidence is never overwritten")
    if not re.fullmatch(r".+@sha256:[0-9a-f]{64}", image or "") or os.environ.get("DIGIMON_DEPLOYED_IMAGE") != image:
        raise ValueError("Runtime must match the explicit immutable image digest")
    source_plan_bytes = (source / "deployment-plan.json").read_bytes()
    if hashlib.sha256(source_plan_bytes).hexdigest() != source_plan_sha256:
        raise ValueError("Archived deployment-plan hash differs")
    original = json.loads(source_plan_bytes)
    if read(source / "experiment_state.json").get("stage") not in {"failed", "complete"}:
        raise ValueError("Only a stopped campaign with saved checkpoints can be assessed")
    if set(checkpoint_hashes) != {3, 4}:
        raise ValueError("Explicit checkpoint 3 and 4 hashes are required")
    inputs = {}

    def source_input(name):
        relative = f"inputs/{name}"
        path = (source / relative).resolve()
        if not path.is_relative_to(source) or sha256(path) != original.get("files", {}).get(relative):
            raise ValueError(f"Archived input differs from its frozen plan: {relative}")
        inputs[relative] = path
        return path

    snapshot = source_input("combined_snapshot.json")
    registry_path = source_input("anchors.json")
    registry = read(registry_path)
    champions = registry.get("champions", [])
    if sorted(c["name"] for c in champions) != sorted(ANCHORS[1:]):
        raise ValueError("Expected the same two frozen learned anchors")
    anchor_hashes = {}
    for champion in champions:
        name = PurePosixPath(champion["weights_path"].replace("\\", "/")).name
        weights = source_input(name)
        anchor_hashes[champion["name"]] = sha256(weights)
        champion["weights_path"] = str(out / "inputs" / name)
    for index, digest in checkpoint_hashes.items():
        path = source / f"generation/gen{index}/model.zip"
        if sha256(path) != digest:
            raise ValueError(f"Checkpoint {index} differs from its approved hash")
        with zipfile.ZipFile(path) as archive:
            if archive.testzip() is not None:
                raise ValueError(f"Checkpoint {index} is not a valid ZIP archive")
    protocol, cohort = expected_cohort(read(snapshot), seed, games_per_archetype)
    out.mkdir(parents=True, exist_ok=False)
    (out / "inputs").mkdir()
    for relative, path in inputs.items():
        target = out / relative
        shutil.copy2(path, target)
        if sha256(target) != original["files"][relative]:
            raise ValueError("Input changed while copying")
    atomic_json(out / "inputs/anchors.json", registry)
    checkpoints = {}
    for index, digest in checkpoint_hashes.items():
        target = out / f"checkpoints/checkpoint-{index}.zip"
        target.parent.mkdir(exist_ok=True)
        shutil.copy2(source / f"generation/gen{index}/model.zip", target)
        if sha256(target) != digest:
            raise ValueError("Checkpoint changed while copying")
        checkpoints[str(index)] = dict(sha256=digest, path=target.relative_to(out).as_posix())
    atomic_json(out / "cohort.json", cohort)
    # Paths are recorded relative to code/, so the artifact remains interpretable off-host.
    code_root = Path(__file__).resolve().parents[2]
    runtime = [code_root / name for name in RUNTIME_FILES]
    plan = dict(schema_version=1, image=image, protocol=protocol, source=str(source),
        source_plan_sha256=source_plan_sha256, source_engine_image=original.get("image"),
        runtime_cards_sha256=sha256(CARDS_JSON),
        source_input_hashes={relative: original["files"][relative] for relative in inputs},
        seed=seed, games_per_archetype=games_per_archetype,
        games_per_anchor=len(cohort) // len(ANCHORS), games_per_checkpoint=len(cohort),
        checkpoints=checkpoints, anchor_model_hashes=anchor_hashes,
        files={p.relative_to(out).as_posix(): sha256(p) for p in out.rglob("*") if p.is_file()},
        runtime_files={p.relative_to(code_root).as_posix(): sha256(p) for p in runtime},
        opening_seat_contract="candidate is player 1; opening player is 1 + seed % 2",
        selection_policy=SELECTION_POLICY,
        automatic_promotion=False, created_unix=time.time())
    atomic_json(out / "plan.json", plan)
    return plan


def validate_report(report, root, plan, index):
    provenance = report.get("provenance", {})
    expected = dict(status="complete", engine_image=plan["image"], cohort_scheme=plan["protocol"],
        evaluation_mode="policy_only", panel=PANEL, checkpoint_index=index,
        candidate_sha256=plan["checkpoints"][str(index)]["sha256"],
        checkpoint_sha256=plan["checkpoints"][str(index)]["sha256"], base_seed=plan["seed"],
        games_per_archetype=plan["games_per_archetype"], requested_games_per_anchor=plan["games_per_anchor"],
        snapshot_sha256=plan["files"]["inputs/combined_snapshot.json"],
        anchors_sha256=plan["files"]["inputs/anchors.json"],
        anchor_model_hashes=plan["anchor_model_hashes"], observation_profile=PROFILE, deterministic=True,
        runtime_cards_sha256=plan["runtime_cards_sha256"])
    if report.get("schema_version") != 1 or report.get("checkpoint_step") != index or any(
        provenance.get(key) != value for key, value in expected.items()
    ):
        raise ValueError("Report provenance differs from the pinned evaluation contract")
    if any(key not in provenance for key in CONTRACT) or provenance["card_eval_event_version"] < 2:
        raise ValueError("Missing model or card telemetry contract")
    if any(champion.get("tensor_layout_hash") != provenance["tensor_layout_hash"]
           for champion in read(root / "inputs/anchors.json")["champions"]):
        raise ValueError("Checkpoint contract differs from the frozen learned anchors")
    if any(not isinstance(report.get(key), list) for key in ("cards", "routes", "route_decisions", "effect_sequences")):
        raise ValueError("Canonical card evaluation metadata is missing")
    expected_games = {(row["anchor"], row["seed"]): row for row in read(root / "cohort.json")}
    games = report.get("games", [])
    if len(games) != len(expected_games):
        raise ValueError("Incomplete evaluation cohort")
    seen = set()
    for game in games:
        context, seed = game.get("context", {}), game.get("seed")
        key = (context.get("anchor"), seed)
        if key in seen or key not in expected_games or type(seed) is not int:
            raise ValueError("Duplicate or unexpected evaluation seed/seat")
        seen.add(key)
        row = expected_games[key]
        if any(context.get(k) != v for k, v in row.items() if k not in ("seed", "opening_player")):
            raise ValueError("Evaluation deck/anchor/seat cohort differs")
        winner = game.get("winner_id")
        if (not game.get("terminal_reason") or game["terminal_reason"] in INCOMPLETE
                or game.get("evaluation_error") or "winner_id" not in game
                or not (winner is None or type(winner) is int and winner in (0, 1, 2))):
            raise ValueError("Incomplete or invalid terminal evaluation game")
    return games


def select_checkpoint(by_anchor, checkpoints):
    thresholds = {name: (-.01 if name == "greedy" else -.02) for name in ANCHORS}
    passed = {name: by_anchor[name]["ci95"][0] >= threshold for name, threshold in thresholds.items()}
    index = 4 if all(passed.values()) else 3
    return dict(checkpoint_index=index, checkpoint_sha256=checkpoints[str(index)]["sha256"],
        anchor_gates=passed, lower_bound_thresholds=thresholds, automatic_promotion=False,
        basis="Checkpoint 4 only when each paired learned-anchor lower bound is at least -2pp "
              "and greedy at least -1pp; otherwise retain checkpoint 3. Continuation choice, not champion promotion.")


def compare_complete_reports(control, candidate, samples):
    from tools.cloud_search.run_seed_comparison import compare_reports
    comparison = compare_reports(control, candidate, seed=19, samples=samples)
    comparison["by_anchor"] = {anchor: compare_reports(
        [g for g in control if g["context"]["anchor"] == anchor],
        [g for g in candidate if g["context"]["anchor"] == anchor], seed=19, samples=samples)["aggregate"]
        for anchor in ANCHORS}
    return comparison


def pinned_path(root, relative):
    path = (root / relative).resolve()
    if Path(relative).is_absolute() or not path.is_relative_to(root.resolve()):
        raise ValueError("Assessment artifact path escapes its root")
    return path


def validate_frozen(root, plan):
    from data_paths import CARDS_JSON
    if read(root / "plan.json") != plan:
        raise ValueError("Assessment plan differs from its saved contract")
    required = {"cohort.json", "inputs/combined_snapshot.json", "inputs/anchors.json",
                "checkpoints/checkpoint-3.zip", "checkpoints/checkpoint-4.zip"}
    if set(plan.get("checkpoints", {})) != {"3", "4"}:
        raise ValueError("Assessment must pin checkpoints 3 and 4")
    for index, checkpoint in plan["checkpoints"].items():
        relative = f"checkpoints/checkpoint-{index}.zip"
        if checkpoint.get("path") != relative or plan["files"].get(relative) != checkpoint.get("sha256"):
            raise ValueError("Frozen checkpoint path/hash binding differs")
    registry = read(root / "inputs/anchors.json")
    for champion in registry["champions"]:
        weights = Path(champion["weights_path"]).resolve()
        if not weights.is_relative_to(root.resolve()):
            raise ValueError("Assessment anchor must be frozen inside the assessment root")
        relative = weights.relative_to(root.resolve()).as_posix()
        if plan["files"].get(relative) != plan["anchor_model_hashes"].get(champion["name"]):
            raise ValueError("Frozen anchor hash binding differs")
        required.add(relative)
    if not required <= set(plan["files"]):
        raise ValueError("Assessment plan omitted required frozen artifacts")
    for relative, digest in plan["files"].items():
        if sha256(pinned_path(root, relative)) != digest:
            raise ValueError("Frozen assessment input changed")
    code_root = Path(__file__).resolve().parents[2]
    if (set(plan["runtime_files"]) != set(RUNTIME_FILES) or
            any(sha256(pinned_path(code_root, relative)) != digest for relative, digest in plan["runtime_files"].items())):
        raise ValueError("Assessment runtime code changed after preparation")
    if sha256(CARDS_JSON) != plan["runtime_cards_sha256"]:
        raise ValueError("Assessment runtime card data changed")
    protocol, cohort = expected_cohort(read(root / "inputs/combined_snapshot.json"), plan["seed"], plan["games_per_archetype"])
    if protocol != plan["protocol"] or cohort != read(root / "cohort.json"):
        raise ValueError("Assessment cohort differs from the current protocol")


def validate_assessment(result_path, source, image):
    """Verify a completed assessment's ancestry and recompute its selection.

    A new campaign must not trust a standalone status/selection JSON. All
    referenced artifacts remain in the read-only assessment alongside result.
    """
    result_path, source = Path(result_path).resolve(), Path(source).resolve()
    root = result_path.parent
    result, plan = read(result_path), read(root / "plan.json")
    if (result.get("status") != "complete" or result.get("image") != image
            or result.get("protocol") != "archetype-paired-v2" or plan.get("image") != image
            or plan.get("protocol") != result["protocol"] or result.get("automatic_promotion") is not False
            or sha256(root / "plan.json") != result.get("plan_sha256")
            or result.get("checkpoints") != plan.get("checkpoints")
            or result.get("runtime_cards_sha256") != plan.get("runtime_cards_sha256")
            or result.get("games_per_archetype") != plan.get("games_per_archetype")
            or result.get("games_per_checkpoint") != plan.get("games_per_checkpoint")
            or plan.get("selection_policy") != SELECTION_POLICY or result.get("bootstrap_seed") != 19
            or type(result.get("bootstrap_samples")) is not int or result["bootstrap_samples"] < 2):
        raise ValueError("Transition assessment result/plan contract differs")
    validate_frozen(root, plan)
    if sha256(source / "deployment-plan.json") != plan["source_plan_sha256"]:
        raise ValueError("Transition assessment belongs to another source campaign")
    original = read(source / "deployment-plan.json")
    original_registry = read(source / "inputs/anchors.json")
    required_sources = {"inputs/combined_snapshot.json", "inputs/anchors.json"}
    for champion in original_registry["champions"]:
        name = PurePosixPath(champion["weights_path"].replace("\\", "/")).name
        relative = f"inputs/{name}"
        required_sources.add(relative)
        if (plan["files"].get(relative) != plan["source_input_hashes"].get(relative)
                or plan["anchor_model_hashes"].get(champion["name"]) != plan["files"].get(relative)):
            raise ValueError("Transition assessment anchor ancestry differs")
        champion["weights_path"] = str(root / relative)
    if (set(plan["source_input_hashes"]) != required_sources
            or plan["files"]["inputs/combined_snapshot.json"] != plan["source_input_hashes"]["inputs/combined_snapshot.json"]
            or read(root / "inputs/anchors.json") != original_registry):
        raise ValueError("Transition assessment copied-input ancestry differs")
    for relative, digest in plan["source_input_hashes"].items():
        if sha256(pinned_path(source, relative)) != digest or original["files"].get(relative) != digest:
            raise ValueError("Transition assessment source input differs")
    for index in (3, 4):
        if sha256(source / f"generation/gen{index}/model.zip") != plan["checkpoints"][str(index)]["sha256"]:
            raise ValueError("Transition assessment checkpoint ancestry differs")
    if set(result.get("reports", {})) != {"3", "4"}:
        raise ValueError("Transition assessment needs both canonical reports")
    validated = []
    for index in (3, 4):
        artifact = result["reports"][str(index)]
        path = pinned_path(root, artifact["path"])
        if sha256(path) != artifact["sha256"]:
            raise ValueError("Transition assessment canonical report changed")
        report = read(path)
        validated.append((report, validate_report(report, root, plan, index)))
    if any(validated[0][0]["provenance"][k] != validated[1][0]["provenance"][k] for k in CONTRACT):
        raise ValueError("Checkpoint observation/action/telemetry contracts differ")
    comparison = compare_complete_reports(validated[0][1], validated[1][1], result["bootstrap_samples"])
    selection = select_checkpoint(comparison["by_anchor"], plan["checkpoints"])
    if comparison != result.get("comparison") or selection != result.get("selection"):
        raise ValueError("Transition assessment comparison/selection differs from its reports")
    return result


def run(root, plan, *, workers=2, timeout=3600, bootstrap_samples=10000):
    from data_paths import CARDS_JSON
    root = Path(root)
    if workers not in (1, 2) or timeout <= 0 or bootstrap_samples < 2:
        raise ValueError("Use one or two workers, positive timeout, and at least two bootstrap samples")
    if os.environ.get("DIGIMON_DEPLOYED_IMAGE") != plan["image"]:
        raise ValueError("Assessment runtime image differs")
    validate_frozen(root, plan)
    if (root / "result.json").exists() or (root / "evaluations").exists():
        raise FileExistsError("Assessment already attempted; use a new output root")
    (root / "evaluations").mkdir()
    result = dict(schema_version=1, status="running", image=plan["image"], protocol=plan["protocol"],
                  plan_sha256=sha256(root / "plan.json"), automatic_promotion=False, started_unix=time.time(),
                  bootstrap_seed=19, bootstrap_samples=bootstrap_samples,
                  runtime_cards_sha256=plan["runtime_cards_sha256"],
                  games_per_archetype=plan["games_per_archetype"], games_per_checkpoint=plan["games_per_checkpoint"])
    atomic_json(root / "result.json", result)

    def evaluate(index):
        folder = root / f"evaluations/checkpoint-{index}"
        folder.mkdir()
        context = dict(engine_image=plan["image"], checkpoint_index=index,
            checkpoint_sha256=plan["checkpoints"][str(index)]["sha256"],
            evaluation_started_unix=time.time(), trigger="engine_transition", frozen_during_collection=True,
            training_algorithm="search_selfplay", assessment_plan_sha256=result["plan_sha256"],
            runtime_cards_sha256=plan["runtime_cards_sha256"])
        atomic_json(folder / "context.json", context)
        command = [sys.executable, "-m", "tools.anchored_eval_cli", "--candidate",
            str(root / plan["checkpoints"][str(index)]["path"]), "--champions", str(root / "inputs/anchors.json"),
            "--games-per-archetype", str(plan["games_per_archetype"]), "--seed", str(plan["seed"]),
            "--device", "cpu", "--profile", PROFILE, "--deck-pool-snapshot", str(root / "inputs/combined_snapshot.json"),
            "--card-eval-dir", str(folder / "reports"), "--checkpoint-step", str(index),
            "--eval-window", str(index), "--eval-panel", PANEL, "--eval-context", str(folder / "context.json")]
        env = {**os.environ, "OMP_NUM_THREADS": "1", "MKL_NUM_THREADS": "1", "OPENBLAS_NUM_THREADS": "1",
               "DIGIMON_EVAL_FAILURE_DIR": str(folder / "failures"), "DIGIMON_CARDS_JSON": str(CARDS_JSON.resolve())}
        with (folder / "evaluation.log").open("w", encoding="utf-8") as log:
            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, timeout=timeout, check=True, env=env)
        paths = list((folder / "reports").glob("checkpoint_*.json"))
        if len(paths) != 1:
            raise ValueError("Expected exactly one canonical report per checkpoint")
        report = read(paths[0])
        games = validate_report(report, root, plan, index)
        return report, games, dict(path=paths[0].relative_to(root).as_posix(), sha256=sha256(paths[0]))

    try:
        with ThreadPoolExecutor(max_workers=workers) as executor:
            evaluated = list(executor.map(evaluate, (3, 4)))
        if any(evaluated[0][0]["provenance"][k] != evaluated[1][0]["provenance"][k] for k in CONTRACT):
            raise ValueError("Checkpoint observation/action/telemetry contracts differ")
        control, candidate = evaluated[0][1], evaluated[1][1]
        comparison = compare_complete_reports(control, candidate, bootstrap_samples)
        result.update(status="complete", reports={str(i): evaluated[j][2] for j, i in enumerate((3, 4))},
            checkpoints=plan["checkpoints"], comparison=comparison,
            selection=select_checkpoint(comparison["by_anchor"], plan["checkpoints"]))
    except BaseException as error:
        result.update(status="failed", error=str(error))
        raise
    finally:
        result["finished_unix"] = time.time()
        atomic_json(root / "result.json", result)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--image", required=True)
    parser.add_argument("--source-plan-sha256", required=True)
    parser.add_argument("--checkpoint3-sha256", required=True)
    parser.add_argument("--checkpoint4-sha256", required=True)
    parser.add_argument("--seed", type=int, default=22261026)
    parser.add_argument("--games-per-archetype", type=int, default=200)
    parser.add_argument("--workers", type=int, choices=(1, 2), default=2)
    parser.add_argument("--timeout", type=int, default=3600)
    args = parser.parse_args()
    plan = prepare(args.source, args.out, image=args.image, source_plan_sha256=args.source_plan_sha256,
        checkpoint_hashes={3: args.checkpoint3_sha256, 4: args.checkpoint4_sha256},
        seed=args.seed, games_per_archetype=args.games_per_archetype)
    result = run(args.out.resolve(), plan, workers=args.workers, timeout=args.timeout)
    print(json.dumps(dict(status=result["status"], selection=result["selection"], image=result["image"])), flush=True)


if __name__ == "__main__":
    main()
