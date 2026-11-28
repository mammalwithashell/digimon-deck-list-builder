"""5-Stage Dense Verifiable Engine Reward for Digimon Tactician GRPO (`verifiable_reward`).

Evaluates LLM tactical completions directly in-process against `digimon_engine`
(`RustDebugGame` / `RustHeadlessGame` + `studio_puzzles` + `CardAdmissibility`)
so RunPod GPU workers can score 8–16 GRPO completions in milliseconds with zero
subprocess overhead, while also supporting an optional `StdioMcpClient` backend.

Reward Stages (cumulative range `[0.00, 1.00]`):
- Stage 1 (`+0.15`): Valid JSON (after stripping `<think>...</think>` & fences)
  and valid `studio-puzzle-v1` / counter-puzzle / `tactician-solve-v1` schema.
- Stage 2 (`+0.15` -> `0.30`): Card admissibility in `data/card_admissibility.json`
  (allowing `TOKEN_*` field tokens) and Challenger/Defender deck multiset purity.
- Stage 3 (`+0.20` -> `0.50`): Engine board legality via `RustDebugGame`
  (`skip_mulligan()` + `apply_scenario()` + legal `prefix_actions` + `validate()`).
- Stage 4 (`+0.35` -> `0.85`): Declared `target` line(s) step legally on
  `RustHeadlessGame` against `[All Turns]` adversary interrupts driven by
  `select_refutation_action` (`refutation_triggers`) + `greedy_action` fallback,
  achieving terminal win for `solver_seat` (or satisfying all `rl_goal.win_assertions`
  in `bounded_horizon` mode).
- Stage 5 (`+0.15` -> `1.00`): At least one non-prefix `avoid` line is legal at
  its root action and fails to satisfy the goal / gets refuted by the defender.
"""

from __future__ import annotations

from collections import Counter
import copy
from dataclasses import dataclass, field
import json
from pathlib import Path
import re
from typing import Any, Mapping, Optional, Sequence

import numpy as np

from digimon_gym.agents.card_admissibility import CardAdmissibility
from digimon_gym.agents.studio_puzzles import (
    _advance_interrupts,
    _is_digi_egg,
    _normalize_lines,
)
from tools.puzzle_lab import (
    PUZZLE_SCHEMA,
    SEARCH_EXAM_SCHEMA,
    SLUG_RE,
    VALID_LINE_ROLES,
    VALID_RL_GOAL_MODES,
    VALID_SETUP_KINDS,
    evaluate_verify_puzzle_result,
    resolve_repo_root,
    slugify_puzzle_id,
)

TACTICIAN_SOLVE_SCHEMA = "tactician-solve-v1"
TACTICIAN_COUNTER_SCHEMA = "tactician-counter-puzzle-v1"

TASK_SOLVE_POSITION = "solve_position"
TASK_COMPOSE_COUNTER_PUZZLE = "compose_counter_puzzle"

STAGE1_WEIGHT = 0.15
STAGE2_WEIGHT = 0.15
STAGE3_WEIGHT = 0.20
STAGE4_WEIGHT = 0.35
STAGE5_WEIGHT = 0.15

_THINK_OPEN_CLOSE_RE = re.compile(r"<think>.*?</think>", flags=re.DOTALL | re.IGNORECASE)
_UNCLOSED_THINK_RE = re.compile(r"^\s*<think>.*?(?=\{)", flags=re.DOTALL | re.IGNORECASE)
_JSON_FENCE_RE = re.compile(r"```(?:json)?\s*(.*?)\s*```", flags=re.DOTALL | re.IGNORECASE)


def strip_thinking_blocks(text: str) -> str:
    """Strip Qwen3 `<think>...</think>` reasoning blocks and surrounding whitespace."""
    if not text:
        return ""
    cleaned = _THINK_OPEN_CLOSE_RE.sub("", str(text)).strip()
    if cleaned.lower().startswith("<think>") and "{" in cleaned:
        cleaned = _UNCLOSED_THINK_RE.sub("", cleaned, count=1).strip()
    return cleaned


def parse_completion_json(text: str) -> Optional[dict[str, Any]]:
    """Strip `<think>...</think>` blocks and markdown fences, returning a JSON dict or `None`."""
    cleaned = strip_thinking_blocks(text)
    if not cleaned:
        return None

    try:
        loaded = json.loads(cleaned)
        if isinstance(loaded, dict):
            return loaded
    except Exception:
        pass

    for match in _JSON_FENCE_RE.finditer(cleaned):
        candidate = match.group(1).strip()
        try:
            loaded = json.loads(candidate)
            if isinstance(loaded, dict):
                return loaded
        except Exception:
            continue

    first_brace = cleaned.find("{")
    last_brace = cleaned.rfind("}")
    if first_brace != -1 and last_brace > first_brace:
        snippet = cleaned[first_brace : last_brace + 1]
        try:
            loaded = json.loads(snippet)
            if isinstance(loaded, dict):
                return loaded
        except Exception:
            return None
    return None


@dataclass
class RewardBreakdown:
    """Detailed per-stage verifiable reward diagnostics for a single completion."""

    total: float = 0.0
    stage1_json_schema: float = 0.0
    stage2_card_admissibility: float = 0.0
    stage3_board_legality: float = 0.0
    stage4_target_verified: float = 0.0
    stage5_avoid_refuted: float = 0.0
    stage_reached: int = 0
    failure_stage: Optional[int] = None
    reason: str = ""
    diagnostics: dict[str, Any] = field(default_factory=dict)

    def finalize(self) -> "RewardBreakdown":
        raw_sum = (
            self.stage1_json_schema
            + self.stage2_card_admissibility
            + self.stage3_board_legality
            + self.stage4_target_verified
            + self.stage5_avoid_refuted
        )
        self.total = round(float(raw_sum), 6)
        return self

    def to_dict(self) -> dict[str, Any]:
        return {
            "total": self.total,
            "stage1_json_schema": self.stage1_json_schema,
            "stage2_card_admissibility": self.stage2_card_admissibility,
            "stage3_board_legality": self.stage3_board_legality,
            "stage4_target_verified": self.stage4_target_verified,
            "stage5_avoid_refuted": self.stage5_avoid_refuted,
            "stage_reached": self.stage_reached,
            "failure_stage": self.failure_stage,
            "reason": self.reason,
            "diagnostics": dict(self.diagnostics),
        }


def _extract_completion_text(completion: Any) -> str:
    """Unwrap TRL string completions or conversational message lists."""
    if isinstance(completion, str):
        return completion
    if isinstance(completion, Mapping):
        return str(completion.get("content") or "")
    if isinstance(completion, Sequence) and not isinstance(completion, (bytes, bytearray)):
        parts: list[str] = []
        for item in completion:
            if isinstance(item, Mapping):
                if item.get("role") in (None, "assistant"):
                    parts.append(str(item.get("content") or ""))
            elif isinstance(item, str):
                parts.append(item)
        return "\n".join(p for p in parts if p)
    return str(completion or "")


def _coerce_meta_dict(raw_meta: Any) -> dict[str, Any]:
    if raw_meta is None:
        return {}
    if isinstance(raw_meta, Mapping):
        return dict(raw_meta)
    if isinstance(raw_meta, str) and raw_meta.strip():
        try:
            loaded = json.loads(raw_meta)
            if isinstance(loaded, dict):
                return loaded
        except Exception:
            return {}
    return {}


