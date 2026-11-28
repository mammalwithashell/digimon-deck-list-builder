"""Dev-Only Multi-Provider LLM Tactical Puzzle Lab (`tools.puzzle_lab`).

Provides headless workflows (`mine-replays`, `synthesize-archetype`,
`check-providers`) that pair open-weight endpoints (local RTX 4070 Ollama /
RunPod vLLM) or gated dev-only Gemini endpoints with `digimon-engine-mcp` and
`digimon-training-mcp` to author engine-verified `studio-puzzle-v1` artifacts.

Service & Licensing Boundary:
Closed-weight LLM providers are strictly dev-only inside `code/tools/puzzle_lab/`
and disabled unless `DIGIMON_ALLOW_CLOSED_LLM=1` or `allow_closed_models=True`.
Production surfaces (`code/server/`, `code/digimon_gym/`, `code/src-tauri/`)
must never import this package.
"""

from __future__ import annotations

import json
import re
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Mapping, Optional, Sequence

__version__ = "0.1.0"

PUZZLE_SCHEMA = "studio-puzzle-v1"
SEARCH_EXAM_SCHEMA = "search-exam-v1"

SLUG_RE = re.compile(r"^[a-z0-9][a-z0-9_-]*$")
FOLDER_SEG_RE = re.compile(r"^[a-z0-9][a-z0-9_-]*$")

VALID_SOURCE_KINDS = frozenset({"composed", "recording", "watch_queue", "dcgo_jsonl"})
VALID_SETUP_KINDS = frozenset({"staged", "replay_prefix"})
VALID_LINE_ROLES = frozenset({"target", "avoid", "alternative"})
VALID_EXPECTATION_KINDS = frozenset({"visits", "avoid", "choose"})
VALID_RL_GOAL_MODES = frozenset({"playout_to_win", "bounded_horizon"})

DEFAULT_FILLER_EGGS = ["ST1-01"] * 4
DEFAULT_FILLER_MAIN = ["ST1-03"] * 50
DEFAULT_FILLER_DECK = DEFAULT_FILLER_EGGS + DEFAULT_FILLER_MAIN

COLOR_NAMES = {
    0: "Red",
    1: "Blue",
    2: "Yellow",
    3: "Green",
    4: "White",
    5: "Black",
    6: "Purple",
}
KIND_NAMES = {
    0: "Digimon",
    1: "Tamer",
    2: "Option",
    3: "DigiEgg",
}


class PuzzleValidationError(ValueError):
    """Raised when a candidate puzzle violates `studio-puzzle-v1` or path safety."""


def resolve_repo_root(start: Optional[Path] = None) -> Path:
    """Walk upward from `start` (or this file) to locate the repository root."""
    cur = (start or Path(__file__)).resolve()
    if cur.is_file():
        cur = cur.parent
    for candidate in (cur, *cur.parents):
        if (candidate / "data" / "cards.json").exists() or (candidate / "qa" / "puzzles").exists():
            return candidate
    return Path.cwd().resolve()


def slugify_puzzle_id(raw: str, fallback: str = "puzzle") -> str:
    """Normalize arbitrary text into a valid kebab-case `studio-puzzle-v1` ID."""
    cleaned = re.sub(r"[^a-z0-9_-]+", "-", str(raw or "").strip().lower())
    cleaned = re.sub(r"-{2,}", "-", cleaned).strip("-_")
    if not cleaned or not SLUG_RE.match(cleaned):
        return fallback
    return cleaned[:80]


def validate_slug(slug: str, label: str = "id") -> str:
    s = str(slug or "").strip()
    if not s or ".." in s or "/" in s or "\\" in s or not SLUG_RE.match(s):
        raise PuzzleValidationError(
            f"Invalid {label} {slug!r}: must match kebab-case alphanumeric ([a-z0-9][a-z0-9_-]*)"
        )
    return s


def validate_folder(folder: Optional[str]) -> str:
    if not folder:
        return ""
    cleaned = str(folder).strip().strip("/").replace("\\", "/")
    if not cleaned:
        return ""
    parts = [p for p in cleaned.split("/") if p]
    for seg in parts:
        if seg in (".", "..") or not FOLDER_SEG_RE.match(seg):
            raise PuzzleValidationError(f"Invalid folder segment {seg!r} in {folder!r}")
    return "/".join(parts)


def safe_under(root: Path, rel: str | Path) -> Path:
    """Resolve `rel` strictly inside `root`, rejecting path traversal."""
    root_res = root.resolve()
    target = (root_res / rel).resolve()
    try:
        target.relative_to(root_res)
    except ValueError as exc:
        raise PuzzleValidationError(f"Path {rel!r} escapes allowlisted root {root_res}") from exc
    return target


