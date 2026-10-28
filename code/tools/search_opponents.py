"""Freeze historical search opponents; independent of PPO league configuration."""
from __future__ import annotations

import json
import math
from collections import Counter
from pathlib import Path

from tools.selfplay_scheduler import atomic_json, sha256

SCHEMA = "search-opponent-pool-v1"
CONTRACT_KEYS = ("observation_profile", "tensor_version", "feature_schema_version",
                 "tensor_size", "tensor_layout_hash", "action_space_size", "action_schema_version")


def metadata_path(model: Path) -> Path:
    return Path(str(model) + ".meta.json")


def checkpoint_entry(identifier: str, path: Path, weight: float = 1.0) -> dict:
    path = path.resolve(strict=True)
    return dict(id=identifier, path=str(path), sha256=sha256(path),
                metadata_sha256=sha256(metadata_path(path)), weight=weight)


def validate_pool(pool: dict, base: Path, current: Path) -> dict:
    """Validate hashes and sidecars without importing ONNX; native load also
    checks the embedded contract, actual shapes and terminal value semantics.
    """
    if set(pool) != {"schema", "selfplay_probability", "opponents"} or pool["schema"] != SCHEMA:
        raise ValueError("Invalid historical opponent pool schema")
    probability = pool["selfplay_probability"]
    if type(probability) not in (int, float) or not math.isfinite(probability) or not 0 <= probability <= 1:
        raise ValueError("selfplay_probability must be finite and in [0, 1]")
    if not isinstance(pool["opponents"], list) or not pool["opponents"]:
        raise ValueError("Requested history pool is empty; supply explicit checkpoints or start at a later generation")
    current_hash = sha256(current)
    current_meta = json.loads(metadata_path(current).read_text())
    if any(k not in current_meta for k in CONTRACT_KEYS) or current_meta.get("value_semantics") != "terminal_outcome":
        raise ValueError("Current model needs a complete terminal-outcome metadata contract")
    ids, hashes, entries = set(), set(), []
    for entry in pool["opponents"]:
        if set(entry) != {"id", "path", "sha256", "metadata_sha256", "weight"}:
            raise ValueError("Invalid opponent entry fields")
        identifier, weight = entry["id"], entry["weight"]
        if not isinstance(identifier, str) or not identifier.strip() or identifier == "current" or identifier in ids:
            raise ValueError("Opponent IDs must be unique, nonempty and not 'current'")
        if type(weight) not in (int, float) or not math.isfinite(weight) or weight <= 0:
            raise ValueError("Opponent weights must be finite and positive")
        path = (base / entry["path"]).resolve(strict=True)
        digest = sha256(path)
        if digest == current_hash or digest in hashes:
            raise ValueError(f"Duplicate/current opponent model: {identifier}")
        if digest != entry["sha256"] or sha256(metadata_path(path)) != entry["metadata_sha256"]:
            raise ValueError(f"Opponent artifact hash mismatch: {identifier}")
        meta = json.loads(metadata_path(path).read_text())
        if any(meta.get(k) != current_meta[k] for k in CONTRACT_KEYS) or meta.get("value_semantics") != "terminal_outcome":
            raise ValueError(f"Opponent contract/value mismatch: {identifier}")
        ids.add(identifier)
        hashes.add(digest)
        entries.append({**entry, "path": str(path)})
    if not math.isfinite(sum(e["weight"] for e in entries)):
        raise ValueError("Opponent weight sum must be finite")
    return {**pool, "opponents": entries}