def _is_strict_int(val: Any) -> bool:
    return isinstance(val, int) and not isinstance(val, bool)


def _convert_search_exam_to_puzzle_dict(doc: Mapping[str, Any]) -> dict[str, Any]:
    """Convert a `search-exam-v1` dict into a `studio-puzzle-v1` dict for unified scoring."""
    setup = dict(doc.get("setup") or {})
    raw_decks = setup.get("decks") or []
    decks: dict[str, Any] = {}
    if isinstance(raw_decks, list) and len(raw_decks) >= 2:
        decks = {"1": list(raw_decks[0]), "2": list(raw_decks[1])}
    elif isinstance(raw_decks, Mapping):
        decks = {"1": list(raw_decks.get("1") or []), "2": list(raw_decks.get("2") or [])}

    viewer = int((doc.get("search") or {}).get("viewer", 0))
    seat = viewer + 1 if viewer in (0, 1) else 1
    seed = int(setup.get("game_seed", 0))
    prefix_actions = list(setup.get("actions") or [])

    if isinstance(setup.get("staged"), Mapping):
        staged = setup["staged"]
        puzzle_setup: dict[str, Any] = {
            "kind": "staged",
            "state": dict(staged.get("state") or {}),
            "zones": dict(staged.get("zones") or {}),
            "prefix_actions": prefix_actions,
        }
    else:
        puzzle_setup = {
            "kind": "replay_prefix",
            "prefix_actions": prefix_actions,
        }

    expectations = [dict(e) for e in (doc.get("expectations") or []) if isinstance(e, Mapping)]
    lines = _normalize_lines(doc.get("lines"), expectations=expectations)
    out: dict[str, Any] = {
        "schema": PUZZLE_SCHEMA,
        "id": str(doc.get("id") or "search-exam"),
        "title": str(doc.get("title") or doc.get("id") or "Search Exam"),
        "description": str(doc.get("why") or doc.get("description") or ""),
        "folder": "",
        "tags": list(doc.get("tags") or []),
        "source": dict(doc.get("source") or {"kind": "composed"}),
        "decks": decks,
        "seed": seed,
        "seat": seat,
        "setup": puzzle_setup,
        "lines": lines,
        "expectations": expectations,
        "assertions": dict(doc.get("assertions") or {"engine": [], "ui": []}),
        "rl_goal": dict(doc.get("rl_goal") or {"mode": "playout_to_win", "win_assertions": []}),
    }
    if doc.get("refutation_triggers"):
        out["refutation_triggers"] = list(doc["refutation_triggers"])
    return out


def _validate_lines_schema(raw_lines: Any) -> tuple[bool, list[dict[str, Any]], str]:
    """Validate and normalize `lines` into `[{id, name, role, actions}, ...]`."""
    if isinstance(raw_lines, Mapping):
        norm: list[dict[str, Any]] = []
        for idx, (lname, acts) in enumerate(raw_lines.items(), 1):
            if not isinstance(acts, list) or not acts or not all(_is_strict_int(a) for a in acts):
                return False, [], f"Line {lname!r} must have a non-empty list[int] of action IDs"
            lower = str(lname).lower()
            role = "avoid" if ("avoid" in lower or "trap" in lower) else ("target" if idx == 1 else "alternative")
            norm.append(
                {
                    "id": slugify_puzzle_id(str(lname), f"line-{idx}"),
                    "name": str(lname),
                    "role": role,
                    "actions": [int(a) for a in acts],
                }
            )
        if not any(ln["role"] == "target" for ln in norm):
            return False, [], "Lines mapping must include at least one target line"
        return True, norm, ""

    if not isinstance(raw_lines, list) or not raw_lines:
        return False, [], "Completion 'lines' must be a non-empty list of line objects"

    norm_list: list[dict[str, Any]] = []
    for idx, ln in enumerate(raw_lines, 1):
        if not isinstance(ln, Mapping):
            return False, [], f"lines[{idx - 1}] is not an object"
        role = str(ln.get("role") or "")
        if role not in VALID_LINE_ROLES:
            return False, [], f"lines[{idx - 1}] has invalid role {role!r}"
        acts = ln.get("actions")
        if not isinstance(acts, list) or not acts or not all(_is_strict_int(a) for a in acts):
            return False, [], f"lines[{idx - 1}] ('{ln.get('id', idx)}') must have non-empty list[int] 'actions'"
        lid = slugify_puzzle_id(str(ln.get("id") or ln.get("name") or f"{role}-{idx}"), f"{role}-{idx}")
        norm_list.append(
            {
                "id": lid,
                "name": str(ln.get("name") or lid),
                "role": role,
                "actions": [int(a) for a in acts],
            }
        )

    if not any(ln["role"] == "target" for ln in norm_list):
        return False, [], "Completion 'lines' must include at least one 'target' line"
    return True, norm_list, ""


def _fill_remaining_library_zones(
    deck: Sequence[str],
    raw_zone: Mapping[str, Any],
) -> dict[str, Any]:
    """Ensure `zones[seat]` has canonical keys and populates `deck_top`/`egg_deck` if omitted."""
    zone = copy.deepcopy(dict(raw_zone))
    field_norm: list[dict[str, Any]] = []
    for perm in zone.get("field") or []:
        if isinstance(perm, Mapping):
            entry: dict[str, Any] = {
                "stack": [str(c) for c in (perm.get("stack") or []) if c is not None],
                "is_suspended": bool(perm.get("is_suspended", False)),
                "turn_played": int(perm.get("turn_played", 0)),
            }
            if perm.get("linked_cards"):
                entry["linked_cards"] = list(perm["linked_cards"])
            field_norm.append(entry)
        elif isinstance(perm, list):
            field_norm.append(
                {
                    "stack": [str(c) for c in perm if c is not None],
                    "is_suspended": False,
                    "turn_played": 0,
                }
            )
    zone["field"] = field_norm

    breeding_raw = zone.get("breeding")
    if isinstance(breeding_raw, Mapping):
        zone["breeding"] = [str(c) for c in (breeding_raw.get("stack") or []) if c] or None
    elif isinstance(breeding_raw, list):
        zone["breeding"] = [str(c) for c in breeding_raw if c] or None
    else:
        zone["breeding"] = None

    for key in ("hand", "security", "trash"):
        items: list[str] = []
        for x in zone.get(key) or []:
            if isinstance(x, Mapping) and x.get("card_id"):
                items.append(str(x["card_id"]))
            elif x:
                items.append(str(x))
        zone[key] = items

    placed: Counter[str] = Counter()
    if zone["breeding"]:
        for cid in zone["breeding"]:
            if not cid.startswith("TOKEN_"):
                placed[cid] += 1
    for perm in zone["field"]:
        for cid in perm["stack"]:
            if not cid.startswith("TOKEN_"):
                placed[cid] += 1
        for lc in perm.get("linked_cards") or []:
            lcid = str(lc["card_id"]) if isinstance(lc, Mapping) and lc.get("card_id") else str(lc)
            if not lcid.startswith("TOKEN_"):
                placed[lcid] += 1
    for key in ("hand", "security", "trash"):
        for cid in zone[key]:
            if not cid.startswith("TOKEN_"):
                placed[cid] += 1

    deck_counts = Counter(str(c) for c in deck if c and not str(c).startswith("TOKEN_"))
    if "deck_top" not in zone or zone.get("deck_top") is None:
        rem_main: list[str] = []
        for cid, cnt in deck_counts.items():
            if _is_digi_egg(cid):
                continue
            left = max(0, cnt - placed.get(cid, 0))
            rem_main.extend([cid] * left)
        zone["deck_top"] = rem_main
    else:
        zone["deck_top"] = [str(c) for c in (zone.get("deck_top") or []) if c]

    if "egg_deck" not in zone or zone.get("egg_deck") is None:
        rem_eggs: list[str] = []
        for cid, cnt in deck_counts.items():
            if not _is_digi_egg(cid):
                continue
            left = max(0, cnt - placed.get(cid, 0))
            rem_eggs.extend([cid] * left)
        zone["egg_deck"] = rem_eggs
    else:
        zone["egg_deck"] = [str(c) for c in (zone.get("egg_deck") or []) if c]

    return zone