def _extract_zone_cards(zone_dict: Mapping[str, Any]) -> tuple[list[str], list[str]]:
    """Return `(egg_cards_used, main_cards_used)` referenced in one player's staged zones."""
    eggs: list[str] = [str(c) for c in (zone_dict.get("egg_deck") or []) if c]
    main: list[str] = []

    breeding = zone_dict.get("breeding")
    if isinstance(breeding, list) and breeding:
        # Bottom card of a breeding stack is typically a Digi-Egg; include all in pool check.
        eggs.append(str(breeding[0]))
        for c in breeding[1:]:
            if c:
                main.append(str(c))
    elif isinstance(breeding, Mapping):
        stack = breeding.get("stack") or []
        if stack:
            eggs.append(str(stack[0]))
            main.extend(str(c) for c in stack[1:] if c)

    for key in ("hand", "security", "trash", "deck_top"):
        for c in zone_dict.get(key) or []:
            if isinstance(c, str) and c:
                main.append(c)
            elif isinstance(c, Mapping) and c.get("card_id"):
                main.append(str(c["card_id"]))

    for perm in zone_dict.get("field") or []:
        if isinstance(perm, Mapping):
            for c in perm.get("stack") or []:
                if c:
                    main.append(str(c))
            for lc in perm.get("linked_cards") or []:
                if isinstance(lc, str) and lc:
                    main.append(lc)
                elif isinstance(lc, Mapping) and lc.get("card_id"):
                    main.append(str(lc["card_id"]))
        elif isinstance(perm, list):
            main.extend(str(c) for c in perm if c)

    return eggs, main


def ensure_zones_and_decks_consistent(
    decks: Mapping[str, Sequence[str]],
    zones: Mapping[str, Mapping[str, Any]],
    *,
    card_catalog: Optional["CardTextCatalog"] = None,
) -> tuple[dict[str, list[str]], dict[str, dict[str, Any]]]:
    """Ensure `decks['1'/'2']` contain every staged card and pad legal 50-card main decks."""
    norm_decks: dict[str, list[str]] = {}
    norm_zones: dict[str, dict[str, Any]] = {}

    for seat_key in ("1", "2"):
        raw_z = dict(zones.get(seat_key) or {})
        field_out = []
        for perm in raw_z.get("field") or []:
            if isinstance(perm, Mapping):
                entry: dict[str, Any] = {
                    "stack": [str(c) for c in (perm.get("stack") or []) if c],
                    "is_suspended": bool(perm.get("is_suspended", False)),
                    "turn_played": int(perm.get("turn_played", 0)),
                }
                if "linked_cards" in perm and perm["linked_cards"]:
                    entry["linked_cards"] = list(perm["linked_cards"])
                if entry["stack"]:
                    field_out.append(entry)
            elif isinstance(perm, list) and perm:
                field_out.append(
                    {
                        "stack": [str(c) for c in perm if c],
                        "is_suspended": False,
                        "turn_played": 0,
                    }
                )

        breeding_raw = raw_z.get("breeding")
        if isinstance(breeding_raw, Mapping):
            breeding_val = [str(c) for c in (breeding_raw.get("stack") or []) if c] or None
        elif isinstance(breeding_raw, list):
            breeding_val = [str(c) for c in breeding_raw if c] or None
        else:
            breeding_val = None

        z_clean: dict[str, Any] = {
            "breeding": breeding_val,
            "deck_top": [str(c) for c in (raw_z.get("deck_top") or []) if c],
            "egg_deck": [str(c) for c in (raw_z.get("egg_deck") or []) if c],
            "field": field_out,
            "hand": [str(c) for c in (raw_z.get("hand") or []) if c],
            "security": [str(c) for c in (raw_z.get("security") or []) if c],
            "trash": [str(c) for c in (raw_z.get("trash") or []) if c],
        }
        if "security_face_up" in raw_z:
            z_clean["security_face_up"] = list(raw_z["security_face_up"])

        base_deck = [str(c) for c in (decks.get(seat_key) or []) if c]
        if not base_deck:
            base_deck = list(DEFAULT_FILLER_DECK)

        # Separate eggs vs main cards using CardTextCatalog when available.
        def _is_egg(cid: str) -> bool:
            if card_catalog is not None:
                info = card_catalog.get(cid)
                if info is not None:
                    return info.card_kind == "DigiEgg"
            return cid in ("ST1-01", "BT1-001", "BT12-002", "EX12-001", "EX13-005", "BT25-005")

        # Count all cards placed in zones (excluding deck_top/egg_deck if we need to populate them).
        placed_non_library: Counter[str] = Counter()
        if z_clean["breeding"]:
            for c in z_clean["breeding"]:
                placed_non_library[c] += 1
        for perm in z_clean["field"]:
            for c in perm["stack"]:
                placed_non_library[c] += 1
            for lc in perm.get("linked_cards") or []:
                cid = lc["card_id"] if isinstance(lc, Mapping) else str(lc)
                placed_non_library[cid] += 1
        for k in ("hand", "security", "trash"):
            for c in z_clean[k]:
                placed_non_library[c] += 1

        deck_counts = Counter(c for c in base_deck if not c.startswith("TOKEN_"))
        for cid, needed in placed_non_library.items():
            if cid.startswith("TOKEN_"):
                continue
            if deck_counts[cid] < needed:
                deck_counts[cid] = needed

        # Ensure at least 50 main-deck cards and up to 4 eggs if needed.
        main_cards: list[str] = []
        egg_cards: list[str] = []
        for cid, cnt in deck_counts.items():
            if _is_egg(cid):
                egg_cards.extend([cid] * cnt)
            else:
                main_cards.extend([cid] * cnt)

        while len(main_cards) < 50:
            main_cards.append("ST1-03")
        if not egg_cards:
            egg_cards = list(DEFAULT_FILLER_EGGS)

        full_deck = egg_cards + main_cards

        # Populate deck_top / egg_deck if omitted so staged state accounts for the library cleanly.
        if not z_clean["deck_top"]:
            rem = Counter(main_cards)
            for cid, cnt in placed_non_library.items():
                if not _is_egg(cid):
                    rem[cid] = max(0, rem[cid] - cnt)
            deck_top: list[str] = []
            for cid, cnt in rem.items():
                deck_top.extend([cid] * cnt)
            z_clean["deck_top"] = deck_top

        if not z_clean["egg_deck"]:
            rem_eggs = Counter(egg_cards)
            for cid, cnt in placed_non_library.items():
                if _is_egg(cid):
                    rem_eggs[cid] = max(0, rem_eggs[cid] - cnt)
            egg_top: list[str] = []
            for cid, cnt in rem_eggs.items():
                egg_top.extend([cid] * cnt)
            z_clean["egg_deck"] = egg_top[:5]

        norm_decks[seat_key] = full_deck
        norm_zones[seat_key] = z_clean

    return norm_decks, norm_zones


