"""Emit `selfplay-shards-v1` supervised warm-start shards from Studio puzzles & search exams.

Accepts both canonical `studio-puzzle-v1` files (`qa/puzzles/`) and `search-exam-v1`
files (`qa/search-exams/`), reconstructs each board plus `K` slot-permuted variants via
`permute_staged_puzzle`, verifies every step in the `target` line in `RustHeadlessGame`,
and writes `selfplay-shards-v1` arrays consumable out-of-the-box by both
`digimon_gym.agents.azero_trainer` and `digimon_gym.agents.azero_lstm_distill`
(plus optional `avoid_offsets.npy` / `avoid_actions.npy` for contrastive hinge loss).
"""

from __future__ import annotations

import argparse
import copy
import json
import random
import sys
from pathlib import Path
from typing import Any, Sequence

import numpy as np

_CODE_DIR = Path(__file__).resolve().parents[1]
if str(_CODE_DIR) not in sys.path:
    sys.path.insert(0, str(_CODE_DIR))

from digimon_gym.agents.studio_puzzles import (  # noqa: E402
    StudioPuzzle,
    _advance_interrupts,
    _make_staged_headless_game,
    _normalize_lines,
    load_studio_puzzle,
    permute_staged_puzzle,
)
from model_contract import current_contract  # noqa: E402

SCHEMA = "selfplay-shards-v1"
DEFAULT_FILLER_DECK = ["ST1-01"] * 4 + ["ST1-03"] * 50


def load_puzzle_or_exam(path: str | Path) -> StudioPuzzle:
    """Load either a `studio-puzzle-v1` or `search-exam-v1` JSON file as a `StudioPuzzle`."""
    p = Path(path)
    doc = json.loads(p.read_text(encoding="utf-8"))
    schema = doc.get("schema")
    if schema == "studio-puzzle-v1":
        return load_studio_puzzle(p)
    if schema != "search-exam-v1":
        raise ValueError(f"{p}: unsupported schema {schema!r} (expected studio-puzzle-v1 or search-exam-v1)")

    setup_raw = doc.get("setup") or {}
    decks_raw = setup_raw.get("decks") or []
    if isinstance(decks_raw, dict):
        deck1 = list(decks_raw.get("1") or DEFAULT_FILLER_DECK)
        deck2 = list(decks_raw.get("2") or DEFAULT_FILLER_DECK)
    elif isinstance(decks_raw, list) and len(decks_raw) >= 2:
        deck1 = list(decks_raw[0] or DEFAULT_FILLER_DECK)
        deck2 = list(decks_raw[1] or DEFAULT_FILLER_DECK)
    else:
        deck1, deck2 = list(DEFAULT_FILLER_DECK), list(DEFAULT_FILLER_DECK)

    viewer = int((doc.get("search") or {}).get("viewer", 0))
    seat = 2 if viewer == 1 else 1
    game_seed = int(setup_raw.get("game_seed", 0))
    prefix_actions = [int(a) for a in (setup_raw.get("actions") or [])]

    if isinstance(setup_raw.get("staged"), dict):
        staged = setup_raw["staged"]
        setup: dict[str, Any] = {
            "kind": "staged",
            "state": copy.deepcopy(staged.get("state") or {"memory": 0, "phase": "Main", "turn": 1, "first_player": seat}),
            "zones": copy.deepcopy(staged.get("zones") or {}),
            "prefix_actions": prefix_actions,
        }
    else:
        setup = {
            "kind": "replay_prefix",
            "prefix_actions": prefix_actions,
        }

    expectations = [dict(e) for e in (doc.get("expectations") or []) if isinstance(e, dict)]
    lines = _normalize_lines(doc.get("lines"), expectations=expectations)
    if not any(ln.get("role") == "target" and ln.get("actions") for ln in lines):
        for exp in expectations:
            if exp.get("kind") in ("chosen", "choose", "visit_share_at_least", "visits"):
                plan = [int(a) for a in (exp.get("plan") or [])]
                if plan:
                    lines.insert(
                        0,
                        {
                            "id": "target-expectation",
                            "name": "target-expectation",
                            "role": "target",
                            "actions": plan,
                        },
                    )
                    break

    refutation_triggers = [
        dict(t) for t in (doc.get("refutation_triggers") or []) if isinstance(t, dict)
    ]
    return StudioPuzzle(
        id=str(doc.get("id") or p.stem),
        title=str(doc.get("title") or doc.get("id") or p.stem),
        folder="",
        tags=[],
        decks={"1": deck1, "2": deck2},
        seed=game_seed,
        seat=seat,
        setup=setup,
        rl_goal={"mode": "playout_to_win"},
        path=p,
        lines=lines,
        expectations=expectations,
        refutation_triggers=refutation_triggers,
    )