def _resolve_candidate_puzzle(
    parsed: Mapping[str, Any],
    meta: Mapping[str, Any],
    task_kind: Optional[str],
) -> tuple[bool, Optional[dict[str, Any]], list[str], str]:
    """Validate Stage 1 schema and resolve `parsed` (+ `meta`) into a canonical puzzle dict.

    Returns `(ok, canonical_puzzle_dict, extra_referenced_cards, error_reason)`.
    """
    extra_cards: list[str] = []
    if "referenced_cards" in parsed:
        rc = parsed.get("referenced_cards")
        if not isinstance(rc, list) or not all(isinstance(c, str) and c.strip() for c in rc):
            return False, None, [], "'referenced_cards' must be list[str] when provided"
        extra_cards.extend(str(c).strip() for c in rc)

    schema = parsed.get("schema")
    effective_task = str(task_kind or parsed.get("task_kind") or meta.get("task_kind") or "").strip()

    # Case 1: Full `search-exam-v1` document
    if schema == SEARCH_EXAM_SCHEMA:
        if not isinstance(parsed.get("setup"), Mapping):
            return False, None, [], "search-exam-v1 missing 'setup' object"
        converted = _convert_search_exam_to_puzzle_dict(parsed)
        return _resolve_candidate_puzzle(converted, meta, effective_task)

    # Case 2: Full `studio-puzzle-v1` document
    if schema == PUZZLE_SCHEMA or ("setup" in parsed and "decks" in parsed):
        if schema is not None and schema != PUZZLE_SCHEMA:
            return False, None, [], f"Unsupported puzzle schema {schema!r}"
        pid = str(parsed.get("id") or "").strip()
        if not pid or not SLUG_RE.match(pid):
            return False, None, [], f"Invalid or missing kebab-case puzzle 'id': {pid!r}"
        if not str(parsed.get("title") or "").strip():
            return False, None, [], "Puzzle 'title' must be a non-empty string"

        seat_val = parsed.get("seat", 1)
        if not _is_strict_int(seat_val) or int(seat_val) not in (1, 2):
            return False, None, [], f"Puzzle 'seat' must be 1 or 2, got {seat_val!r}"

        decks = parsed.get("decks")
        if not isinstance(decks, Mapping):
            return False, None, [], "Puzzle 'decks' must be an object with '1' and '2' lists"
        d1 = decks.get("1")
        d2 = decks.get("2")
        if (
            not isinstance(d1, list)
            or not d1
            or not all(isinstance(c, str) and c.strip() for c in d1)
            or not isinstance(d2, list)
            or not d2
            or not all(isinstance(c, str) and c.strip() for c in d2)
        ):
            return False, None, [], "Puzzle 'decks.1' and 'decks.2' must be non-empty list[str]"

        setup = parsed.get("setup")
        if not isinstance(setup, Mapping) or setup.get("kind") not in VALID_SETUP_KINDS:
            return False, None, [], f"Puzzle 'setup.kind' must be one of {sorted(VALID_SETUP_KINDS)}"

        prefix_raw = setup.get("prefix_actions") or []
        if not isinstance(prefix_raw, list) or not all(_is_strict_int(a) for a in prefix_raw):
            return False, None, [], "Puzzle 'setup.prefix_actions' must be list[int]"

        if setup["kind"] == "staged":
            state = setup.get("state")
            zones = setup.get("zones")
            if not isinstance(state, Mapping):
                return False, None, [], "Staged puzzle missing 'setup.state'"
            for req_k in ("memory", "phase", "turn", "first_player"):
                if req_k not in state:
                    return False, None, [], f"Staged puzzle 'setup.state' missing {req_k!r}"
            if (
                not _is_strict_int(state["memory"])
                or not isinstance(state["phase"], str)
                or not state["phase"].strip()
                or not _is_strict_int(state["turn"])
                or not _is_strict_int(state["first_player"])
                or int(state["first_player"]) not in (1, 2)
            ):
                return False, None, [], "Staged puzzle 'setup.state' field types are invalid"
            if (
                not isinstance(zones, Mapping)
                or not isinstance(zones.get("1"), Mapping)
                or not isinstance(zones.get("2"), Mapping)
            ):
                return False, None, [], "Staged puzzle 'setup.zones' must contain '1' and '2' objects"

            for sk in ("1", "2"):
                z_s = zones[sk]
                if "field" in z_s and z_s["field"] is not None:
                    if not isinstance(z_s["field"], list):
                        return False, None, [], f"zones[{sk!r}].field must be a list"
                    for f_i, perm in enumerate(z_s["field"]):
                        if not isinstance(perm, Mapping) or not isinstance(perm.get("stack"), list):
                            return (
                                False,
                                None,
                                [],
                                f"zones[{sk!r}].field[{f_i}] must be an object with 'stack' list",
                            )

        ok_lines, norm_lines, line_err = _validate_lines_schema(parsed.get("lines"))
        if not ok_lines:
            return False, None, [], line_err

        rl_goal = parsed.get("rl_goal") or {"mode": "playout_to_win", "win_assertions": []}
        if not isinstance(rl_goal, Mapping) or rl_goal.get("mode", "playout_to_win") not in VALID_RL_GOAL_MODES:
            return False, None, [], f"Invalid 'rl_goal': {rl_goal!r}"
        if rl_goal.get("mode") == "bounded_horizon" and not rl_goal.get("win_assertions"):
            return False, None, [], "bounded_horizon 'rl_goal' requires non-empty 'win_assertions'"

        canonical = copy.deepcopy(dict(parsed))
        canonical["schema"] = PUZZLE_SCHEMA
        canonical["seat"] = int(seat_val)
        canonical["seed"] = int(parsed.get("seed", 0)) if _is_strict_int(parsed.get("seed", 0)) else 0
        canonical["lines"] = norm_lines
        canonical["rl_goal"] = dict(rl_goal)
        if not canonical.get("refutation_triggers") and meta.get("refutation_triggers"):
            canonical["refutation_triggers"] = copy.deepcopy(list(meta["refutation_triggers"]))
        return True, canonical, extra_cards, ""

    # Case 3: Mode 2 (`compose_counter_puzzle`) compact response (`challenger_zones` + `lines`)
    if "challenger_zones" in parsed or effective_task == TASK_COMPOSE_COUNTER_PUZZLE:
        cz = parsed.get("challenger_zones")
        if not isinstance(cz, Mapping):
            return False, None, [], "Counter-puzzle completion must include 'challenger_zones' object (or full studio-puzzle-v1)"
        ok_lines, norm_lines, line_err = _validate_lines_schema(parsed.get("lines"))
        if not ok_lines:
            return False, None, [], line_err

        solver_seat = int(meta.get("seat", 1)) if int(meta.get("seat", 1)) in (1, 2) else 1
        defender_seat = 2 if solver_seat == 1 else 1
        s_key = str(solver_seat)
        d_key = str(defender_seat)

        allowed_decks = meta.get("allowed_decks") or meta.get("decks") or {}
        challenger_deck = list(allowed_decks.get(s_key) or meta.get("challenger_deck") or [])
        defender_deck = list(allowed_decks.get(d_key) or meta.get("defender_deck") or [])
        defender_zones = meta.get("defender_zones")
        if not challenger_deck or not defender_deck or not isinstance(defender_zones, Mapping):
            return (
                False,
                None,
                [],
                "Counter-puzzle compact completion requires 'allowed_decks' and 'defender_zones' in ground_truth_meta",
            )

        state_meta = dict(meta.get("state") or {})
        state_norm = {
            "memory": int(parsed.get("memory", state_meta.get("memory", 3))),
            "phase": str(state_meta.get("phase") or "Main"),
            "turn": int(state_meta.get("turn", 4)),
            "first_player": int(state_meta.get("first_player", solver_seat)),
        }
        zones_merged = {
            s_key: _fill_remaining_library_zones(challenger_deck, cz),
            d_key: _fill_remaining_library_zones(defender_deck, defender_zones),
        }
        rl_goal = parsed.get("rl_goal") or meta.get("rl_goal") or {"mode": "playout_to_win", "win_assertions": []}
        if not isinstance(rl_goal, Mapping) or rl_goal.get("mode", "playout_to_win") not in VALID_RL_GOAL_MODES:
            return False, None, [], f"Invalid 'rl_goal' in counter-puzzle: {rl_goal!r}"
        if rl_goal.get("mode") == "bounded_horizon" and not rl_goal.get("win_assertions"):
            return False, None, [], "bounded_horizon 'rl_goal' requires non-empty 'win_assertions'"

        pid = slugify_puzzle_id(str(parsed.get("id") or parsed.get("title") or meta.get("puzzle_id") or "counter-puzzle"))
        canonical = {
            "schema": PUZZLE_SCHEMA,
            "id": pid,
            "title": str(parsed.get("title") or pid),
            "description": str(parsed.get("description") or ""),
            "folder": "",
            "tags": list(parsed.get("tags") or []),
            "source": {"kind": "composed"},
            "decks": {s_key: challenger_deck, d_key: defender_deck},
            "seed": int(meta.get("seed", 0)),
            "seat": solver_seat,
            "setup": {
                "kind": "staged",
                "state": state_norm,
                "zones": zones_merged,
                "prefix_actions": list(meta.get("prefix_actions") or []),
            },
            "lines": norm_lines,
            "expectations": list(parsed.get("expectations") or []),
            "assertions": dict(parsed.get("assertions") or {"engine": [], "ui": []}),
            "rl_goal": dict(rl_goal),
            "refutation_triggers": copy.deepcopy(list(meta.get("refutation_triggers") or [])),
        }
        return True, canonical, extra_cards, ""

    # Case 4: Mode 1 (`solve_position`) response (`lines` or `target_actions` / `avoid_actions`)
    raw_lines = parsed.get("lines")
    if raw_lines is None and isinstance(parsed.get("target_actions"), list):
        synth_lines: list[dict[str, Any]] = [
            {"id": "target", "name": "target", "role": "target", "actions": list(parsed["target_actions"])}
        ]
        if isinstance(parsed.get("avoid_actions"), list) and parsed["avoid_actions"]:
            synth_lines.append(
                {"id": "avoid", "name": "avoid", "role": "avoid", "actions": list(parsed["avoid_actions"])}
            )
        raw_lines = synth_lines

    if schema is not None and schema not in (TACTICIAN_SOLVE_SCHEMA, PUZZLE_SCHEMA):
        return False, None, [], f"Unsupported solve schema {schema!r}"

    ok_lines, norm_lines, line_err = _validate_lines_schema(raw_lines)
    if not ok_lines:
        return False, None, [], line_err

    base_puzzle = meta.get("base_puzzle")
    if not isinstance(base_puzzle, Mapping):
        return (
            False,
            None,
            [],
            "Solve-position completion has no 'setup'/'decks' and ground_truth_meta has no 'base_puzzle'",
        )

    if base_puzzle.get("schema") == SEARCH_EXAM_SCHEMA:
        canonical = _convert_search_exam_to_puzzle_dict(base_puzzle)
    else:
        canonical = copy.deepcopy(dict(base_puzzle))

    canonical["lines"] = norm_lines
    if parsed.get("rl_goal") and isinstance(parsed["rl_goal"], Mapping):
        canonical["rl_goal"] = dict(parsed["rl_goal"])
    if meta.get("refutation_triggers") and not canonical.get("refutation_triggers"):
        canonical["refutation_triggers"] = copy.deepcopy(list(meta["refutation_triggers"]))
    return True, canonical, extra_cards, ""