def normalize_studio_puzzle_doc(
    raw: Mapping[str, Any],
    *,
    default_source_kind: str = "composed",
    card_catalog: Optional["CardTextCatalog"] = None,
) -> dict[str, Any]:
    """Normalize a draft dict into canonical `studio-puzzle-v1` layout."""
    pid = slugify_puzzle_id(str(raw.get("id") or raw.get("title") or "puzzle"))
    title = str(raw.get("title") or pid).strip() or pid
    description = str(raw.get("description") or raw.get("why") or "").strip()
    folder = validate_folder(raw.get("folder"))
    tags = sorted({str(t).strip().lower() for t in (raw.get("tags") or []) if str(t).strip()})

    source_raw = dict(raw.get("source") or {})
    if source_raw.get("kind") not in VALID_SOURCE_KINDS:
        source_raw["kind"] = default_source_kind if default_source_kind in VALID_SOURCE_KINDS else "composed"

    seat = int(raw.get("seat", 1))
    if seat not in (1, 2):
        seat = 1
    seed = int(raw.get("seed", 0))

    raw_decks = raw.get("decks") or {}
    if isinstance(raw_decks, list) and len(raw_decks) >= 2:
        decks_in = {"1": list(raw_decks[0]), "2": list(raw_decks[1])}
    elif isinstance(raw_decks, Mapping):
        decks_in = {
            "1": list(raw_decks.get("1") or DEFAULT_FILLER_DECK),
            "2": list(raw_decks.get("2") or DEFAULT_FILLER_DECK),
        }
    else:
        decks_in = {"1": list(DEFAULT_FILLER_DECK), "2": list(DEFAULT_FILLER_DECK)}

    setup_in = dict(raw.get("setup") or {})
    setup_kind = str(setup_in.get("kind") or "staged")
    if setup_kind not in VALID_SETUP_KINDS:
        setup_kind = "staged"

    if setup_kind == "staged":
        state_in = dict(setup_in.get("state") or {})
        state_norm = {
            "memory": int(state_in.get("memory", 0)),
            "phase": str(state_in.get("phase") or "Main"),
            "turn": max(1, int(state_in.get("turn", 1))),
            "first_player": int(state_in.get("first_player", seat)) if int(state_in.get("first_player", seat)) in (1, 2) else seat,
        }
        decks_norm, zones_norm = ensure_zones_and_decks_consistent(
            decks_in,
            setup_in.get("zones") or {"1": {}, "2": {}},
            card_catalog=card_catalog,
        )
        setup_norm: dict[str, Any] = {
            "kind": "staged",
            "state": state_norm,
            "zones": zones_norm,
            "prefix_actions": [int(a) for a in (setup_in.get("prefix_actions") or [])],
        }
    else:
        decks_norm = {
            "1": [str(c) for c in decks_in["1"]],
            "2": [str(c) for c in decks_in["2"]],
        }
        setup_norm = {
            "kind": "replay_prefix",
            "prefix_actions": [int(a) for a in (setup_in.get("prefix_actions") or [])],
        }

    lines_norm: list[dict[str, Any]] = []
    raw_lines = raw.get("lines") or []
    if isinstance(raw_lines, Mapping):
        for idx, (lname, actions) in enumerate(raw_lines.items(), 1):
            role = "avoid" if "avoid" in str(lname).lower() else ("target" if idx == 1 else "alternative")
            lines_norm.append(
                {
                    "id": slugify_puzzle_id(str(lname), f"line-{idx}"),
                    "name": str(lname),
                    "role": role,
                    "actions": [int(a) for a in (actions or [])],
                }
            )
    else:
        for idx, ln in enumerate(raw_lines, 1):
            if not isinstance(ln, Mapping):
                continue
            role = str(ln.get("role") or "target")
            if role not in VALID_LINE_ROLES:
                role = "alternative"
            lid = slugify_puzzle_id(str(ln.get("id") or ln.get("name") or f"{role}-{idx}"), f"{role}-{idx}")
            lname = str(ln.get("name") or lid)
            lines_norm.append(
                {
                    "id": lid,
                    "name": lname,
                    "role": role,
                    "actions": [int(a) for a in (ln.get("actions") or [])],
                }
            )

    expectations_norm: list[dict[str, Any]] = []
    for exp in raw.get("expectations") or []:
        if not isinstance(exp, Mapping):
            continue
        kind_raw = str(exp.get("kind") or "")
        kind_map = {
            "chosen": "choose",
            "choose": "choose",
            "not_chosen": "avoid",
            "avoid": "avoid",
            "visit_share_at_least": "visits",
            "visits": "visits",
        }
        kind = kind_map.get(kind_raw)
        if not kind:
            continue
        item: dict[str, Any] = {
            "kind": kind,
            "plan": [int(a) for a in (exp.get("plan") or [])],
        }
        if kind == "visits":
            item["min_share"] = float(exp.get("min_share", exp.get("share", 0.25)))
        if exp.get("label"):
            item["label"] = str(exp["label"])
        expectations_norm.append(item)

    # Auto-derive expectations from target/avoid lines if none were explicitly supplied.
    if not expectations_norm and lines_norm:
        targets = [ln for ln in lines_norm if ln["role"] == "target" and ln["actions"]]
        avoids = [ln for ln in lines_norm if ln["role"] == "avoid" and ln["actions"]]
        if len(targets) == 1:
            expectations_norm.append(
                {
                    "kind": "choose",
                    "plan": [targets[0]["actions"][0]],
                    "label": f"Choose target line {targets[0]['name']}",
                }
            )
        for av in avoids:
            if not targets or av["actions"][0] != targets[0]["actions"][0]:
                expectations_norm.append(
                    {
                        "kind": "avoid",
                        "plan": [av["actions"][0]],
                        "label": f"Avoid trap/suboptimal line {av['name']}",
                    }
                )

    assertions_in = raw.get("assertions") or {}
    assertions_norm = {
        "engine": list(assertions_in.get("engine") or []) if isinstance(assertions_in, Mapping) else [],
        "ui": list(assertions_in.get("ui") or []) if isinstance(assertions_in, Mapping) else [],
    }

    rl_goal_in = dict(raw.get("rl_goal") or {})
    rl_mode = str(rl_goal_in.get("mode") or "playout_to_win")
    if rl_mode not in VALID_RL_GOAL_MODES:
        rl_mode = "playout_to_win"
    rl_goal_norm = {
        "mode": rl_mode,
        "max_turns": int(rl_goal_in["max_turns"]) if rl_goal_in.get("max_turns") is not None else None,
        "max_actions": int(rl_goal_in["max_actions"]) if rl_goal_in.get("max_actions") is not None else None,
        "win_assertions": list(rl_goal_in.get("win_assertions") or []),
    }

    doc: dict[str, Any] = {
        "schema": PUZZLE_SCHEMA,
        "id": pid,
        "title": title,
        "description": description,
        "folder": folder,
        "tags": tags,
        "source": source_raw,
        "decks": decks_norm,
        "seed": seed,
        "seat": seat,
        "setup": setup_norm,
        "lines": lines_norm,
        "expectations": expectations_norm,
        "assertions": assertions_norm,
        "rl_goal": rl_goal_norm,
    }

    if raw.get("refutation_triggers"):
        triggers_norm: list[dict[str, Any]] = []
        for tr in raw["refutation_triggers"]:
            if not isinstance(tr, Mapping):
                continue
            triggers_norm.append(
                {
                    "source_card_id": str(tr["source_card_id"]) if tr.get("source_card_id") else None,
                    "phase": str(tr["phase"]) if tr.get("phase") else None,
                    "accept_optional": bool(tr.get("accept_optional", True)),
                    "prefer_label_contains": (
                        str(tr["prefer_label_contains"])
                        if tr.get("prefer_label_contains") is not None
                        else None
                    ),
                }
            )
        if triggers_norm:
            doc["refutation_triggers"] = triggers_norm

    return doc