def resolve_pool(run_dir: Path, generation: int, current: Path, *,
                 checkpoints: list[str] = (), source: Path | None = None,
                 history_window: int = 0, selfplay_probability: float | None = None) -> dict:
    if type(history_window) is not int or history_window < 0:
        raise ValueError("history_window must be a nonnegative integer")
    pool = (json.loads(source.read_text()) if source else
            dict(schema=SCHEMA, selfplay_probability=0.5, opponents=[]))
    if source:
        pool = validate_pool(pool, source.resolve().parent, current)
    if selfplay_probability is not None:
        pool["selfplay_probability"] = selfplay_probability
    for spec in checkpoints:
        identifier, separator, raw = spec.partition("=")
        if not separator or not raw:
            raise ValueError("--opponent-checkpoint must be ID=PATH")
        pool["opponents"].append(checkpoint_entry(identifier, Path(raw)))
    candidates = sorted((int(p.name[3:]), p) for p in run_dir.glob("gen*")
                        if p.is_dir() and p.name[3:].isdigit() and int(p.name[3:]) < generation - 1
                        and (p / "summary.json").exists() and (p / "model.zip").exists()
                        and (p / "model.onnx").exists())
    for index, folder in (candidates[-history_window:] if history_window else []):
        summary = json.loads((folder / "summary.json").read_text())
        if summary.get("generation") != index:
            raise ValueError(f"Completed generation receipt mismatch: {folder}")
        pool["opponents"].append(checkpoint_entry(f"generation-{index}", folder / "model.onnx"))
    pool = validate_pool(pool, Path.cwd(), current)
    return pool


def freeze_pool(run_dir: Path, generation: int, current: Path | None, *,
                checkpoints: list[str] = (), source: Path | None = None,
                history_window: int = 0, selfplay_probability: float | None = None,
                resume: bool = False) -> Path | None:
    """Commit a generation's pool before scheduler launch. Never refresh on resume."""
    if type(history_window) is not int or history_window < 0:
        raise ValueError("history_window must be a nonnegative integer")
    target = run_dir / f"gen{generation}" / "opponent_pool.json"
    request_path = target.with_name("opponent_pool_request.json")
    if not (checkpoints or source or history_window):
        if selfplay_probability is not None or target.exists() or request_path.exists():
            raise ValueError("Opponent mixture requires configured history; cannot remove a frozen pool")
        return None
    if current is None:
        raise ValueError("Historical opponents require a current model, not uniform bootstrap")
    request = dict(checkpoints=list(checkpoints), source=str(source.resolve()) if source else None,
                   source_sha256=sha256(source) if source else None, history_window=history_window,
                   selfplay_probability=selfplay_probability, generation=generation)
    pool = resolve_pool(run_dir, generation, current, checkpoints=checkpoints, source=source,
                        history_window=history_window, selfplay_probability=selfplay_probability)
    if target.exists() or request_path.exists():
        if (not resume or not target.exists() or not request_path.exists()
                or json.loads(target.read_text()) != pool or json.loads(request_path.read_text()) != request):
            raise ValueError("Frozen opponent pool/request differs; preserving partial generation")
    elif resume:
        raise ValueError("Missing frozen opponent pool; cannot add history during resume")
    else:
        target.parent.mkdir(parents=True, exist_ok=True)
        atomic_json(request_path, request)
        atomic_json(target, pool)
    return target


def pool_assets(path: Path, current: Path) -> dict[str, Path]:
    pool = validate_pool(json.loads(path.read_text()), path.resolve().parent, current)
    assets = {"opponent_pool": path}
    for index, entry in enumerate(pool["opponents"]):
        model = Path(entry["path"])
        assets[f"opponent_{index}"] = model
        assets[f"opponent_{index}_meta"] = metadata_path(model)
    return assets


def summarize_opponents(directories: list[Path]) -> dict:
    opponents, learner_decks, opening_order = Counter(), Counter(), Counter()
    rows, games = 0, 0
    for directory in directories:
        manifest = json.loads((directory / "manifest.json").read_text())
        receipts = manifest.get("opponent_games", [])
        if len(receipts) != manifest["games"]:
            raise ValueError("Missing historical opponent game receipts")
        for pair, record in zip(manifest["game_decks"], receipts, strict=True):
            opponents[record["opponent_id"]] += 1
            learner = record["learner_seat"]
            seats = (0, 1) if learner is None else (learner,)
            if learner is not None:
                opening_order["first" if learner == record["first_player"] else "second"] += 1
                if record["training_rows_by_seat"][1 - learner] != 0:
                    raise ValueError("Historical opponent rows leaked into learner training")
            for seat in seats:
                learner_decks[manifest["deck_names"][pair[seat]].split(":sha256:", 1)[0]] += 1
            rows += sum(record["training_rows_by_seat"])
            games += 1
    return dict(games=games, games_by_opponent=dict(opponents), learner_appearances=dict(learner_decks),
                historical_learner_opening_order=dict(opening_order), training_rows=rows,
                scope="collected games before quarantine; historical actor rows excluded")