def _collect_zone_card_counts(zone: Mapping[str, Any]) -> tuple[Counter[str], list[str], list[str]]:
    """Return `(non_token_counts, token_ids, illegal_non_field_tokens)` for one seat's zones."""
    counts: Counter[str] = Counter()
    tokens: list[str] = []
    illegal_tokens: list[str] = []

    def _add(cid_raw: Any, *, allow_token: bool) -> None:
        cid = str(cid_raw or "").strip()
        if not cid:
            return
        if cid.startswith("TOKEN_"):
            tokens.append(cid)
            if not allow_token:
                illegal_tokens.append(cid)
            return
        counts[cid] += 1

    br = zone.get("breeding")
    if isinstance(br, list):
        for c in br:
            _add(c, allow_token=False)
    elif isinstance(br, Mapping):
        for c in br.get("stack") or []:
            _add(c, allow_token=False)

    for key in ("hand", "security", "trash", "deck_top", "egg_deck"):
        for item in zone.get(key) or []:
            if isinstance(item, Mapping):
                _add(item.get("card_id"), allow_token=False)
            else:
                _add(item, allow_token=False)

    for perm in zone.get("field") or []:
        if isinstance(perm, Mapping):
            for c in perm.get("stack") or []:
                _add(c, allow_token=True)
            for lc in perm.get("linked_cards") or []:
                if isinstance(lc, Mapping):
                    _add(lc.get("card_id"), allow_token=False)
                else:
                    _add(lc, allow_token=False)
        elif isinstance(perm, list):
            for c in perm:
                _add(c, allow_token=True)

    return counts, tokens, illegal_tokens