def validate_studio_puzzle_doc(
    doc: Mapping[str, Any],
    *,
    require_target_line: bool = True,
    validate_with_engine: bool = False,
) -> None:
    """Strictly validate a `studio-puzzle-v1` document; raises `PuzzleValidationError`."""
    if doc.get("schema") != PUZZLE_SCHEMA:
        raise PuzzleValidationError(f"Expected schema={PUZZLE_SCHEMA!r}, got {doc.get('schema')!r}")
    validate_slug(str(doc.get("id") or ""), "id")
    if not str(doc.get("title") or "").strip():
        raise PuzzleValidationError("Puzzle 'title' must be non-empty")
    validate_folder(doc.get("folder"))

    source = doc.get("source")
    if not isinstance(source, Mapping) or source.get("kind") not in VALID_SOURCE_KINDS:
        raise PuzzleValidationError(
            f"Puzzle 'source.kind' must be one of {sorted(VALID_SOURCE_KINDS)}, got {source!r}"
        )

    decks = doc.get("decks")
    if not isinstance(decks, Mapping) or not decks.get("1") or not decks.get("2"):
        raise PuzzleValidationError("Puzzle 'decks' must contain non-empty '1' and '2' lists")

    if int(doc.get("seat", 0)) not in (1, 2):
        raise PuzzleValidationError(f"Puzzle 'seat' must be 1 or 2, got {doc.get('seat')!r}")
    if not isinstance(doc.get("seed"), int):
        raise PuzzleValidationError("Puzzle 'seed' must be an integer")

    setup = doc.get("setup")
    if not isinstance(setup, Mapping) or setup.get("kind") not in VALID_SETUP_KINDS:
        raise PuzzleValidationError(f"Puzzle 'setup.kind' must be in {sorted(VALID_SETUP_KINDS)}")
    if setup["kind"] == "staged":
        state = setup.get("state")
        zones = setup.get("zones")
        if not isinstance(state, Mapping):
            raise PuzzleValidationError("Staged puzzle missing 'setup.state'")
        for req_key in ("memory", "phase", "turn", "first_player"):
            if req_key not in state:
                raise PuzzleValidationError(f"Staged puzzle 'setup.state' missing {req_key!r}")
        if not isinstance(zones, Mapping) or "1" not in zones or "2" not in zones:
            raise PuzzleValidationError("Staged puzzle 'setup.zones' must contain '1' and '2'")

    lines = doc.get("lines")
    if not isinstance(lines, list):
        raise PuzzleValidationError("Puzzle 'lines' must be a list")
    target_count = 0
    for ln in lines:
        if not isinstance(ln, Mapping):
            raise PuzzleValidationError("Each line in 'lines' must be an object")
        validate_slug(str(ln.get("id") or ""), "line.id")
        if ln.get("role") not in VALID_LINE_ROLES:
            raise PuzzleValidationError(f"Invalid line role {ln.get('role')!r}")
        actions = ln.get("actions")
        if not isinstance(actions, list) or not all(isinstance(a, int) for a in actions):
            raise PuzzleValidationError(f"Line {ln.get('id')!r} actions must be list[int]")
        if ln["role"] == "target" and actions:
            target_count += 1
    if require_target_line and target_count == 0:
        raise PuzzleValidationError("Puzzle must include at least one non-empty 'target' line")

    expectations = doc.get("expectations")
    if not isinstance(expectations, list):
        raise PuzzleValidationError("Puzzle 'expectations' must be a list")
    for exp in expectations:
        if not isinstance(exp, Mapping) or exp.get("kind") not in VALID_EXPECTATION_KINDS:
            raise PuzzleValidationError(f"Invalid expectation entry {exp!r}")
        if not isinstance(exp.get("plan"), list) or not exp["plan"]:
            raise PuzzleValidationError(f"Expectation {exp!r} must have non-empty 'plan'")

    assertions = doc.get("assertions")
    if not isinstance(assertions, Mapping) or "engine" not in assertions or "ui" not in assertions:
        raise PuzzleValidationError("Puzzle 'assertions' must contain 'engine' and 'ui' lists")

    rl_goal = doc.get("rl_goal")
    if not isinstance(rl_goal, Mapping) or rl_goal.get("mode") not in VALID_RL_GOAL_MODES:
        raise PuzzleValidationError(f"Invalid 'rl_goal': {rl_goal!r}")
    if rl_goal["mode"] == "bounded_horizon" and not rl_goal.get("win_assertions"):
        raise PuzzleValidationError("bounded_horizon rl_goal requires non-empty 'win_assertions'")

    if "refutation_triggers" in doc:
        rtrs = doc["refutation_triggers"]
        if not isinstance(rtrs, list):
            raise PuzzleValidationError("'refutation_triggers' must be a list when present")
        for rtr in rtrs:
            if not isinstance(rtr, Mapping) or not isinstance(rtr.get("accept_optional"), bool):
                raise PuzzleValidationError(f"Invalid refutation_trigger entry: {rtr!r}")

    if validate_with_engine and setup["kind"] == "staged":
        try:
            import digimon_engine as de

            dbg = de.RustDebugGame(list(decks["1"]), list(decks["2"]), int(doc["seed"]))
            dbg.skip_mulligan()
            dbg.apply_scenario(json.dumps({"state": setup["state"], "zones": setup["zones"]}))
            for aid in setup.get("prefix_actions") or []:
                dbg.step(int(aid))
            dbg.validate()
        except Exception as exc:
            raise PuzzleValidationError(f"RustDebugGame validation failed: {exc}") from exc