def _resolve_inputs(
    puzzles_or_exams: Sequence[str | Path | StudioPuzzle] | str | Path | StudioPuzzle,
) -> list[StudioPuzzle]:
    if isinstance(puzzles_or_exams, (str, Path, StudioPuzzle)):
        items: list[str | Path | StudioPuzzle] = [puzzles_or_exams]
    else:
        items = list(puzzles_or_exams)

    resolved: list[StudioPuzzle] = []
    for item in items:
        if isinstance(item, StudioPuzzle):
            resolved.append(item)
            continue
        p = Path(item)
        if p.is_dir():
            for child in sorted(p.rglob("*.json")):
                if child.name == "_library.json":
                    continue
                try:
                    loaded = load_puzzle_or_exam(child)
                except Exception:
                    continue
                if any(ln.get("role") == "target" and ln.get("actions") for ln in loaded.lines):
                    resolved.append(loaded)
        else:
            resolved.append(load_puzzle_or_exam(p))
    return resolved


def _extract_root_avoid_actions(
    puzzle: StudioPuzzle,
    root_legal_set: set[int],
    target_root_action: int,
) -> list[int]:
    avoid_ids: set[int] = set()
    for ln in puzzle.lines:
        if str(ln.get("role")) == "avoid":
            acts = [int(a) for a in (ln.get("actions") or [])]
            if acts:
                avoid_ids.add(acts[0])
    for exp in puzzle.expectations:
        if str(exp.get("kind")) in ("not_chosen", "avoid"):
            plan = [int(a) for a in (exp.get("plan") or [])]
            if plan:
                avoid_ids.add(plan[0])
    return sorted(a for a in avoid_ids if a in root_legal_set and a != int(target_root_action))


def _record_target_trajectory(
    puzzle: StudioPuzzle,
    target_actions: list[int],
    *,
    observation_profile: str,
    target_visit: float,
) -> tuple[list[dict[str, Any]], int]:
    """Step `target_actions` on `RustHeadlessGame`, verifying legality at every step."""
    game = _make_staged_headless_game(puzzle, profile=observation_profile)
    rows: list[dict[str, Any]] = []
    forced_count = 0

    for step_idx, act_id in enumerate(target_actions):
        if not _advance_interrupts(game, puzzle.seat, puzzle.refutation_triggers):
            raise ValueError(f"{puzzle.id}: opponent interrupt loop before step {step_idx}")
        if game.is_game_over:
            raise ValueError(f"{puzzle.id}: game ended prematurely before step {step_idx}")
        if int(game.current_player_id) != int(puzzle.seat):
            raise ValueError(
                f"{puzzle.id}: expected solver seat {puzzle.seat} at step {step_idx}, got {game.current_player_id}"
            )

        mask = np.asarray(game.get_action_mask())
        legal_acts = np.flatnonzero(mask > 0.5).astype(np.int32)
        legal_set = {int(a) for a in legal_acts.tolist()}
        if int(act_id) not in legal_set:
            raise ValueError(
                f"{puzzle.id}: target action {act_id} at step {step_idx} is illegal (legal={sorted(legal_set)})"
            )

        if len(legal_acts) == 1:
            forced_count += 1
            game.step(int(act_id))
            continue

        obs = np.asarray(game.get_board_tensor(int(puzzle.seat)), dtype=np.float32)
        visits = np.where(legal_acts == int(act_id), float(target_visit), 0.0).astype(np.float32)
        avoid_acts = (
            _extract_root_avoid_actions(puzzle, legal_set, int(act_id)) if step_idx == 0 else []
        )
        rows.append(
            {
                "obs": obs,
                "actions": legal_acts,
                "visits": visits,
                "avoid_actions": np.asarray(avoid_acts, dtype=np.int32),
            }
        )
        game.step(int(act_id))

    if not game.is_game_over:
        _advance_interrupts(game, puzzle.seat, puzzle.refutation_triggers)

    return rows, forced_count