def _check_card_admissibility_and_purity(
    puzzle: Mapping[str, Any],
    meta: Mapping[str, Any],
    extra_cards: Sequence[str],
    admissibility: CardAdmissibility,
) -> tuple[bool, str, dict[str, Any]]:
    """Stage 2 check: `data/card_admissibility.json` + deck copy multiset purity."""
    decks = puzzle["decks"]
    all_non_token_ids: set[str] = set()

    for seat_key in ("1", "2"):
        for cid_raw in decks.get(seat_key) or []:
            cid = str(cid_raw).strip()
            if cid.startswith("TOKEN_"):
                return (
                    False,
                    f"Token card {cid!r} cannot appear in decks.{seat_key}",
                    {"illegal_deck_token": cid},
                )
            all_non_token_ids.add(cid)

    for cid in extra_cards:
        if not cid.startswith("TOKEN_"):
            all_non_token_ids.add(cid)

    for rtr in puzzle.get("refutation_triggers") or []:
        if isinstance(rtr, Mapping) and rtr.get("source_card_id"):
            scid = str(rtr["source_card_id"]).strip()
            if scid and not scid.startswith("TOKEN_"):
                all_non_token_ids.add(scid)

    setup = puzzle.get("setup") or {}
    zones = setup.get("zones") or {}
    zone_counts_by_seat: dict[str, Counter[str]] = {}
    if setup.get("kind") == "staged" and isinstance(zones, Mapping):
        for seat_key in ("1", "2"):
            z_counts, _, bad_tokens = _collect_zone_card_counts(zones.get(seat_key) or {})
            if bad_tokens:
                return (
                    False,
                    f"Token card(s) {bad_tokens} placed outside field in zones.{seat_key}",
                    {"illegal_zone_tokens": bad_tokens},
                )
            zone_counts_by_seat[seat_key] = z_counts
            all_non_token_ids.update(z_counts.keys())

    inadmissible: dict[str, list[str]] = {}
    for cid in sorted(all_non_token_ids):
        if not admissibility.admissible(cid):
            inadmissible[cid] = admissibility.reasons(cid)
    if inadmissible:
        first_cid = next(iter(inadmissible))
        return (
            False,
            f"Inadmissible or unregistered card {first_cid!r} ({','.join(inadmissible[first_cid])})",
            {"inadmissible_cards": inadmissible},
        )

    # Check staged zone card counts against puzzle decks[seat] multiset
    for seat_key, z_counts in zone_counts_by_seat.items():
        deck_counts = Counter(str(c) for c in (decks.get(seat_key) or []))
        excess = {
            cid: {"placed": cnt, "in_deck": deck_counts.get(cid, 0)}
            for cid, cnt in z_counts.items()
            if cnt > deck_counts.get(cid, 0)
        }
        if excess:
            first_cid = next(iter(excess))
            return (
                False,
                f"zones.{seat_key} places {excess[first_cid]['placed']} copies of {first_cid!r}, "
                f"exceeding decks.{seat_key} count ({excess[first_cid]['in_deck']})",
                {"excess_zone_cards": {seat_key: excess}},
            )

    # Check against ground_truth_meta allowed Challenger/Defender deck multisets when provided
    allowed_decks = meta.get("allowed_decks")
    if isinstance(allowed_decks, Mapping):
        for seat_key in ("1", "2"):
            if seat_key not in allowed_decks or not allowed_decks[seat_key]:
                continue
            allowed_counts = Counter(str(c) for c in allowed_decks[seat_key] if not str(c).startswith("TOKEN_"))
            cand_deck_counts = Counter(str(c) for c in (decks.get(seat_key) or []) if not str(c).startswith("TOKEN_"))
            deck_excess = {
                cid: {"candidate": cnt, "allowed": allowed_counts.get(cid, 0)}
                for cid, cnt in cand_deck_counts.items()
                if cnt > allowed_counts.get(cid, 0)
            }
            if deck_excess:
                first_cid = next(iter(deck_excess))
                return (
                    False,
                    f"decks.{seat_key} uses {deck_excess[first_cid]['candidate']} copies of {first_cid!r}, "
                    f"exceeding allowed decklist count ({deck_excess[first_cid]['allowed']})",
                    {"allowed_deck_violations": {seat_key: deck_excess}},
                )

    return True, "", {"admissible_card_count": len(all_non_token_ids)}


def _check_board_legality_inprocess(
    puzzle: Mapping[str, Any],
    meta: Mapping[str, Any],
) -> tuple[bool, str, dict[str, Any]]:
    """Stage 3 check: `RustDebugGame` staging, zone invariants, `prefix_actions`, and `validate()`."""
    import digimon_engine as de

    setup = puzzle["setup"]
    decks = puzzle["decks"]
    seed = int(puzzle.get("seed", 0))
    prefix_actions = [int(a) for a in (setup.get("prefix_actions") or [])]

    if setup["kind"] == "staged":
        state = setup["state"]
        mem = int(state["memory"])
        turn = int(state["turn"])
        if mem < -10 or mem > 10:
            return False, f"Staged memory {mem} out of legal [-10, 10] gauge bounds", {"memory": mem}
        if turn < 1:
            return False, f"Staged turn {turn} must be >= 1", {"turn": turn}

        zones = setup["zones"]
        for seat_key in ("1", "2"):
            z = zones[seat_key]
            field_list = z.get("field") or []
            if len(field_list) > 14:
                return (
                    False,
                    f"zones.{seat_key}.field has {len(field_list)} permanents (max 14)",
                    {"field_len": len(field_list)},
                )
            for zone_name in ("hand", "security", "deck_top"):
                for cid_raw in z.get(zone_name) or []:
                    cid = cid_raw.get("card_id") if isinstance(cid_raw, Mapping) else cid_raw
                    if cid and _is_digi_egg(str(cid)):
                        return (
                            False,
                            f"Digi-Egg {cid!r} cannot be staged in zones.{seat_key}.{zone_name}",
                            {"illegal_egg_zone": zone_name, "card_id": str(cid)},
                        )
            for cid in z.get("egg_deck") or []:
                if cid and not _is_digi_egg(str(cid)):
                    return (
                        False,
                        f"Non-Digi-Egg {cid!r} cannot be staged in zones.{seat_key}.egg_deck",
                        {"illegal_egg_deck_card": str(cid)},
                    )
            for f_i, perm in enumerate(field_list):
                stack = perm.get("stack") or []
                if not stack:
                    return (
                        False,
                        f"zones.{seat_key}.field[{f_i}] has an empty card stack",
                        {"empty_field_slot": f_i},
                    )
                top_cid = str(stack[-1])
                if _is_digi_egg(top_cid):
                    return (
                        False,
                        f"Digi-Egg {top_cid!r} cannot be top card on battle area (zones.{seat_key}.field[{f_i}])",
                        {"egg_top_on_field": top_cid},
                    )

        # Enforce locked Defender Boss setup when `enforce_defender_setup` is set in Mode 2
        if meta.get("enforce_defender_setup") and isinstance(meta.get("defender_zones"), Mapping):
            solver_seat = int(puzzle["seat"])
            def_key = "2" if solver_seat == 1 else "1"
            expected_def = meta["defender_zones"]
            cand_def = zones[def_key]
            exp_stacks = [list(p.get("stack") or []) for p in (expected_def.get("field") or []) if isinstance(p, Mapping)]
            got_stacks = [list(p.get("stack") or []) for p in (cand_def.get("field") or []) if isinstance(p, Mapping)]
            exp_sec = len(expected_def.get("security") or [])
            got_sec = len(cand_def.get("security") or [])
            if got_stacks != exp_stacks or got_sec != exp_sec:
                return (
                    False,
                    f"Defender Boss setup in zones.{def_key} was modified from required ground_truth_meta",
                    {"expected_defender_stacks": exp_stacks, "got_defender_stacks": got_stacks},
                )

    try:
        dbg = de.RustDebugGame(list(decks["1"]), list(decks["2"]), seed)
        if setup["kind"] == "staged":
            dbg.skip_mulligan()
            dbg.apply_scenario(json.dumps({"state": setup["state"], "zones": setup["zones"]}))
            dbg.validate()
        else:
            if not prefix_actions:
                dbg.skip_mulligan()

        for step_idx, aid in enumerate(prefix_actions):
            if dbg.is_game_over:
                return (
                    False,
                    f"Game ended prematurely at prefix_actions[{step_idx}]",
                    {"prefix_step": step_idx},
                )
            mask = np.asarray(dbg.get_action_mask())
            if aid < 0 or aid >= len(mask) or mask[aid] <= 0.5:
                return (
                    False,
                    f"Illegal prefix_actions[{step_idx}] = {aid}",
                    {"prefix_step": step_idx, "action_id": aid},
                )
            dbg.step(int(aid))

        dbg.validate()
        if dbg.is_game_over:
            return False, "Staged game is already game_over before puzzle lines start", {}
    except Exception as exc:
        return False, f"RustDebugGame validation failed: {exc}", {"exception": str(exc)}

    return True, "", {"current_player_id": int(dbg.current_player_id)}