def evaluate_verify_puzzle_result(
    verify_res: Mapping[str, Any],
    *,
    solver_seat: int = 1,
) -> tuple[bool, str]:
    """Evaluate a `digimon-engine-mcp` `verify_puzzle` response per §2.2."""
    if not isinstance(verify_res, Mapping):
        return False, "verify_puzzle returned non-dict response"
    if verify_res.get("ok") is False:
        return False, str(verify_res.get("error") or "verify_puzzle returned ok=False")
    for top_flag in ("verified", "passed", "all_passed"):
        if top_flag in verify_res and verify_res[top_flag] is False:
            return False, str(verify_res.get("reason") or f"verify_puzzle reported {top_flag}=False")

    target_results = verify_res.get("target_results")
    if not isinstance(target_results, list) or not target_results:
        return False, "verify_puzzle returned no target_results"

    for idx, tr in enumerate(target_results):
        if not isinstance(tr, Mapping):
            return False, f"target_results[{idx}] is not an object"
        if tr.get("error"):
            return False, f"target line {tr.get('id', idx)} errored: {tr['error']}"
        if "legal" in tr and tr["legal"] is False:
            return False, f"target line {tr.get('id', idx)} contains illegal action"
        if "passed" in tr and tr["passed"] is False:
            return False, f"target line {tr.get('id', idx)} failed verification"
        if "solved" in tr and tr["solved"] is False:
            return False, f"target line {tr.get('id', idx)} did not solve goal"
        if "passed" not in tr and "solved" not in tr and "final_winner" in tr:
            if tr["final_winner"] != solver_seat and not tr.get("goal_satisfied"):
                return (
                    False,
                    f"target line {tr.get('id', idx)} final_winner={tr['final_winner']!r} != seat {solver_seat}",
                )

    avoid_results = verify_res.get("avoid_results") or []
    if not isinstance(avoid_results, list):
        return False, "verify_puzzle avoid_results is not a list"

    for idx, ar in enumerate(avoid_results):
        if not isinstance(ar, Mapping):
            return False, f"avoid_results[{idx}] is not an object"
        if ar.get("solved") is True:
            return False, f"avoid line {ar.get('id', idx)} unexpectedly solved the puzzle"
        if "refuted" in ar and ar["refuted"] is False:
            return False, f"avoid line {ar.get('id', idx)} was not refuted"
        if "passed" in ar and ar["passed"] is False:
            return False, f"avoid line {ar.get('id', idx)} failed avoid check"
        if "passed" not in ar and "refuted" not in ar and "final_winner" in ar:
            if ar["final_winner"] == solver_seat:
                return False, f"avoid line {ar.get('id', idx)} won for solver seat {solver_seat}"

    return True, "verified"


