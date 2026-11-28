"""Studio Puzzle RL Curriculum Loader & Reset Wrapper (add-board-state-studio).

Loads canonical `studio-puzzle-v1` JSON files recursively from `qa/puzzles/`
(supporting folder and tag filtering) and wraps `DigimonEnv` so RL training
episodes can reset directly onto staged boards while preserving the configured
`tensor_profile`, observation space, and `ActionMasker` contract.

Supports both RL goal modes:
- `playout_to_win`: play from staged board to standard terminal win/loss.
- `bounded_horizon`: terminate early with +1.0 when `win_assertions` are
  satisfied within `max_turns` / `max_actions`, or -1.0 on horizon expiry/loss.
"""

from __future__ import annotations

import json
import random
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Optional, Sequence

import gymnasium


@dataclass
class StudioPuzzle:
    id: str
    title: str
    folder: str
    tags: list[str]
    decks: dict[str, list[str]]
    seed: int
    seat: int
    setup: dict[str, Any]
    rl_goal: dict[str, Any] = field(default_factory=lambda: {"mode": "playout_to_win"})
    path: Optional[Path] = None

    def to_scenario_dict(self) -> dict[str, Any]:
        return {
            "state": self.setup.get(
                "state",
                {"memory": 0, "phase": "Main", "turn": 1, "first_player": self.seat},
            ),
            "zones": self.setup.get("zones", {}),
        }


def load_studio_puzzle(path: str | Path) -> StudioPuzzle:
    p = Path(path)
    doc = json.loads(p.read_text(encoding="utf-8"))
    if doc.get("schema") != "studio-puzzle-v1":
        raise ValueError(f"{p} is not a studio-puzzle-v1 file (got {doc.get('schema')!r})")
    decks = doc.get("decks") or {}
    if "1" not in decks or "2" not in decks:
        raise ValueError(f"{p} missing '1' or '2' deck")
    return StudioPuzzle(
        id=str(doc["id"]),
        title=str(doc.get("title") or doc["id"]),
        folder=str(doc.get("folder") or ""),
        tags=[str(t) for t in doc.get("tags", [])],
        decks={"1": list(decks["1"]), "2": list(decks["2"])},
        seed=int(doc.get("seed", 0)),
        seat=int(doc.get("seat", 1)),
        setup=dict(doc.get("setup") or {}),
        rl_goal=dict(doc.get("rl_goal") or {"mode": "playout_to_win"}),
        path=p,
    )


def load_studio_puzzles_dir(
    root: str | Path,
    *,
    folder: Optional[str] = None,
    tags: Optional[Sequence[str]] = None,
) -> list[StudioPuzzle]:
    root_path = Path(root)
    if not root_path.exists():
        return []
    wanted_folder = folder.strip("/") if folder else None
    wanted_tags = set(t.strip().lower() for t in (tags or []) if t.strip())

    out: list[StudioPuzzle] = []
    for p in sorted(root_path.rglob("*.json")):
        if p.name == "_library.json":
            continue
        try:
            puzzle = load_studio_puzzle(p)
        except Exception:
            continue
        rel_folder = str(p.parent.relative_to(root_path)).replace("\\", "/")
        if rel_folder == ".":
            rel_folder = ""
        effective_folder = puzzle.folder or rel_folder
        if wanted_folder is not None:
            if effective_folder != wanted_folder and not effective_folder.startswith(wanted_folder + "/"):
                continue
        if wanted_tags and not wanted_tags.issubset({t.lower() for t in puzzle.tags}):
            continue
        out.append(puzzle)
    return out


def _evaluate_goal_assertion(game: Any, assertion: dict[str, Any]) -> bool:
    """Evaluate a lightweight engine assertion against a live RustHeadlessGame
    or RustDebugGame via `to_ui_json()` / `get_rl_state()`."""
    kind = assertion.get("kind", "")
    ui = game.to_ui_json()
    rl = game.get_rl_state() if hasattr(game, "get_rl_state") else {}

    if kind == "memory_equals":
        return int(rl.get("memory", ui.get("memoryGauge", 0))) == int(assertion["value"])
    if kind == "memory_at_least":
        return int(rl.get("memory", ui.get("memoryGauge", 0))) >= int(assertion["value"])
    if kind == "zone_count":
        pid = int(assertion.get("player", 1))
        p = ui.get(f"player{pid}", {})
        zone = assertion.get("zone", "")
        mapping = {
            "hand": len(p.get("handIds", [])),
            "security": int(p.get("securityCount", 0)),
            "trash": len(p.get("trashIds", [])),
            "field": len(p.get("battleArea", [])),
            "deck": int(p.get("deckCount", 0)),
        }
        return mapping.get(zone) == int(assertion["value"])
    if kind == "stack_top":
        pid = int(assertion.get("player", 1))
        ba = ui.get(f"player{pid}", {}).get("battleArea", [])
        idx = int(assertion.get("field_index", 0))
        if idx < 0 or idx >= len(ba):
            return False
        return ba[idx].get("topCardId") == assertion.get("card_id")
    if kind == "effective_dp":
        pid = int(assertion.get("player", 1))
        ba = ui.get(f"player{pid}", {}).get("battleArea", [])
        idx = int(assertion.get("field_index", 0))
        if idx < 0 or idx >= len(ba):
            return False
        return int(ba[idx].get("dp", -1)) == int(assertion["value"])
    return False