def _evaluate_assertion(game: Any, solver_seat: int, assertion: Mapping[str, Any]) -> bool:
    """Evaluate an engine goal assertion on `RustHeadlessGame` (matches `digimon-engine-mcp`)."""
    kind = str(assertion.get("kind") or "")
    ui = game.to_ui_json()
    rl = game.get_rl_state() if hasattr(game, "get_rl_state") else {}

    if kind == "winner":
        wanted = int(assertion.get("player", solver_seat))
        return bool(game.is_game_over) and int(game.winner_id or 0) == wanted
    if kind == "memory_equals":
        return int(rl.get("memory", ui.get("memoryGauge", 0))) == int(assertion["value"])
    if kind == "memory_at_least":
        return int(rl.get("memory", ui.get("memoryGauge", 0))) >= int(assertion["value"])
    if kind == "zone_count":
        pid = int(assertion.get("player", solver_seat))
        p = ui.get(f"player{pid}", {})
        zone = str(assertion.get("zone") or "")
        mapping = {
            "hand": len(p.get("handIds", [])),
            "security": int(p.get("securityCount", 0)),
            "trash": len(p.get("trashIds", [])),
            "field": len(p.get("battleArea", [])),
            "battle_area": len(p.get("battleArea", [])),
            "deck": int(p.get("deckCount", 0)),
        }
        return mapping.get(zone) == int(assertion["value"])
    if kind == "stack_top":
        pid = int(assertion.get("player", solver_seat))
        ba = ui.get(f"player{pid}", {}).get("battleArea", [])
        idx = int(assertion.get("field_index", 0))
        if idx < 0 or idx >= len(ba):
            return False
        return ba[idx].get("topCardId") == assertion.get("card_id")
    if kind == "field_contains":
        pid = int(assertion.get("player", solver_seat))
        ba = ui.get(f"player{pid}", {}).get("battleArea", [])
        wanted_cid = assertion.get("card_id")
        return any(perm.get("topCardId") == wanted_cid for perm in ba)
    if kind == "field_not_contains":
        pid = int(assertion.get("player", solver_seat))
        ba = ui.get(f"player{pid}", {}).get("battleArea", [])
        wanted_cid = assertion.get("card_id")
        return all(perm.get("topCardId") != wanted_cid for perm in ba)
    if kind == "effective_dp":
        pid = int(assertion.get("player", solver_seat))
        ba = ui.get(f"player{pid}", {}).get("battleArea", [])
        idx = int(assertion.get("field_index", 0))
        if idx < 0 or idx >= len(ba):
            return False
        return int(ba[idx].get("dp", -1)) == int(assertion["value"])
    return False


def _line_satisfies_goal(
    game: Any,
    solver_seat: int,
    puzzle: Mapping[str, Any],
    *,
    actions_taken: int,
    start_turn: int,
) -> bool:
    """Check whether `game` satisfies `puzzle['rl_goal']` for `solver_seat`."""
    rl_goal = puzzle.get("rl_goal") or {"mode": "playout_to_win"}
    mode = str(rl_goal.get("mode") or "playout_to_win")
    win_assertions = list(rl_goal.get("win_assertions") or [])
    if not win_assertions:
        eng_assertions = (puzzle.get("assertions") or {}).get("engine") or []
        if isinstance(eng_assertions, list):
            win_assertions = list(eng_assertions)

    if mode == "bounded_horizon":
        if game.is_game_over and int(game.winner_id or 0) != solver_seat:
            return False
        max_actions = rl_goal.get("max_actions")
        if max_actions is not None and actions_taken > int(max_actions):
            return False
        max_turns = rl_goal.get("max_turns")
        if max_turns is not None:
            cur_turn = int((game.get_rl_state() or {}).get("turn_count", start_turn))
            if (cur_turn - start_turn) > int(max_turns):
                return False
        if not win_assertions:
            return bool(game.is_game_over) and int(game.winner_id or 0) == solver_seat
        return all(_evaluate_assertion(game, solver_seat, a) for a in win_assertions)

    # Default: `playout_to_win`
    if bool(game.is_game_over):
        if int(game.winner_id or 0) != solver_seat:
            return False
        non_winner_assertions = [
            a for a in win_assertions if isinstance(a, Mapping) and a.get("kind") != "winner"
        ]
        return all(_evaluate_assertion(game, solver_seat, a) for a in non_winner_assertions)

    # Allow explicit non-winner engine assertions if no terminal win was required
    non_winner = [a for a in win_assertions if isinstance(a, Mapping) and a.get("kind") != "winner"]
    if non_winner and len(non_winner) == len(win_assertions):
        return all(_evaluate_assertion(game, solver_seat, a) for a in non_winner)
    return False


def _build_staged_headless_game(puzzle: Mapping[str, Any], seed_offset: int = 0) -> Any:
    import digimon_engine as de

    setup = puzzle["setup"]
    decks = puzzle["decks"]
    base_seed = int(puzzle.get("seed", 0))
    # For replay_prefix setups, opening draws depend on exact game seed; for staged setups with
    # explicit deck_top, seed_offset tests adversary RNG robustness without changing staged zones.
    eff_seed = base_seed if setup["kind"] == "replay_prefix" else (base_seed + seed_offset)

    game = de.RustHeadlessGame(
        list(decks["1"]),
        list(decks["2"]),
        seed=eff_seed,
        observation_profile="standard_lite_v2",
    )
    prefix_actions = [int(a) for a in (setup.get("prefix_actions") or [])]
    if setup["kind"] == "staged":
        game.skip_mulligan()
        game.apply_scenario(json.dumps({"state": setup["state"], "zones": setup["zones"]}))
        for aid in prefix_actions:
            if game.is_game_over:
                break
            game.step(aid)
    else:
        if not prefix_actions:
            game.skip_mulligan()
        else:
            for aid in prefix_actions:
                if game.is_game_over:
                    break
                game.step(aid)
    return game