def build_search_exam_from_puzzle(doc: Mapping[str, Any]) -> dict[str, Any]:
    """Convert a validated `studio-puzzle-v1` document into `search-exam-v1`."""
    setup = doc.get("setup") or {}
    seat_0based = max(0, int(doc.get("seat", 1)) - 1)

    exam_expectations: list[dict[str, Any]] = []
    for exp in doc.get("expectations") or []:
        kind = exp.get("kind", "")
        plan = [int(a) for a in exp.get("plan", [])]
        if kind == "visits":
            exam_expectations.append(
                {
                    "kind": "visit_share_at_least",
                    "plan": plan,
                    "share": float(exp.get("min_share", exp.get("share", 0.1))),
                    "label": exp.get("label", "target line"),
                }
            )
        elif kind == "avoid":
            exam_expectations.append(
                {"kind": "not_chosen", "plan": plan, "label": exp.get("label", "avoid line")}
            )
        elif kind == "choose":
            exam_expectations.append(
                {"kind": "chosen", "plan": plan, "label": exp.get("label", "expected line")}
            )

    lines_map = {
        str(ln.get("name") or ln.get("id")): [int(a) for a in ln.get("actions", [])]
        for ln in (doc.get("lines") or [])
    }

    decks_list = [
        list(doc["decks"].get("1") or DEFAULT_FILLER_DECK),
        list(doc["decks"].get("2") or DEFAULT_FILLER_DECK),
    ]
    exam_setup: dict[str, Any] = {
        "decks": decks_list,
        "game_seed": int(doc.get("seed", 0)),
        "actions": [int(a) for a in (setup.get("prefix_actions") or [])],
    }
    if setup.get("kind") == "staged":
        exam_setup["staged"] = {
            "state": setup.get("state"),
            "zones": setup.get("zones"),
        }

    exam: dict[str, Any] = {
        "schema": SEARCH_EXAM_SCHEMA,
        "id": doc["id"],
        "title": doc.get("title", doc["id"]),
        "why": doc.get("description", ""),
        "source": doc.get("source", {"kind": "composed"}),
        "model": {"name": "puzzle-lab", "sha256": ""},
        "setup": exam_setup,
        "search": {
            "viewer": seat_0based,
            "regime": "open-list",
            "simulations": 128,
            "worlds": 4,
            "batch": 4,
            "search_seed": int(doc.get("seed", 0)),
        },
        "expectations": exam_expectations,
        "lines": lines_map,
    }
    if doc.get("refutation_triggers"):
        exam["refutation_triggers"] = list(doc["refutation_triggers"])
    return exam