class PuzzleResetWrapper(gymnasium.Wrapper):
    """Gymnasium wrapper that resets `DigimonEnv` onto sampled Studio puzzles.

    Preserves `observation_space`, `action_space`, `tensor_profile`, and
    `action_mask()` / `get_action_mask()` for `ActionMasker` compatibility.
    """

    def __init__(
        self,
        env: gymnasium.Env,
        puzzles: Sequence[StudioPuzzle],
        *,
        puzzle_prob: float = 1.0,
        seed: Optional[int] = None,
    ):
        super().__init__(env)
        if not puzzles:
            raise ValueError("PuzzleResetWrapper requires at least one StudioPuzzle")
        self._puzzles = list(puzzles)
        self._puzzle_prob = float(puzzle_prob)
        self._rng = random.Random(seed)
        self._active_puzzle: Optional[StudioPuzzle] = None
        self._puzzle_actions_taken = 0
        self._start_turn = 1

    def action_mask(self):
        return self.env.unwrapped.action_mask()

    def get_action_mask(self):
        return self.env.unwrapped.get_action_mask()

    def reset(self, *, seed: Optional[int] = None, options: Optional[dict[str, Any]] = None):
        if seed is not None:
            self._rng.seed(seed)
        self._active_puzzle = None
        self._puzzle_actions_taken = 0

        if self._rng.random() > self._puzzle_prob:
            return self.env.reset(seed=seed, options=options)

        puzzle = self._rng.choice(self._puzzles)
        self._active_puzzle = puzzle
        merged_opts = dict(options or {})
        merged_opts["deck1"] = list(puzzle.decks["1"])
        merged_opts["deck2"] = list(puzzle.decks["2"])

        obs, info = self.env.reset(seed=puzzle.seed if seed is None else seed, options=merged_opts)
        game = getattr(self.env.unwrapped, "runner", None) or getattr(self.env.unwrapped, "game", None)
        if game is None or not hasattr(game, "apply_scenario"):
            raise RuntimeError(
                "DigimonEnv's RustHeadlessGame does not expose apply_scenario(); rebuild digimon_engine wheel"
            )

        if hasattr(game, "skip_mulligan"):
            game.skip_mulligan()
        game.apply_scenario(json.dumps(puzzle.to_scenario_dict()))

        for aid in puzzle.setup.get("prefix_actions", []):
            if game.is_game_over:
                break
            game.step(int(aid))

        rl_state = game.get_rl_state() if hasattr(game, "get_rl_state") else {}
        self._start_turn = int(rl_state.get("turn_count", 1))

        # Recompute fresh observation and action mask from the staged board.
        obs = game.get_board_tensor(None)
        mask = self.env.unwrapped.action_mask()
        info = dict(info)
        info["action_mask"] = mask
        info["studio_puzzle_id"] = puzzle.id
        info["studio_puzzle_goal_mode"] = puzzle.rl_goal.get("mode", "playout_to_win")
        return obs, info

    def step(self, action):
        obs, reward, terminated, truncated, info = self.env.step(action)
        puzzle = self._active_puzzle
        if puzzle is None:
            return obs, reward, terminated, truncated, info

        self._puzzle_actions_taken += 1
        goal = puzzle.rl_goal or {}
        if goal.get("mode") != "bounded_horizon":
            return obs, reward, terminated, truncated, info

        game = getattr(self.env.unwrapped, "runner", None) or getattr(self.env.unwrapped, "game", None)
        if game is None:
            return obs, reward, terminated, truncated, info

        win_assertions = goal.get("win_assertions") or []
        if win_assertions and all(_evaluate_goal_assertion(game, a) for a in win_assertions):
            info = dict(info)
            info["studio_puzzle_solved"] = True
            return obs, 1.0, True, False, info

        if terminated:
            return obs, reward, terminated, truncated, info

        max_actions = goal.get("max_actions")
        max_turns = goal.get("max_turns")
        rl_state = game.get_rl_state() if hasattr(game, "get_rl_state") else {}
        cur_turn = int(rl_state.get("turn_count", self._start_turn))

        expired = False
        if max_actions is not None and self._puzzle_actions_taken >= int(max_actions):
            expired = True
        if max_turns is not None and (cur_turn - self._start_turn) >= int(max_turns):
            expired = True

        if expired:
            info = dict(info)
            info["studio_puzzle_solved"] = False
            info["studio_puzzle_horizon_expired"] = True
            return obs, -1.0, True, False, info

        return obs, reward, terminated, truncated, info