def _simulate_line_inprocess(
    puzzle: Mapping[str, Any],
    actions: Sequence[int],
    *,
    adversary_seeds: int = 2,
) -> dict[str, Any]:
    """Step `actions` on `RustHeadlessGame` with `refutation_triggers` + `greedy_action` interrupts."""
    solver_seat = int(puzzle["seat"])
    triggers = list(puzzle.get("refutation_triggers") or [])
    seeds = max(1, int(adversary_seeds))

    root_legal = False
    all_steps_legal = True
    solved_seeds = 0
    error_msg: Optional[str] = None
    final_winner: Optional[int] = None
    steps_trace: list[dict[str, Any]] = []

    for s_idx in range(seeds):
        game = _build_staged_headless_game(puzzle, seed_offset=s_idx * 7919)
        start_turn = int((game.get_rl_state() or {}).get("turn_count", 1))
        if not _advance_interrupts(game, solver_seat, triggers):
            error_msg = "Exceeded max opponent interrupt steps before root action"
            all_steps_legal = False
            break

        if not actions:
            error_msg = "Line has empty actions"
            all_steps_legal = False
            break

        root_mask = np.asarray(game.get_action_mask())
        first_act = int(actions[0])
        if game.is_game_over or first_act < 0 or first_act >= len(root_mask) or root_mask[first_act] <= 0.5:
            root_legal = False
            all_steps_legal = False
            error_msg = f"Root action {first_act} is illegal at puzzle start"
            break
        if s_idx == 0:
            root_legal = True

        actions_taken = 0
        seed_legal = True
        for step_i, act_raw in enumerate(actions):
            act = int(act_raw)
            if step_i > 0:
                if not _advance_interrupts(game, solver_seat, triggers):
                    error_msg = f"Exceeded max opponent interrupt steps before step {step_i}"
                    seed_legal = False
                    break
                # For `avoid` lines, an opponent interrupt may delete/bounce everything or win mid-line!
                if game.is_game_over:
                    seed_legal = False
                    error_msg = f"Game ended before solver step {step_i} (winner={game.winner_id})"
                    break
            mask = np.asarray(game.get_action_mask())
            if int(game.current_player_id) != solver_seat or act < 0 or act >= len(mask) or mask[act] <= 0.5:
                seed_legal = False
                error_msg = f"Action {act} illegal at step {step_i} (current_player={game.current_player_id})"
                break
            if s_idx == 0:
                decoded = {
                    int(d["action_id"]): d.get("label", f"Action {d['action_id']}")
                    for d in (game.legal_decoded_actions() or [])
                    if isinstance(d, Mapping) and d.get("action_id") is not None
                }
                steps_trace.append(
                    {
                        "step": step_i,
                        "action_id": act,
                        "label": decoded.get(act, f"Action {act}"),
                    }
                )
            game.step(act)
            actions_taken += 1

        _advance_interrupts(game, solver_seat, triggers)
        if s_idx == 0:
            final_winner = int(game.winner_id) if game.winner_id is not None else None

        if not seed_legal:
            all_steps_legal = False
        elif _line_satisfies_goal(
            game,
            solver_seat,
            puzzle,
            actions_taken=actions_taken,
            start_turn=start_turn,
        ):
            solved_seeds += 1

    return {
        "root_legal": root_legal,
        "all_steps_legal": all_steps_legal,
        "solved_all_seeds": all_steps_legal and (solved_seeds == seeds),
        "refuted_all_seeds": root_legal and (solved_seeds == 0),
        "solved_seeds": solved_seeds,
        "total_seeds": seeds,
        "final_winner": final_winner,
        "error": error_msg,
        "steps": steps_trace,
    }


def _verify_stages_4_and_5_via_mcp(
    puzzle: Mapping[str, Any],
    mcp_client: Any,
    *,
    adversary_seeds: int = 2,
) -> tuple[bool, str, bool, str, dict[str, Any]]:
    """Optional `StdioMcpClient` (`verify_puzzle`) evaluator for Stages 4 and 5."""
    solver_seat = int(puzzle["seat"])
    target_lines = [ln for ln in puzzle["lines"] if ln["role"] == "target"]
    avoid_lines = [ln for ln in puzzle["lines"] if ln["role"] == "avoid"]

    # Evaluate target-only puzzle first so Stage 4 can succeed even if avoid is absent/broken.
    target_only_doc = copy.deepcopy(dict(puzzle))
    target_only_doc["lines"] = target_lines
    res_target = mcp_client.call_tool(
        "verify_puzzle",
        {"puzzle_json": json.dumps(target_only_doc), "seeds": max(1, int(adversary_seeds))},
    )
    t_ok, t_reason = evaluate_verify_puzzle_result(res_target, solver_seat=solver_seat)
    if not t_ok:
        return False, t_reason, False, "Skipped Stage 5 because target line failed", {"mcp_target": res_target}

    if not avoid_lines:
        return True, "", False, "No 'avoid' line declared", {"mcp_target": res_target}

    # Check non-prefix requirement + MCP avoid verification
    valid_avoids = [
        av
        for av in avoid_lines
        if not any(t["actions"][: len(av["actions"])] == av["actions"] for t in target_lines)
    ]
    if not valid_avoids:
        return (
            True,
            "",
            False,
            "Declared 'avoid' line is an exact prefix of a 'target' line",
            {"mcp_target": res_target},
        )

    full_doc = copy.deepcopy(dict(puzzle))
    full_doc["lines"] = target_lines + valid_avoids
    res_full = mcp_client.call_tool(
        "verify_puzzle",
        {"puzzle_json": json.dumps(full_doc), "seeds": max(1, int(adversary_seeds))},
    )
    avoid_results = res_full.get("avoid_results") or []
    any_refuted = any(
        isinstance(ar, Mapping) and (ar.get("refuted") is True or ar.get("passed") is True)
        for ar in avoid_results
    )
    any_solved = any(
        isinstance(ar, Mapping)
        and (ar.get("solved") is True or (ar.get("refuted") is False and not ar.get("error")))
        for ar in avoid_results
    )
    if any_refuted and not any_solved:
        return True, "", True, "verified", {"mcp_verify": res_full}
    _, f_reason = evaluate_verify_puzzle_result(res_full, solver_seat=solver_seat)
    return True, "", False, f"Avoid line not refuted via MCP: {f_reason}", {"mcp_verify": res_full}


_ADMISSIBILITY_CACHE: dict[str, CardAdmissibility] = {}


def _get_cached_admissibility(path: Optional[Path] = None) -> CardAdmissibility:
    resolved = str((path or (resolve_repo_root() / "data" / "card_admissibility.json")).resolve())
    if resolved not in _ADMISSIBILITY_CACHE:
        _ADMISSIBILITY_CACHE[resolved] = CardAdmissibility.from_artifact(Path(resolved))
    return _ADMISSIBILITY_CACHE[resolved]