def write_studio_puzzle(
    doc: Mapping[str, Any],
    puzzles_dir: Path,
    *,
    search_exams_dir: Optional[Path] = None,
    require_target_line: bool = True,
    validate_with_engine: bool = False,
) -> tuple[Path, Optional[Path]]:
    """Validate and persist a `studio-puzzle-v1` file (and optional `search-exam-v1`)."""
    validate_studio_puzzle_doc(
        doc,
        require_target_line=require_target_line,
        validate_with_engine=validate_with_engine,
    )
    puzzles_dir.mkdir(parents=True, exist_ok=True)
    pid = validate_slug(str(doc["id"]), "id")
    folder = validate_folder(doc.get("folder"))
    dest_dir = safe_under(puzzles_dir, folder) if folder else puzzles_dir.resolve()
    dest_dir.mkdir(parents=True, exist_ok=True)
    puzzle_path = safe_under(dest_dir, f"{pid}.json")
    puzzle_path.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")

    exam_path: Optional[Path] = None
    if search_exams_dir is not None:
        search_exams_dir.mkdir(parents=True, exist_ok=True)
        exam_doc = build_search_exam_from_puzzle(doc)
        exam_path = safe_under(search_exams_dir, f"{pid}.json")
        exam_path.write_text(json.dumps(exam_doc, indent=2) + "\n", encoding="utf-8")

    return puzzle_path, exam_path


@dataclass(frozen=True)
class PrintedCardInfo:
    card_id: str
    card_name: str
    card_kind: str
    level: Optional[int]
    play_cost: Optional[int]
    dp: Optional[int]
    colors: tuple[str, ...]
    traits: tuple[str, ...]
    evo_costs: tuple[dict[str, Any], ...]
    effect_text: str
    inherited_text: str
    security_text: str
    link_text: str
    xros_req: str = ""

    def to_prompt_dict(self) -> dict[str, Any]:
        out: dict[str, Any] = {
            "card_id": self.card_id,
            "card_name": self.card_name,
            "kind": self.card_kind,
            "colors": list(self.colors),
        }
        if self.level is not None:
            out["level"] = self.level
        if self.play_cost is not None:
            out["play_cost"] = self.play_cost
        if self.dp is not None:
            out["dp"] = self.dp
        if self.traits:
            out["traits"] = list(self.traits)
        if self.evo_costs:
            out["evo_costs"] = list(self.evo_costs)
        if self.effect_text:
            out["effect_text"] = self.effect_text
        if self.inherited_text:
            out["inherited_text"] = self.inherited_text
        if self.security_text:
            out["security_text"] = self.security_text
        if self.link_text:
            out["link_text"] = self.link_text
        if self.xros_req:
            out["xros_req"] = self.xros_req
        return out