def emit_puzzle_shards(
    puzzles_or_exams: Sequence[str | Path | StudioPuzzle] | str | Path | StudioPuzzle,
    out_dir: str | Path,
    permutations_per_puzzle: int = 16,
    observation_profile: str = "standard_lite_v2",
    seed: int = 0,
    target_visit: float = 100.0,
    max_distractors: int = 2,
) -> dict[str, Any]:
    """Emit `selfplay-shards-v1` supervised shards (with optional `avoid_*.npy`) into `out_dir`."""
    if permutations_per_puzzle < 0:
        raise ValueError(f"permutations_per_puzzle must be >= 0, got {permutations_per_puzzle}")

    contract = current_contract(observation_profile)
    obs_dim = int(contract["tensor_size"])
    action_dim = int(contract["action_space_size"])

    puzzles = _resolve_inputs(puzzles_or_exams)
    if not puzzles:
        raise ValueError("No puzzles or search exams with target lines found")

    rng = random.Random(seed)
    obs_rows: list[np.ndarray] = []
    policy_offsets: list[int] = [0]
    policy_actions: list[np.ndarray] = []
    policy_visits: list[np.ndarray] = []
    avoid_offsets: list[int] = [0]
    avoid_actions: list[np.ndarray] = []
    z_list: list[float] = []
    terminated_list: list[int] = []
    game_ids: list[int] = []
    movers: list[int] = []
    weights: list[float] = []

    next_game_id = 0
    puzzles_emitted = 0
    forced_decisions_total = 0

    for puzzle in puzzles:
        target_lines = [
            ln for ln in puzzle.lines if str(ln.get("role")) == "target" and ln.get("actions")
        ]
        if not target_lines:
            continue
        puzzles_emitted += 1

        can_permute = (
            (puzzle.setup or {}).get("kind", "staged") == "staged"
            and not (puzzle.setup or {}).get("prefix_actions")
            and permutations_per_puzzle > 0
        )
        variants: list[StudioPuzzle] = [puzzle]
        if can_permute:
            for _ in range(permutations_per_puzzle):
                variants.append(permute_staged_puzzle(puzzle, rng, max_distractors=max_distractors))

        for var_idx, var_puzzle in enumerate(variants):
            var_targets = [
                ln for ln in var_puzzle.lines if str(ln.get("role")) == "target" and ln.get("actions")
            ]
            for t_ln in var_targets:
                t_acts = [int(a) for a in t_ln["actions"]]
                try:
                    traj_rows, forced_cnt = _record_target_trajectory(
                        var_puzzle,
                        t_acts,
                        observation_profile=observation_profile,
                        target_visit=target_visit,
                    )
                except Exception:
                    if var_idx == 0:
                        raise
                    continue
                if not traj_rows:
                    continue
                forced_decisions_total += forced_cnt
                mover_id = max(0, min(1, int(var_puzzle.seat) - 1))
                for step_row in traj_rows:
                    obs_rows.append(step_row["obs"])
                    acts_arr = step_row["actions"]
                    vis_arr = step_row["visits"]
                    av_arr = step_row["avoid_actions"]
                    policy_actions.append(acts_arr)
                    policy_visits.append(vis_arr)
                    policy_offsets.append(policy_offsets[-1] + int(len(acts_arr)))
                    avoid_actions.append(av_arr)
                    avoid_offsets.append(avoid_offsets[-1] + int(len(av_arr)))
                    z_list.append(1.0)
                    terminated_list.append(1)
                    game_ids.append(next_game_id)
                    movers.append(mover_id)
                    weights.append(1.0)
                next_game_id += 1

    total_rows = len(obs_rows)
    if total_rows == 0:
        raise ValueError("No valid non-forced trajectory rows emitted from supplied puzzles")

    out_path = Path(out_dir)
    shard_dir = out_path / "shard-000"
    shard_dir.mkdir(parents=True, exist_ok=True)

    obs_np = np.stack(obs_rows, axis=0).astype(np.float32)
    if obs_np.shape != (total_rows, obs_dim):
        raise RuntimeError(f"Unexpected obs shape {obs_np.shape}, expected {(total_rows, obs_dim)}")

    pol_off_np = np.asarray(policy_offsets, dtype=np.int64)
    pol_act_np = np.concatenate(policy_actions).astype(np.int32)
    pol_vis_np = np.concatenate(policy_visits).astype(np.float32)
    z_np = np.asarray(z_list, dtype=np.float32)
    term_np = np.asarray(terminated_list, dtype=np.uint8)
    gid_np = np.asarray(game_ids, dtype=np.int32)
    mover_np = np.asarray(movers, dtype=np.uint8)
    weight_np = np.asarray(weights, dtype=np.float32)

    np.save(shard_dir / "obs.npy", obs_np)
    np.save(shard_dir / "policy_offsets.npy", pol_off_np)
    np.save(shard_dir / "policy_actions.npy", pol_act_np)
    np.save(shard_dir / "policy_visits.npy", pol_vis_np)
    np.save(shard_dir / "z.npy", z_np)
    np.save(shard_dir / "terminated.npy", term_np)
    np.save(shard_dir / "game_id.npy", gid_np)
    np.save(shard_dir / "mover.npy", mover_np)
    np.save(shard_dir / "policy_weight.npy", weight_np)

    av_off_np = np.asarray(avoid_offsets, dtype=np.int64)
    av_act_np = (
        np.concatenate(avoid_actions).astype(np.int32)
        if avoid_offsets[-1] > 0
        else np.zeros(0, dtype=np.int32)
    )
    if len(av_act_np) > 0:
        np.save(shard_dir / "avoid_offsets.npy", av_off_np)
        np.save(shard_dir / "avoid_actions.npy", av_act_np)

    manifest = {
        "schema": SCHEMA,
        "source": "emit_puzzle_shards",
        "evaluation_only": False,
        "information": {
            "regime": "open_decklist",
            "knowledge_schema": 2,
            "prior_hash": None,
            "legality_hash": None,
            "prior_provenance": None,
        },
        "obs_dim": obs_dim,
        "action_dim": action_dim,
        "observation_profile": observation_profile,
        "model_contract": contract,
        "value_target": "terminal_outcome",
        "seed": int(seed),
        "puzzles": int(puzzles_emitted),
        "permutations_per_puzzle": int(permutations_per_puzzle),
        "games": int(next_game_id),
        "rows": int(total_rows),
        "avoid_edges": int(len(av_act_np)),
        "forced_decisions_total": int(forced_decisions_total),
        "shards": [{"dir": "shard-000", "rows": int(total_rows)}],
    }
    manifest_path = out_path / "manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return manifest


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    p.add_argument(
        "--inputs",
        type=Path,
        nargs="+",
        required=True,
        help="studio-puzzle-v1 / search-exam-v1 JSON files or directories",
    )
    p.add_argument("--out", type=Path, required=True, help="Output selfplay-shards-v1 directory")
    p.add_argument("--permutations-per-puzzle", type=int, default=16)
    p.add_argument("--observation-profile", type=str, default="standard_lite_v2")
    p.add_argument("--seed", type=int, default=0)
    p.add_argument("--target-visit", type=float, default=100.0)
    p.add_argument("--max-distractors", type=int, default=2)
    return p


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    summary = emit_puzzle_shards(
        puzzles_or_exams=list(args.inputs),
        out_dir=args.out,
        permutations_per_puzzle=args.permutations_per_puzzle,
        observation_profile=args.observation_profile,
        seed=args.seed,
        target_visit=args.target_visit,
        max_distractors=args.max_distractors,
    )
    print(
        f"emit_puzzle_shards: wrote {summary['rows']} rows across {summary['games']} trajectories "
        f"({summary['avoid_edges']} avoid edges) -> {args.out}",
        flush=True,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