def score_single_completion(
    completion: Any,
    *,
    ground_truth_meta: Any = None,
    task_kind: Optional[str] = None,
    admissibility: Optional[CardAdmissibility] = None,
    admissibility_path: Optional[Path] = None,
    mcp_client: Optional[Any] = None,
    adversary_seeds: int = 2,
) -> RewardBreakdown:
    """Evaluate one LLM completion across all 5 progressive verifiable engine stages."""
    breakdown = RewardBreakdown()
    raw_text = _extract_completion_text(completion)
    parsed = parse_completion_json(raw_text)
    if parsed is None:
        breakdown.failure_stage = 1
        breakdown.reason = "Completion did not contain a valid JSON object"
        return breakdown.finalize()

    meta = _coerce_meta_dict(ground_truth_meta)
    ok_schema, puzzle, extra_cards, schema_err = _resolve_candidate_puzzle(parsed, meta, task_kind)
    if not ok_schema or puzzle is None:
        breakdown.failure_stage = 1
        breakdown.reason = f"Stage 1 schema error: {schema_err}"
        return breakdown.finalize()

    # Stage 1 passed (+0.15)
    breakdown.stage1_json_schema = STAGE1_WEIGHT
    breakdown.stage_reached = 1

    # Stage 2: Card Admissibility & Deck Copy Purity (+0.15 -> 0.30)
    adm = admissibility or _get_cached_admissibility(admissibility_path)
    ok_adm, adm_err, adm_diag = _check_card_admissibility_and_purity(puzzle, meta, extra_cards, adm)
    breakdown.diagnostics.update(adm_diag)
    if not ok_adm:
        breakdown.failure_stage = 2
        breakdown.reason = f"Stage 2 card admissibility/purity error: {adm_err}"
        return breakdown.finalize()

    breakdown.stage2_card_admissibility = STAGE2_WEIGHT
    breakdown.stage_reached = 2

    # Stage 3: Engine Board Legality (+0.20 -> 0.50)
    ok_board, board_err, board_diag = _check_board_legality_inprocess(puzzle, meta)
    breakdown.diagnostics.update(board_diag)
    if not ok_board:
        breakdown.failure_stage = 3
        breakdown.reason = f"Stage 3 board legality error: {board_err}"
        return breakdown.finalize()

    breakdown.stage3_board_legality = STAGE3_WEIGHT
    breakdown.stage_reached = 3

    # Stages 4 & 5 via optional MCP backend if provided
    if mcp_client is not None:
        t_ok, t_err, a_ok, a_err, mcp_diag = _verify_stages_4_and_5_via_mcp(
            puzzle,
            mcp_client,
            adversary_seeds=adversary_seeds,
        )
        breakdown.diagnostics.update(mcp_diag)
        if not t_ok:
            breakdown.failure_stage = 4
            breakdown.reason = f"Stage 4 target verification failed: {t_err}"
            return breakdown.finalize()
        breakdown.stage4_target_verified = STAGE4_WEIGHT
        breakdown.stage_reached = 4
        if not a_ok:
            breakdown.failure_stage = 5
            breakdown.reason = f"Stage 5 avoid verification failed: {a_err}"
            return breakdown.finalize()
        breakdown.stage5_avoid_refuted = STAGE5_WEIGHT
        breakdown.stage_reached = 5
        breakdown.reason = "verified"
        return breakdown.finalize()

    # In-process Stages 4 & 5 directly on `RustHeadlessGame`
    target_lines = [ln for ln in puzzle["lines"] if ln["role"] == "target"]
    avoid_lines = [ln for ln in puzzle["lines"] if ln["role"] == "avoid"]

    target_sims: list[dict[str, Any]] = []
    for t_ln in target_lines:
        sim = _simulate_line_inprocess(puzzle, t_ln["actions"], adversary_seeds=adversary_seeds)
        sim["id"] = t_ln["id"]
        target_sims.append(sim)
        if not sim["solved_all_seeds"]:
            breakdown.diagnostics["target_results"] = target_sims
            breakdown.failure_stage = 4
            detail = sim.get("error") or f"final_winner={sim.get('final_winner')} did not satisfy puzzle goal"
            breakdown.reason = f"Stage 4 target line {t_ln['id']!r} failed: {detail}"
            return breakdown.finalize()

    breakdown.diagnostics["target_results"] = target_sims
    breakdown.stage4_target_verified = STAGE4_WEIGHT
    breakdown.stage_reached = 4

    # Stage 5: At least one non-prefix `avoid` line is legal at root AND refuted (+0.15 -> 1.00)
    if not avoid_lines:
        breakdown.failure_stage = 5
        breakdown.reason = "Stage 5 missing 'avoid' line: target solved, but no avoid trap line was declared"
        return breakdown.finalize()

    avoid_sims: list[dict[str, Any]] = []
    any_valid_refuted = False
    for a_ln in avoid_lines:
        is_target_prefix = any(
            t_ln["actions"][: len(a_ln["actions"])] == a_ln["actions"] for t_ln in target_lines
        )
        sim = _simulate_line_inprocess(puzzle, a_ln["actions"], adversary_seeds=adversary_seeds)
        sim["id"] = a_ln["id"]
        sim["is_target_prefix"] = is_target_prefix
        avoid_sims.append(sim)
        if sim["solved_seeds"] > 0:
            breakdown.diagnostics["avoid_results"] = avoid_sims
            breakdown.failure_stage = 5
            breakdown.reason = f"Stage 5 avoid line {a_ln['id']!r} unexpectedly satisfied the puzzle goal"
            return breakdown.finalize()
        if not is_target_prefix and sim["refuted_all_seeds"]:
            any_valid_refuted = True

    breakdown.diagnostics["avoid_results"] = avoid_sims
    if not any_valid_refuted:
        breakdown.failure_stage = 5
        first_err = avoid_sims[0].get("error") or "avoid line is a prefix of target or illegal at root"
        breakdown.reason = f"Stage 5 no legal refuted avoid line: {first_err}"
        return breakdown.finalize()

    breakdown.stage5_avoid_refuted = STAGE5_WEIGHT
    breakdown.stage_reached = 5
    breakdown.reason = "verified"
    return breakdown.finalize()


class DigimonVerifiableReward:
    """TRL `GRPOTrainer`-compatible dense verifiable engine reward callable."""

    def __init__(
        self,
        *,
        admissibility: Optional[CardAdmissibility] = None,
        admissibility_path: Optional[Path] = None,
        mcp_client: Optional[Any] = None,
        adversary_seeds: int = 2,
        name: str = "digimon_verifiable_reward",
    ) -> None:
        # TRL's `GRPOTrainer` inspects `reward_func.__name__` when logging per-reward metrics.
        self.__name__ = str(name)
        self._admissibility = admissibility
        self._admissibility_path = admissibility_path
        self._mcp_client = mcp_client
        self.adversary_seeds = max(1, int(adversary_seeds))
        self.last_breakdowns: list[RewardBreakdown] = []

    def score_batch_detailed(
        self,
        prompts: Sequence[Any],
        completions: Sequence[Any],
        **kwargs: Any,
    ) -> list[RewardBreakdown]:
        _ = prompts  # Prompt context is carried in `ground_truth_meta` / completion JSON.
        n = len(completions)
        raw_metas = kwargs.get("ground_truth_meta")
        raw_tasks = kwargs.get("task_kind")

        adm = self._admissibility or _get_cached_admissibility(self._admissibility_path)
        out: list[RewardBreakdown] = []
        for idx in range(n):
            meta_i = None
            if isinstance(raw_metas, Sequence) and not isinstance(raw_metas, (str, bytes)):
                if idx < len(raw_metas):
                    meta_i = raw_metas[idx]
            elif isinstance(raw_metas, Mapping):
                meta_i = raw_metas

            task_i: Optional[str] = None
            if isinstance(raw_tasks, Sequence) and not isinstance(raw_tasks, (str, bytes)):
                if idx < len(raw_tasks) and raw_tasks[idx] is not None:
                    task_i = str(raw_tasks[idx])
            elif isinstance(raw_tasks, str):
                task_i = raw_tasks

            bd = score_single_completion(
                completions[idx],
                ground_truth_meta=meta_i,
                task_kind=task_i,
                admissibility=adm,
                mcp_client=self._mcp_client,
                adversary_seeds=self.adversary_seeds,
            )
            out.append(bd)
        self.last_breakdowns = out
        return out

    def __call__(
        self,
        prompts: Sequence[Any],
        completions: Sequence[Any],
        **kwargs: Any,
    ) -> list[float]:
        breakdowns = self.score_batch_detailed(prompts, completions, **kwargs)
        return [float(b.total) for b in breakdowns]