class CardTextCatalog:
    """Lazy printed-card metadata lookup combining `digimon_engine.CardDatabase` and `data/cards.json`."""

    def __init__(self, cards_json_path: Optional[Path] = None) -> None:
        self._cards_json_path = cards_json_path
        self._engine_db: Any = None
        self._raw_cards: Optional[dict[str, dict[str, Any]]] = None
        self._cache: dict[str, PrintedCardInfo] = {}

    def _ensure_loaded(self) -> None:
        if self._raw_cards is None:
            path = self._cards_json_path or (resolve_repo_root() / "data" / "cards.json")
            if path.exists():
                loaded = json.loads(path.read_text(encoding="utf-8"))
                if isinstance(loaded, dict):
                    self._raw_cards = loaded
                elif isinstance(loaded, list):
                    self._raw_cards = {str(c.get("card_id")): c for c in loaded if isinstance(c, dict)}
                else:
                    self._raw_cards = {}
            else:
                self._raw_cards = {}
        if self._engine_db is None:
            try:
                import digimon_engine as de

                self._engine_db = de.CardDatabase()
            except Exception:
                self._engine_db = False

    def get(self, card_id: str) -> Optional[PrintedCardInfo]:
        cid = str(card_id or "").strip()
        if not cid:
            return None
        if cid in self._cache:
            return self._cache[cid]
        self._ensure_loaded()
        raw = (self._raw_cards or {}).get(cid) or {}
        py_card = None
        if self._engine_db:
            try:
                py_card = self._engine_db.get_card(cid)
            except Exception:
                py_card = None
        if py_card is None and not raw:
            if cid == "TOKEN_KOTENKEN":
                info = PrintedCardInfo(
                    card_id="TOKEN_KOTENKEN",
                    card_name="Kotenken Token",
                    card_kind="Digimon",
                    level=None,
                    play_cost=None,
                    dp=9000,
                    colors=("Black",),
                    traits=(),
                    evo_costs=(),
                    effect_text="<Blocker> (This Digimon can block in the blocker timing.)",
                    inherited_text="",
                    security_text="",
                    link_text="",
                )
                self._cache[cid] = info
                return info
            if cid.startswith("TOKEN_"):
                info = PrintedCardInfo(
                    card_id=cid,
                    card_name=cid.removeprefix("TOKEN_").replace("_", " ").title() + " Token",
                    card_kind="Digimon",
                    level=None,
                    play_cost=None,
                    dp=None,
                    colors=(),
                    traits=(),
                    evo_costs=(),
                    effect_text="Token Digimon.",
                    inherited_text="",
                    security_text="",
                    link_text="",
                )
                self._cache[cid] = info
                return info
            return None

        if py_card is not None:
            kind_raw = getattr(py_card, "card_kind", None)
            kind_int = int(kind_raw) if isinstance(kind_raw, int) else getattr(kind_raw, "value", None)
            kind_str = KIND_NAMES.get(kind_int, str(kind_raw).split(".")[-1]) if kind_raw is not None else "Digimon"
            colors_raw = getattr(py_card, "colors", []) or []
            colors = tuple(
                COLOR_NAMES.get(int(c), str(c).split(".")[-1]) if isinstance(c, int) else str(c).split(".")[-1]
                for c in colors_raw
            )
            evo_list: list[dict[str, Any]] = []
            for ec in getattr(py_card, "evo_costs", []) or []:
                c_val = getattr(ec, "card_color", None)
                c_name = COLOR_NAMES.get(int(c_val), str(c_val)) if isinstance(c_val, int) else str(c_val)
                evo_list.append(
                    {
                        "color": c_name,
                        "level": getattr(ec, "level", None),
                        "memory_cost": getattr(ec, "memory_cost", None),
                    }
                )
            info = PrintedCardInfo(
                card_id=cid,
                card_name=str(getattr(py_card, "card_name", "") or raw.get("card_name_eng") or cid),
                card_kind=kind_str,
                level=getattr(py_card, "level", raw.get("level")),
                play_cost=getattr(py_card, "play_cost", raw.get("play_cost")),
                dp=getattr(py_card, "dp", raw.get("dp")),
                colors=colors or tuple(str(c) for c in (raw.get("card_colors") or [])),
                traits=tuple(str(t) for t in (getattr(py_card, "traits", None) or raw.get("type_eng") or [])),
                evo_costs=tuple(evo_list) or tuple(raw.get("evo_costs") or []),
                effect_text=str(getattr(py_card, "effect_text", "") or raw.get("effect_description_eng") or ""),
                inherited_text=str(
                    getattr(py_card, "inherited_text", "") or raw.get("inherited_effect_description_eng") or ""
                ),
                security_text=str(
                    getattr(py_card, "security_text", "") or raw.get("security_effect_description_eng") or ""
                ),
                link_text=str(getattr(py_card, "link_text", "") or raw.get("link_effect_description_eng") or ""),
                xros_req=str(raw.get("xros_req") or ""),
            )
        else:
            kind_raw = raw.get("card_kind", 0)
            kind_str = KIND_NAMES.get(kind_raw, str(kind_raw)) if isinstance(kind_raw, int) else str(kind_raw)
            colors = tuple(
                COLOR_NAMES.get(c, str(c)) if isinstance(c, int) else str(c)
                for c in (raw.get("card_colors") or [])
            )
            traits_raw = raw.get("type_eng") or []
            if isinstance(traits_raw, str):
                traits_raw = [t.strip() for t in traits_raw.split("/") if t.strip()]
            info = PrintedCardInfo(
                card_id=cid,
                card_name=str(raw.get("card_name_eng") or cid),
                card_kind=kind_str,
                level=raw.get("level"),
                play_cost=raw.get("play_cost"),
                dp=raw.get("dp"),
                colors=colors,
                traits=tuple(str(t) for t in traits_raw),
                evo_costs=tuple(raw.get("evo_costs") or []),
                effect_text=str(raw.get("effect_description_eng") or ""),
                inherited_text=str(raw.get("inherited_effect_description_eng") or ""),
                security_text=str(raw.get("security_effect_description_eng") or ""),
                link_text=str(raw.get("link_effect_description_eng") or ""),
                xros_req=str(raw.get("xros_req") or ""),
            )

        self._cache[cid] = info
        return info

    def describe_cards(self, card_ids: Sequence[str]) -> list[dict[str, Any]]:
        seen: set[str] = set()
        out: list[dict[str, Any]] = []
        for cid in card_ids:
            if not cid or cid in seen:
                continue
            seen.add(cid)
            info = self.get(cid)
            if info is not None:
                out.append(info.to_prompt_dict())
        return out
