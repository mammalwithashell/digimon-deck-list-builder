"""Guided Play-In Compiler Wrapper + `Role.AUTHOR` LLM Macro-Plan Fallback (`play_in`).

Implements §1.2 / §1.3 of `2026-11-28-next-wave-belief-critic-grpo-lab-design.md`:
1. Invokes `digimon-engine-mcp`'s `compile_play_in` tool first (goal-directed
   Rust beam search from Turn-1 Mulligan `keep` to functional equivalence on the
   target `studio-puzzle-v1` board).
2. When `compile_play_in` reports `matched == False` and an `LLMRouter` is
   supplied, runs up to `max_llm_attempts` fallback turns where `Role.AUTHOR`
   proposes a legal Turn-1 stacked-deck + `prefix_actions` (or symbolic macro
   steps / MCP stepping trace) reaching functional equivalence on the target
   `studio-puzzle-v1` board.
3. Remaps `target` / `avoid` / `alternative` lines if inert filler cards shifted
   hand indices, verifies lines still hold via `verify_puzzle`, and persists the
   upgraded `setup.kind = "replay_prefix"` puzzle + optional symbolic DCGO YAML.
"""

from __future__ import annotations

import copy
import json
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Mapping, Optional, Sequence

from . import (
    DEFAULT_FILLER_EGGS,
    DEFAULT_FILLER_MAIN,
    CardTextCatalog,
    PuzzleValidationError,
    evaluate_verify_puzzle_result,
    normalize_studio_puzzle_doc,
    validate_studio_puzzle_doc,
)
from .llm_router import ChatMessage, LLMRouter, Role, ToolCall, ToolSpec
from .mcp_client import McpError, StdioMcpClient

ACTION_PASS = 62

PLAY_IN_FALLBACK_SCHEMA: dict[str, Any] = {
    "type": "object",
    "properties": {
        "rationale": {"type": "string"},
        "stacked_decks": {
            "type": "object",
            "properties": {
                "1": {"type": "array", "items": {"type": "string"}},
                "2": {"type": "array", "items": {"type": "string"}},
            },
        },
        "opening_stacks": {
            "type": "object",
            "properties": {
                "1": {"type": "object"},
                "2": {"type": "object"},
            },
        },
        "prefix_actions": {"type": "array", "items": {"type": "integer"}},
        "symbolic_steps": {"type": "array", "items": {"type": "object"}},
        "remapped_lines": {"type": "array", "items": {"type": "object"}},
    },
    "required": ["rationale"],
    "additionalProperties": False,
}

_SEED_PERMUTATION_CACHE: dict[int, dict[str, dict[str, list[int]]]] = {}
_SENTINEL_POOL_CACHE: Optional[tuple[list[str], list[str]]] = None


@dataclass
class PlayInCompileResult:
    """Result of compiling a staged `studio-puzzle-v1` into a legal Turn-1 `replay_prefix`."""

    ok: bool
    matched: bool
    strict_zones: bool
    used_llm_fallback: bool = False
    llm_attempts_used: int = 0
    prefix_actions: list[int] = field(default_factory=list)
    stacked_decks: dict[str, list[str]] = field(default_factory=dict)
    remapped_lines: list[dict[str, Any]] = field(default_factory=list)
    distance_score: float = 0.0
    dcgo_yaml: Optional[str] = None
    upgraded_puzzle: Optional[dict[str, Any]] = None
    out_puzzle_path: Optional[Path] = None
    out_dcgo_yaml_path: Optional[Path] = None
    verify_result: Optional[dict[str, Any]] = None
    mcp_compile_result: Optional[dict[str, Any]] = None
    error: str = ""

    def to_dict(self) -> dict[str, Any]:
        return {
            "ok": self.ok,
            "matched": self.matched,
            "strict_zones": self.strict_zones,
            "used_llm_fallback": self.used_llm_fallback,
            "llm_attempts_used": self.llm_attempts_used,
            "prefix_actions": list(self.prefix_actions),
            "stacked_decks": {k: list(v) for k, v in self.stacked_decks.items()},
            "remapped_lines": copy.deepcopy(self.remapped_lines),
            "distance_score": float(self.distance_score),
            "dcgo_yaml": self.dcgo_yaml,
            "out_puzzle_path": str(self.out_puzzle_path) if self.out_puzzle_path else None,
            "out_dcgo_yaml_path": str(self.out_dcgo_yaml_path) if self.out_dcgo_yaml_path else None,
            "error": self.error or None,
        }


def _get_sentinel_pools() -> tuple[list[str], list[str]]:
    """Return `(4 distinct DigiEgg IDs, 50 distinct Main-deck IDs)` for seed bijection."""
    global _SENTINEL_POOL_CACHE
    if _SENTINEL_POOL_CACHE is not None:
        return _SENTINEL_POOL_CACHE

    import digimon_engine as de

    db = de.CardDatabase()
    eggs: list[str] = []
    mains: list[str] = []
    for cid in sorted(de.load_implemented_card_ids()):
        if cid.startswith("TOKEN_"):
            continue
        card = db.get_card(cid)
        if card is None:
            continue
        kind_raw = getattr(card, "card_kind", None)
        is_egg = (kind_raw == de.CardKind.DigiEgg) or ("DigiEgg" in str(kind_raw)) or (kind_raw == 3)
        if is_egg:
            if len(eggs) < 4:
                eggs.append(cid)
        else:
            if len(mains) < 50:
                mains.append(cid)
        if len(eggs) >= 4 and len(mains) >= 50:
            break

    if len(eggs) < 4 or len(mains) < 50:
        raise RuntimeError("Insufficient implemented cards to calibrate seed permutation")
    _SENTINEL_POOL_CACHE = (eggs, mains)
    return _SENTINEL_POOL_CACHE


def _seed_slot_bijection(seed: int) -> dict[str, dict[str, list[int]]]:
    """Discover which input indices `(0..4)` for eggs and `(0..50)` for main land in
    `hand`, `security`, `deck_top`, and `egg_deck` after Mulligan `keep` for `seed`."""
    s = int(seed)
    if s in _SEED_PERMUTATION_CACHE:
        return _SEED_PERMUTATION_CACHE[s]

    import digimon_engine as de

    eggs, mains = _get_sentinel_pools()
    egg_pos = {cid: idx for idx, cid in enumerate(eggs)}
    main_pos = {cid: idx for idx, cid in enumerate(mains)}

    game = de.RustHeadlessGame(eggs + mains, eggs + mains, seed=s)
    # Keep Mulligan for both seats
    while not game.is_game_over and str((game.get_rl_state() or {}).get("phase") or "") == "Mulligan":
        game.step(0)

    scen = game.to_scenario()
    mapping: dict[str, dict[str, list[int]]] = {}
    for seat_key in ("1", "2"):
        z = (scen.get("zones") or {}).get(seat_key) or {}
        mapping[seat_key] = {
            "hand": [main_pos[c] for c in (z.get("hand") or []) if c in main_pos],
            "security": [main_pos[c] for c in (z.get("security") or []) if c in main_pos],
            "deck_top": [main_pos[c] for c in (z.get("deck_top") or []) if c in main_pos],
            "egg_deck": [egg_pos[c] for c in (z.get("egg_deck") or []) if c in egg_pos],
        }
    _SEED_PERMUTATION_CACHE[s] = mapping
    return mapping


def stack_deck_for_seed(
    spec: Mapping[str, Any],
    *,
    seat: str,
    seed: int = 0,
    base_deck: Optional[Sequence[str]] = None,
    card_catalog: Optional[CardTextCatalog] = None,
) -> list[str]:
    """Construct a legal `4 egg + 50 main` deck array whose seeded Turn-1 deal matches `spec`.

    `spec` may provide:
    - `hand`: up to 5 opening hand card IDs
    - `draws` / `deck_top`: ordered subsequent draws from top of deck
    - `security`: up to 5 security card IDs (top to bottom)
    - `eggs` / `egg_deck`: up to 4 Digi-Egg card IDs (top to bottom)
    - `stack`: flat draw-order prefix (`hand[0..5]` followed by `draws[...]`)
    """
    catalog = card_catalog or CardTextCatalog()
    bijection = _seed_slot_bijection(seed)[str(seat)]

    def _is_egg(cid: str) -> bool:
        info = catalog.get(cid)
        if info is not None:
            return info.card_kind == "DigiEgg"
        return cid in ("ST1-01", "BT1-001", "BT12-002", "EX12-001", "EX12-002", "EX13-005")

    base_eggs: list[str] = []
    base_mains: list[str] = []
    for cid in base_deck or []:
        s_cid = str(cid).strip()
        if not s_cid or s_cid.startswith("TOKEN_"):
            continue
        if _is_egg(s_cid):
            base_eggs.append(s_cid)
        else:
            base_mains.append(s_cid)

    if not base_eggs:
        base_eggs = list(DEFAULT_FILLER_EGGS)
    while len(base_eggs) < 4:
        base_eggs.append(base_eggs[-1])

    if not base_mains:
        base_mains = list(DEFAULT_FILLER_MAIN)
    while len(base_mains) < 50:
        base_mains.append("ST1-03")

    flat_stack = [str(c) for c in (spec.get("stack") or []) if c]
    wanted_hand = [str(c) for c in (spec.get("hand") or flat_stack[:5]) if c][:5]
    wanted_draws = [
        str(c)
        for c in (spec.get("draws") or spec.get("deck_top") or flat_stack[5:])
        if c
    ][:40]
    wanted_sec = [str(c) for c in (spec.get("security") or []) if c][:5]
    wanted_eggs = [str(c) for c in (spec.get("eggs") or spec.get("egg_deck") or []) if c][:4]

    # Build multiset pool of remaining main-deck cards for unpinned slots.
    pinned_mains = wanted_hand + wanted_draws + wanted_sec
    rem_counts = Counter(base_mains)
    for c in pinned_mains:
        if rem_counts.get(c, 0) > 0:
            rem_counts[c] -= 1
    filler_pool: list[str] = []
    for c, cnt in rem_counts.items():
        if cnt > 0:
            filler_pool.extend([c] * cnt)
    while len(filler_pool) + len(pinned_mains) < 50:
        filler_pool.append("ST1-03")

    main_slots: list[Optional[str]] = [None] * 50
    for i, cid in enumerate(wanted_hand):
        if i < len(bijection["hand"]):
            main_slots[bijection["hand"][i]] = cid
    for i, cid in enumerate(wanted_sec):
        if i < len(bijection["security"]):
            main_slots[bijection["security"][i]] = cid
    for i, cid in enumerate(wanted_draws):
        if i < len(bijection["deck_top"]):
            main_slots[bijection["deck_top"][i]] = cid

    filler_iter = iter(filler_pool)
    final_mains = [
        slot_val if slot_val is not None else next(filler_iter, "ST1-03")
        for slot_val in main_slots
    ]

    egg_slots: list[Optional[str]] = [None] * 4
    for i, cid in enumerate(wanted_eggs):
        if i < len(bijection["egg_deck"]):
            egg_slots[bijection["egg_deck"][i]] = cid
    rem_eggs = Counter(base_eggs[:4])
    for c in wanted_eggs:
        if rem_eggs.get(c, 0) > 0:
            rem_eggs[c] -= 1
    egg_fill = [c for c, cnt in rem_eggs.items() for _ in range(cnt)]
    egg_iter = iter(egg_fill)
    final_eggs = [
        slot_val if slot_val is not None else next(egg_iter, base_eggs[0])
        for slot_val in egg_slots
    ]

    return final_eggs + final_mains


def _load_input_puzzle(
    puzzle: Path | str | Mapping[str, Any],
    *,
    card_catalog: Optional[CardTextCatalog] = None,
) -> tuple[dict[str, Any], Optional[Path]]:
    if isinstance(puzzle, Mapping):
        doc = normalize_studio_puzzle_doc(puzzle, card_catalog=card_catalog)
        return doc, None
    p = Path(puzzle)
    raw = json.loads(p.read_text(encoding="utf-8"))
    doc = normalize_studio_puzzle_doc(raw, card_catalog=card_catalog)
    return doc, p


def _normalize_perm_sig(perm: Mapping[str, Any]) -> tuple[tuple[str, ...], bool, tuple[str, ...]]:
    stack = tuple(str(c) for c in (perm.get("stack") or []) if c)
    susp = bool(perm.get("is_suspended", False))
    linked = tuple(
        str(lc["card_id"] if isinstance(lc, Mapping) else lc)
        for lc in (perm.get("linked_cards") or [])
        if lc
    )
    return stack, susp, linked


def check_functional_equivalence(
    actual_scenario: Mapping[str, Any],
    actual_current_player: int,
    target_puzzle: Mapping[str, Any],
    *,
    strict_zones: bool = False,
) -> tuple[bool, float, list[str]]:
    """Evaluate §1.2 functional equivalence between a replayed state and staged `target_puzzle`.

    Returns `(matched, distance_score, discrepancies)`.
    """
    setup = target_puzzle.get("setup") or {}
    target_state = setup.get("state") or {}
    target_zones = setup.get("zones") or {}

    actual_state = actual_scenario.get("state") or {}
    actual_zones = actual_scenario.get("zones") or {}

    discrepancies: list[str] = []
    distance = 0.0

    target_phase = str(target_state.get("phase") or "Main")
    actual_phase = str(actual_state.get("phase") or "Main")
    if actual_phase != target_phase:
        distance += 5.0
        discrepancies.append(f"phase mismatch: actual={actual_phase!r} vs target={target_phase!r}")

    target_player = int(target_puzzle.get("seat", target_state.get("first_player", 1)))
    if int(actual_current_player) != target_player:
        distance += 5.0
        discrepancies.append(
            f"current_player mismatch: actual={actual_current_player} vs target={target_player}"
        )

    target_mem = int(target_state.get("memory", 0))
    actual_mem = int(actual_state.get("memory", 0))
    if actual_mem != target_mem:
        distance += float(abs(actual_mem - target_mem))
        discrepancies.append(f"memory mismatch: actual={actual_mem} vs target={target_mem}")

    actual_turn = int(actual_state.get("turn", 1))
    target_turn = int(target_state.get("turn", 1))

    for pk in ("1", "2"):
        tz = target_zones.get(pk) or {}
        az = actual_zones.get(pk) or {}

        # 1. Exact field stacks + is_suspended (+ turn_played < turn readiness parity)
        t_field = [_normalize_perm_sig(p) for p in (tz.get("field") or []) if isinstance(p, Mapping)]
        a_field = [_normalize_perm_sig(p) for p in (az.get("field") or []) if isinstance(p, Mapping)]
        if Counter(a_field) != Counter(t_field):
            distance += 4.0 * max(1, abs(len(a_field) - len(t_field)))
            discrepancies.append(f"seat {pk} field mismatch: actual={a_field} vs target={t_field}")
        else:
            # Verify summoning-sickness / Delay readiness (`turn_played < turn`) matches per stack
            for tp in tz.get("field") or []:
                if not isinstance(tp, Mapping):
                    continue
                t_ready = int(tp.get("turn_played", 0)) < target_turn
                sig = _normalize_perm_sig(tp)
                matching_ap = [
                    ap
                    for ap in (az.get("field") or [])
                    if isinstance(ap, Mapping) and _normalize_perm_sig(ap) == sig
                ]
                if matching_ap and not any(
                    (int(ap.get("turn_played", 0)) < actual_turn) == t_ready for ap in matching_ap
                ):
                    distance += 2.0
                    discrepancies.append(
                        f"seat {pk} stack {list(sig[0])} readiness mismatch (target ready={t_ready})"
                    )

        # 2. Exact breeding stack
        t_breed = list(tz.get("breeding") or []) or None
        a_breed = list(az.get("breeding") or []) or None
        if a_breed != t_breed:
            distance += 3.0
            discrepancies.append(f"seat {pk} breeding mismatch: actual={a_breed} vs target={t_breed}")

        # 3. Exact security_count
        t_sec_n = len(tz.get("security") or [])
        a_sec_n = len(az.get("security") or [])
        if a_sec_n != t_sec_n:
            distance += float(abs(a_sec_n - t_sec_n)) * 2.0
            discrepancies.append(f"seat {pk} security_count mismatch: actual={a_sec_n} vs target={t_sec_n}")

        # 4. Hand & trash subset (or exact multiset under strict_zones)
        t_hand = Counter(str(c) for c in (tz.get("hand") or []) if c)
        a_hand = Counter(str(c) for c in (az.get("hand") or []) if c)
        if strict_zones:
            if a_hand != t_hand:
                diff_n = sum((t_hand - a_hand).values()) + sum((a_hand - t_hand).values())
                distance += float(diff_n)
                discrepancies.append(
                    f"seat {pk} strict hand mismatch: actual={dict(a_hand)} vs target={dict(t_hand)}"
                )
        else:
            missing_hand = t_hand - a_hand
            if missing_hand:
                distance += float(sum(missing_hand.values())) * 2.0
                discrepancies.append(f"seat {pk} hand missing required cards: {dict(missing_hand)}")

        t_trash = Counter(str(c) for c in (tz.get("trash") or []) if c)
        a_trash = Counter(str(c) for c in (az.get("trash") or []) if c)
        if strict_zones:
            if a_trash != t_trash:
                diff_n = sum((t_trash - a_trash).values()) + sum((a_trash - t_trash).values())
                distance += float(diff_n)
                discrepancies.append(
                    f"seat {pk} strict trash mismatch: actual={dict(a_trash)} vs target={dict(t_trash)}"
                )
        else:
            missing_trash = t_trash - a_trash
            if missing_trash:
                distance += float(sum(missing_trash.values())) * 2.0
                discrepancies.append(f"seat {pk} trash missing required cards: {dict(missing_trash)}")

    return len(discrepancies) == 0, distance, discrepancies


def remap_lines_for_replayed_board(
    target_puzzle: Mapping[str, Any],
    actual_scenario: Mapping[str, Any],
) -> list[dict[str, Any]]:
    """Remap solver root hand/field action indices when inert filler cards shifted positions."""
    setup = target_puzzle.get("setup") or {}
    if setup.get("kind") != "staged":
        return copy.deepcopy(list(target_puzzle.get("lines") or []))

    seat_str = str(target_puzzle.get("seat", 1))
    tz = ((setup.get("zones") or {}).get(seat_str)) or {}
    az = ((actual_scenario.get("zones") or {}).get(seat_str)) or {}

    t_hand = [str(c) for c in (tz.get("hand") or [])]
    a_hand = [str(c) for c in (az.get("hand") or [])]
    used_a_hand: set[int] = set()
    hand_map: dict[int, int] = {}
    for ti, cid in enumerate(t_hand):
        for ai, acid in enumerate(a_hand):
            if ai not in used_a_hand and acid == cid:
                hand_map[ti] = ai
                used_a_hand.add(ai)
                break

    t_field = [_normalize_perm_sig(p) for p in (tz.get("field") or []) if isinstance(p, Mapping)]
    a_field = [_normalize_perm_sig(p) for p in (az.get("field") or []) if isinstance(p, Mapping)]
    used_a_field: set[int] = set()
    field_map: dict[int, int] = {}
    for ti, sig in enumerate(t_field):
        for ai, asig in enumerate(a_field):
            if ai not in used_a_field and asig == sig:
                field_map[ti] = ai
                used_a_field.add(ai)
                break

    def _remap_root_action(a: int) -> int:
        if 0 <= a < 30:
            return hand_map.get(a, a)
        if 30 <= a < 60:
            return 30 + hand_map.get(a - 30, a - 30)
        if 63 <= a <= 92:
            return 63 + hand_map.get(a - 63, a - 63)
        if 100 <= a <= 399:
            slot, tgt = divmod(a - 100, 15)
            return 100 + field_map.get(slot, slot) * 15 + tgt
        if 400 <= a <= 999:
            h_idx, f_idx = divmod(a - 400, 15)
            nh = hand_map.get(h_idx, h_idx)
            nf = 14 if f_idx == 14 else field_map.get(f_idx, f_idx)
            return 400 + nh * 15 + nf
        if 1000 <= a <= 1149:
            slot, sub = divmod(a - 1000, 10)
            return 1000 + field_map.get(slot, slot) * 10 + sub
        return a

    remapped: list[dict[str, Any]] = []
    for ln in target_puzzle.get("lines") or []:
        if not isinstance(ln, Mapping):
            continue
        new_ln = copy.deepcopy(dict(ln))
        acts = [int(x) for x in (ln.get("actions") or [])]
        if acts:
            new_ln["actions"] = [_remap_root_action(acts[0])] + acts[1:]
        remapped.append(new_ln)
    return remapped


def _lower_symbolic_steps(
    game: Any,
    symbolic_steps: Sequence[Mapping[str, Any]],
) -> list[int]:
    """Lower LLM symbolic macro steps (`keep`, `pass`, `hatch`, `move`, `play`, `digivolve`, `attack`, `main`, `select`)
    against `RustHeadlessGame.legal_decoded_actions()`."""
    lowered: list[int] = []
    for idx, step in enumerate(symbolic_steps):
        if game.is_game_over:
            break
        if "action_id" in step and step["action_id"] is not None:
            aid = int(step["action_id"])
            game.step(aid)
            lowered.append(aid)
            continue

        kind = str(step.get("kind") or step.get("do") or step.get("action") or "").strip().lower()
        legal = list(game.legal_decoded_actions())
        mask = game.get_action_mask()
        legal_ids = [i for i, v in enumerate(mask) if v > 0.5]

        chosen: Optional[int] = None
        if kind in ("keep", "mulligan_keep"):
            chosen = 0 if 0 in legal_ids else (legal_ids[0] if legal_ids else None)
        elif kind in ("pass", "decline", "end_turn"):
            chosen = ACTION_PASS if ACTION_PASS in legal_ids else None
        elif kind == "hatch":
            chosen = 60 if 60 in legal_ids else next(
                (int(d["action_id"]) for d in legal if str(d.get("kind") or "").lower() == "hatch"),
                None,
            )
        elif kind == "move":
            chosen = 61 if 61 in legal_ids else next(
                (int(d["action_id"]) for d in legal if "move" in str(d.get("kind") or "").lower()),
                None,
            )
        elif kind == "play":
            wanted_cid = str(step.get("card_id") or step.get("card") or "")
            matches = [
                int(d["action_id"])
                for d in legal
                if str(d.get("kind") or "").lower() == "play"
                and (not wanted_cid or d.get("card_id") == wanted_cid)
            ]
            chosen = matches[0] if matches else None
        elif kind == "digivolve":
            wanted_cid = str(step.get("card_id") or step.get("using") or step.get("card") or "")
            target_idx = step.get("target_index")
            matches = [
                int(d["action_id"])
                for d in legal
                if "digivolve" in str(d.get("kind") or "").lower()
                and (not wanted_cid or d.get("card_id") == wanted_cid)
                and (target_idx is None or d.get("target_index") == int(target_idx))
            ]
            chosen = matches[0] if matches else None
        elif kind == "attack":
            src_idx = step.get("source_index", step.get("attacker_slot"))
            tgt_zone = str(step.get("target_zone") or "security").lower()
            matches = [
                int(d["action_id"])
                for d in legal
                if str(d.get("kind") or "").lower() == "attack"
                and (src_idx is None or d.get("source_index") == int(src_idx))
                and (not tgt_zone or str(d.get("target_zone") or "").lower() == tgt_zone)
            ]
            chosen = matches[0] if matches else None
        elif kind == "main":
            wanted_cid = str(step.get("card_id") or step.get("card") or "")
            matches = [
                int(d["action_id"])
                for d in legal
                if int(d["action_id"]) >= 1000
                and (not wanted_cid or d.get("card_id") == wanted_cid)
            ]
            chosen = matches[0] if matches else None
        elif kind in ("select", "selection"):
            label_sub = str(step.get("label_contains") or "").lower()
            matches = [
                int(d["action_id"])
                for d in legal
                if not label_sub or label_sub in str(d.get("label") or "").lower()
            ]
            chosen = matches[0] if matches else (legal_ids[0] if legal_ids else None)

        if chosen is None or chosen not in legal_ids:
            raise ValueError(
                f"Symbolic step {idx} ({step!r}) could not be lowered against legal actions {legal}"
            )
        game.step(int(chosen))
        lowered.append(int(chosen))
    return lowered


def render_symbolic_dcgo_yaml(
    stacked_decks: Mapping[str, Sequence[str]],
    prefix_actions: Sequence[int],
    *,
    seed: int = 0,
    rest_deck_p0: str = "custom",
    rest_deck_p1: str = "custom",
    title: str = "play-in-scenario",
) -> str:
    """Render a `dcgo-harness`-compatible symbolic YAML (`decks` + `steps: [{ do: ... }]`)."""
    import digimon_engine as de

    d1 = list(stacked_decks.get("1") or [])
    d2 = list(stacked_decks.get("2") or [])
    game = de.RustHeadlessGame(d1, d2, seed=int(seed))

    # Extract dealt draw order after Mulligan keep for DCGO `stack` prefix
    probe = de.RustHeadlessGame(d1, d2, seed=int(seed))
    while not probe.is_game_over and str((probe.get_rl_state() or {}).get("phase") or "") == "Mulligan":
        probe.step(0)
    probe_zones = probe.to_scenario().get("zones") or {}
    p0_stack = list((probe_zones.get("1") or {}).get("hand") or []) + list(
        (probe_zones.get("1") or {}).get("deck_top") or []
    )[:10]
    p1_stack = list((probe_zones.get("2") or {}).get("hand") or []) + list(
        (probe_zones.get("2") or {}).get("deck_top") or []
    )[:10]

    yaml_steps: list[str] = []
    for aid_raw in prefix_actions:
        if game.is_game_over:
            break
        aid = int(aid_raw)
        rl_phase = str((game.get_rl_state() or {}).get("phase") or "")
        if rl_phase == "Mulligan":
            game.step(aid)
            continue

        decoded_map = {int(d["action_id"]): d for d in game.legal_decoded_actions()}
        dec = decoded_map.get(aid, {})
        kind = str(dec.get("kind") or "").lower()
        cid = dec.get("card_id")
        src_idx = dec.get("source_index")
        tgt_idx = dec.get("target_index")
        tgt_zone = str(dec.get("target_zone") or "")

        if aid == 60 or kind == "hatch":
            yaml_steps.append("  - do: { hatch: {} }")
        elif aid == 61 or "move" in kind:
            yaml_steps.append("  - do: { move: { from: breeding } }")
        elif aid == ACTION_PASS or kind == "pass":
            yaml_steps.append("  - do: { pass: {} }")
        elif kind == "play" and cid:
            from_ref = f"hand.{src_idx}" if src_idx is not None else "hand"
            yaml_steps.append(f"  - do: {{ play: {{ card: {json.dumps(str(cid))}, from: {from_ref} }} }}")
        elif "digivolve" in kind and cid:
            from_ref = "breeding" if tgt_idx == 14 else f"field.{tgt_idx if tgt_idx is not None else 0}"
            yaml_steps.append(
                f"  - do: {{ digivolve: {{ from: {from_ref}, using: {json.dumps(str(cid))} }} }}"
            )
        elif kind == "attack":
            att_ref = f"field.{src_idx if src_idx is not None else 0}"
            tgt_ref = "security" if tgt_zone == "security" or tgt_idx is None else f"opp.field.{tgt_idx}"
            yaml_steps.append(f"  - do: {{ attack: {{ attacker: {att_ref}, target: {tgt_ref} }} }}")
        elif 1000 <= aid <= 1159:
            on_ref = "breeding" if aid >= 1150 else f"field.{(aid - 1000) // 10}"
            yaml_steps.append(f"  - do: {{ main: {{ on: {on_ref} }} }}")
        else:
            yaml_steps.append(f"  - do: {{ select: {{ raw_action_id: {aid} }} }}")

        game.step(aid)

    lines = [
        f"# Generated by tools.puzzle_lab.play_in ({title})",
        "decks:",
        "  p0:",
        f"    stack: {json.dumps(p0_stack)}",
        f"    rest: {json.dumps(rest_deck_p0)}",
        "  p1:",
        f"    stack: {json.dumps(p1_stack)}",
        f"    rest: {json.dumps(rest_deck_p1)}",
        "steps:",
    ]
    if yaml_steps:
        lines.extend(yaml_steps)
    else:
        lines.append("  []")
    return "\n".join(lines) + "\n"


def build_upgraded_replay_prefix_puzzle(
    base_doc: Mapping[str, Any],
    *,
    stacked_decks: Mapping[str, Sequence[str]],
    prefix_actions: Sequence[int],
    remapped_lines: Optional[Sequence[Mapping[str, Any]]] = None,
    play_in_method: str = "beam_search",
    card_catalog: Optional[CardTextCatalog] = None,
) -> dict[str, Any]:
    """Construct canonical `setup.kind = 'replay_prefix'` `studio-puzzle-v1` document."""
    raw = copy.deepcopy(dict(base_doc))
    raw["decks"] = {
        "1": [str(c) for c in (stacked_decks.get("1") or base_doc["decks"]["1"])],
        "2": [str(c) for c in (stacked_decks.get("2") or base_doc["decks"]["2"])],
    }
    raw["setup"] = {
        "kind": "replay_prefix",
        "prefix_actions": [int(a) for a in prefix_actions],
    }
    if remapped_lines is not None:
        raw["lines"] = [dict(ln) for ln in remapped_lines]
        # Refresh auto-derived root expectations when lines were remapped
        targets = [ln for ln in raw["lines"] if ln.get("role") == "target" and ln.get("actions")]
        avoids = [ln for ln in raw["lines"] if ln.get("role") == "avoid" and ln.get("actions")]
        new_exps: list[dict[str, Any]] = []
        if len(targets) == 1:
            new_exps.append(
                {
                    "kind": "choose",
                    "plan": [int(targets[0]["actions"][0])],
                    "label": f"Choose target line {targets[0].get('name', targets[0].get('id', 'target'))}",
                }
            )
        for av in avoids:
            if not targets or int(av["actions"][0]) != int(targets[0]["actions"][0]):
                new_exps.append(
                    {
                        "kind": "avoid",
                        "plan": [int(av["actions"][0])],
                        "label": f"Avoid trap/suboptimal line {av.get('name', av.get('id', 'avoid'))}",
                    }
                )
        if new_exps:
            raw["expectations"] = new_exps

    source = dict(raw.get("source") or {"kind": "composed"})
    source["play_in_compiled"] = True
    source["play_in_method"] = play_in_method
    raw["source"] = source

    tags = list(raw.get("tags") or [])
    if "play-in" not in tags:
        tags.append("play-in")
    raw["tags"] = tags

    norm = normalize_studio_puzzle_doc(raw, card_catalog=card_catalog)
    validate_studio_puzzle_doc(norm)
    return norm


def _build_play_in_author_tools() -> list[ToolSpec]:
    return [
        ToolSpec(
            name="new_game_from_decks",
            description="Start a Turn-1 game on digimon-engine-mcp from stacked decks to test prefix_actions.",
            parameters={
                "type": "object",
                "properties": {
                    "deck1": {"type": "array", "items": {"type": "string"}},
                    "deck2": {"type": "array", "items": {"type": "string"}},
                    "seed": {"type": "integer"},
                },
                "required": ["deck1", "deck2"],
            },
        ),
        ToolSpec(
            name="legal_actions",
            description="List legal actions for a player on game_id.",
            parameters={
                "type": "object",
                "properties": {
                    "game_id": {"type": "string"},
                    "player": {"type": "integer"},
                },
                "required": ["game_id", "player"],
            },
        ),
        ToolSpec(
            name="step",
            description="Step a single action_id on game_id.",
            parameters={
                "type": "object",
                "properties": {
                    "game_id": {"type": "string"},
                    "action_id": {"type": "integer"},
                },
                "required": ["game_id", "action_id"],
            },
        ),
        ToolSpec(
            name="field",
            description="Inspect current field, hand, breeding, memory, turn, and phase on game_id.",
            parameters={
                "type": "object",
                "properties": {"game_id": {"type": "string"}},
                "required": ["game_id"],
            },
        ),
    ]


def compile_play_in_with_fallback(
    puzzle: Path | str | Mapping[str, Any],
    *,
    engine_mcp: StdioMcpClient,
    router: Optional[LLMRouter] = None,
    beam_width: int = 32,
    max_turns: int = 5,
    strict_zones: bool = False,
    max_llm_attempts: int = 3,
    max_tool_turns: int = 6,
    out_puzzle_path: Optional[Path] = None,
    out_dcgo_yaml_path: Optional[Path] = None,
    rest_deck_p0: str = "custom",
    rest_deck_p1: str = "custom",
    adversary_sims: int = 64,
    verify_seeds: int = 4,
    card_catalog: Optional[CardTextCatalog] = None,
) -> PlayInCompileResult:
    """Compile a staged `studio-puzzle-v1` board into a legal Turn-1 `replay_prefix` setup.

    1. Calls `digimon-engine-mcp`'s `compile_play_in` first.
    2. If `matched == False` and `router` is provided, runs up to `max_llm_attempts`
       `Role.AUTHOR` macro-plan turns, verifying functional equivalence and `verify_puzzle`.
    """
    catalog = card_catalog or CardTextCatalog()
    base_doc, src_path = _load_input_puzzle(puzzle, card_catalog=catalog)

    mcp_args: dict[str, Any] = {
        "puzzle_json": base_doc,
        "beam_width": int(beam_width),
        "max_turns": int(max_turns),
        "strict_zones": bool(strict_zones),
        "rest_deck_p0": str(rest_deck_p0),
        "rest_deck_p1": str(rest_deck_p1),
    }
    if src_path is not None and src_path.is_file():
        mcp_args["puzzle_path"] = str(src_path)
    if out_puzzle_path is not None:
        mcp_args["out_puzzle_path"] = str(out_puzzle_path)
    if out_dcgo_yaml_path is not None:
        mcp_args["out_dcgo_yaml_path"] = str(out_dcgo_yaml_path)

    mcp_res: dict[str, Any]
    try:
        mcp_res = engine_mcp.call_tool("compile_play_in", mcp_args)
    except McpError as exc:
        mcp_res = {"ok": False, "matched": False, "error": str(exc), "distance_score": 999.0}

    if mcp_res.get("ok") is not False and mcp_res.get("matched") is True:
        stacked_decks = {
            "1": [str(c) for c in ((mcp_res.get("stacked_decks") or {}).get("1") or base_doc["decks"]["1"])],
            "2": [str(c) for c in ((mcp_res.get("stacked_decks") or {}).get("2") or base_doc["decks"]["2"])],
        }
        prefix_actions = [int(a) for a in (mcp_res.get("prefix_actions") or [])]
        remapped_lines = list(mcp_res.get("remapped_lines") or base_doc.get("lines") or [])

        upgraded = build_upgraded_replay_prefix_puzzle(
            base_doc,
            stacked_decks=stacked_decks,
            prefix_actions=prefix_actions,
            remapped_lines=remapped_lines,
            play_in_method="beam_search",
            card_catalog=catalog,
        )
        dcgo_yaml = mcp_res.get("dcgo_yaml")
        if dcgo_yaml is None and out_dcgo_yaml_path is not None:
            dcgo_yaml = render_symbolic_dcgo_yaml(
                stacked_decks,
                prefix_actions,
                seed=int(base_doc.get("seed", 0)),
                rest_deck_p0=rest_deck_p0,
                rest_deck_p1=rest_deck_p1,
                title=base_doc["id"],
            )

        if out_puzzle_path is not None:
            out_puzzle_path.parent.mkdir(parents=True, exist_ok=True)
            out_puzzle_path.write_text(json.dumps(upgraded, indent=2) + "\n", encoding="utf-8")
        if out_dcgo_yaml_path is not None and dcgo_yaml is not None:
            out_dcgo_yaml_path.parent.mkdir(parents=True, exist_ok=True)
            out_dcgo_yaml_path.write_text(str(dcgo_yaml), encoding="utf-8")

        return PlayInCompileResult(
            ok=True,
            matched=True,
            strict_zones=bool(strict_zones),
            used_llm_fallback=False,
            llm_attempts_used=0,
            prefix_actions=prefix_actions,
            stacked_decks=stacked_decks,
            remapped_lines=upgraded["lines"],
            distance_score=float(mcp_res.get("distance_score", 0.0)),
            dcgo_yaml=str(dcgo_yaml) if dcgo_yaml is not None else None,
            upgraded_puzzle=upgraded,
            out_puzzle_path=out_puzzle_path,
            out_dcgo_yaml_path=out_dcgo_yaml_path,
            mcp_compile_result=mcp_res,
        )

    # Pure beam search missed (`matched == False`); exit cleanly if no LLM router is configured.
    if router is None or int(max_llm_attempts) <= 0:
        return PlayInCompileResult(
            ok=bool(mcp_res.get("ok", True)),
            matched=False,
            strict_zones=bool(strict_zones),
            used_llm_fallback=False,
            llm_attempts_used=0,
            distance_score=float(mcp_res.get("distance_score", 0.0)),
            mcp_compile_result=mcp_res,
            error=str(mcp_res.get("error") or "compile_play_in beam search did not find a match"),
        )

    # ── Step 2: Role.AUTHOR LLM Macro-Plan Fallback ──────────────────────────
    import digimon_engine as de

    seed = int(base_doc.get("seed", 0))
    solver_seat = int(base_doc.get("seat", 1))

    # Collect visible card texts to ground Role.AUTHOR's macro-turn plan
    setup = base_doc.get("setup") or {}
    visible_cids: list[str] = []
    for pk in ("1", "2"):
        z = (setup.get("zones") or {}).get(pk) or {}
        for k in ("hand", "trash", "security"):
            visible_cids.extend(str(c) for c in (z.get(k) or []) if isinstance(c, str))
        if isinstance(z.get("breeding"), list):
            visible_cids.extend(str(c) for c in z["breeding"] if isinstance(c, str))
        for perm in z.get("field") or []:
            if isinstance(perm, Mapping):
                visible_cids.extend(str(c) for c in (perm.get("stack") or []) if isinstance(c, str))
                for lc in perm.get("linked_cards") or []:
                    visible_cids.append(str(lc["card_id"] if isinstance(lc, Mapping) else lc))
    printed_cards = catalog.describe_cards(visible_cids)

    avail_mcp_tools = [
        spec
        for spec in _build_play_in_author_tools()
        if engine_mcp.has_tool(spec.name)
    ]

    prompt_payload = {
        "task": "Propose a legal Turn-1 play-in plan reaching functional equivalence on target_puzzle.",
        "beam_search_attempt_summary": {
            "matched": False,
            "distance_score": mcp_res.get("distance_score"),
            "error": mcp_res.get("error"),
            "best_partial_prefix_actions": mcp_res.get("prefix_actions"),
        },
        "strict_zones": bool(strict_zones),
        "seed": seed,
        "target_puzzle": {
            "id": base_doc["id"],
            "seat": solver_seat,
            "state": setup.get("state"),
            "zones": setup.get("zones"),
            "lines": base_doc.get("lines"),
        },
        "printed_cards": printed_cards,
        "output_instructions": [
            "Return JSON matching PLAY_IN_FALLBACK_SCHEMA.",
            "Provide either `opening_stacks` (`{\"1\": {\"hand\": [...], \"draws\": [...], \"security\": [...], \"eggs\": [...]}, \"2\": ...}`) or full 54-card `stacked_decks` (`{\"1\": [...], \"2\": [...]}`).",
            "Provide either integer `prefix_actions` (from Turn-1 Mulligan keep `[0, 0, ...]`) or `symbolic_steps` (`[{\"kind\": \"pass\"}, {\"kind\": \"play\", \"card_id\": \"...\"}, ...]`).",
        ],
    }

    messages: list[ChatMessage] = [
        ChatMessage(
            role="system",
            content=(
                "You are Role.AUTHOR for the Digimon TCG Guided Play-In Compiler (`play_in`). "
                "Pure Rust beam search could not reach this mid-game board (e.g. `<Delay>` Option "
                "placed on an earlier turn or multi-step effect setup). Construct a legal Turn-1 "
                "stacked opening deck + action prefix that reaches functional equivalence on "
                "`target_puzzle` and keeps `target` / `avoid` lines valid."
            ),
        ),
        ChatMessage(role="user", content=json.dumps(prompt_payload, indent=2)),
    ]

    last_distance = float(mcp_res.get("distance_score", 999.0))
    last_error = str(mcp_res.get("error") or "")

    for attempt in range(1, int(max_llm_attempts) + 1):
        opened_game_ids: list[str] = []
        try:
            if avail_mcp_tools and max_tool_turns > 0:
                def _exec_tool(tc: ToolCall) -> dict[str, Any]:
                    res = engine_mcp.call_tool(tc.name, tc.arguments)
                    if tc.name == "new_game_from_decks" and res.get("game_id"):
                        opened_game_ids.append(str(res["game_id"]))
                    return res

                loop_res = router.run_tool_loop(
                    Role.AUTHOR,
                    messages,
                    tools=avail_mcp_tools,
                    tool_executor=_exec_tool,
                    max_turns=int(max_tool_turns),
                    final_json_schema=PLAY_IN_FALLBACK_SCHEMA,
                    schema_name="play_in_plan",
                )
                messages = list(loop_res.messages)
                proposal = loop_res.final_response.parsed_json or {}
            else:
                proposal = router.complete_structured(
                    Role.AUTHOR,
                    messages,
                    json_schema=PLAY_IN_FALLBACK_SCHEMA,
                    schema_name="play_in_plan",
                )
                messages.append(ChatMessage(role="assistant", content=json.dumps(proposal)))
        finally:
            if engine_mcp.has_tool("close_game"):
                for gid in opened_game_ids:
                    try:
                        engine_mcp.call_tool("close_game", {"game_id": gid})
                    except Exception:
                        pass

        # Resolve stacked decks for seats 1 & 2
        raw_stacked = proposal.get("stacked_decks") or {}
        raw_opening = proposal.get("opening_stacks") or {}
        cand_decks: dict[str, list[str]] = {}
        for pk in ("1", "2"):
            if isinstance(raw_opening.get(pk), Mapping) and raw_opening[pk]:
                cand_decks[pk] = stack_deck_for_seed(
                    raw_opening[pk],
                    seat=pk,
                    seed=seed,
                    base_deck=base_doc["decks"].get(pk),
                    card_catalog=catalog,
                )
            elif isinstance(raw_stacked.get(pk), list) and len(raw_stacked[pk]) >= 50:
                cand_decks[pk] = [str(c) for c in raw_stacked[pk]]
            elif isinstance(raw_stacked.get(pk), list) and raw_stacked[pk]:
                # Short prefix list -> treat as draw stack prefix for `seed`
                cand_decks[pk] = stack_deck_for_seed(
                    {"stack": raw_stacked[pk]},
                    seat=pk,
                    seed=seed,
                    base_deck=base_doc["decks"].get(pk),
                    card_catalog=catalog,
                )
            else:
                cand_decks[pk] = list(base_doc["decks"][pk])

        # Replay candidate prefix on real RustHeadlessGame
        try:
            sim = de.RustHeadlessGame(cand_decks["1"], cand_decks["2"], seed=seed)
            raw_prefix = proposal.get("prefix_actions")
            raw_sym = proposal.get("symbolic_steps")

            executed_prefix: list[int] = []
            if isinstance(raw_prefix, list) and raw_prefix:
                # Auto-prepend Mulligan keep `[0, 0]` if the LLM started directly from Breeding (e.g. 60/62)
                first_act = int(raw_prefix[0])
                if first_act not in (0, 1):
                    while (
                        not sim.is_game_over
                        and str((sim.get_rl_state() or {}).get("phase") or "") == "Mulligan"
                    ):
                        sim.step(0)
                        executed_prefix.append(0)
                for step_i, raw_a in enumerate(raw_prefix):
                    if sim.is_game_over:
                        raise ValueError(f"Game ended early before prefix_actions[{step_i}]")
                    aid = int(raw_a)
                    mask = sim.get_action_mask()
                    if aid < 0 or aid >= len(mask) or mask[aid] <= 0.5:
                        legal_now = sim.legal_decoded_actions()
                        raise ValueError(
                            f"Illegal action {aid} at prefix step {step_i} (legal={legal_now[:12]})"
                        )
                    sim.step(aid)
                    executed_prefix.append(aid)
            elif isinstance(raw_sym, list) and raw_sym:
                first_kind = str((raw_sym[0] or {}).get("kind") or (raw_sym[0] or {}).get("action") or "").lower()
                if first_kind not in ("keep", "mulligan_keep"):
                    while (
                        not sim.is_game_over
                        and str((sim.get_rl_state() or {}).get("phase") or "") == "Mulligan"
                    ):
                        sim.step(0)
                        executed_prefix.append(0)
                executed_prefix.extend(_lower_symbolic_steps(sim, raw_sym))
            else:
                while (
                    not sim.is_game_over
                    and str((sim.get_rl_state() or {}).get("phase") or "") == "Mulligan"
                ):
                    sim.step(0)
                    executed_prefix.append(0)

            actual_scen = sim.to_scenario()
            actual_player = int(sim.current_player_id)
        except Exception as exc:
            last_error = f"Replay execution failed on attempt {attempt}: {exc}"
            if attempt < max_llm_attempts:
                messages.append(
                    ChatMessage(
                        role="user",
                        content=json.dumps({"attempt_failed": attempt, "error": last_error}, indent=2),
                    )
                )
            continue

        matched, dist_score, diffs = check_functional_equivalence(
            actual_scen,
            actual_player,
            base_doc,
            strict_zones=bool(strict_zones),
        )
        last_distance = dist_score
        if not matched:
            last_error = f"Board did not match functional equivalence (distance={dist_score:.1f}): {'; '.join(diffs)}"
            if attempt < max_llm_attempts:
                messages.append(
                    ChatMessage(
                        role="user",
                        content=json.dumps(
                            {
                                "attempt_failed": attempt,
                                "distance_score": dist_score,
                                "discrepancies": diffs,
                                "actual_board": {
                                    "state": actual_scen.get("state"),
                                    "current_player": actual_player,
                                    "zones": {
                                        pk: {
                                            "hand": (actual_scen.get("zones", {}).get(pk) or {}).get("hand"),
                                            "field": (actual_scen.get("zones", {}).get(pk) or {}).get("field"),
                                            "breeding": (actual_scen.get("zones", {}).get(pk) or {}).get("breeding"),
                                            "security_count": len(
                                                (actual_scen.get("zones", {}).get(pk) or {}).get("security") or []
                                            ),
                                            "trash": (actual_scen.get("zones", {}).get(pk) or {}).get("trash"),
                                        }
                                        for pk in ("1", "2")
                                    },
                                },
                            },
                            indent=2,
                        ),
                    )
                )
            continue

        remapped_lines = (
            list(proposal["remapped_lines"])
            if isinstance(proposal.get("remapped_lines"), list) and proposal["remapped_lines"]
            else remap_lines_for_replayed_board(base_doc, actual_scen)
        )

        try:
            upgraded = build_upgraded_replay_prefix_puzzle(
                base_doc,
                stacked_decks=cand_decks,
                prefix_actions=executed_prefix,
                remapped_lines=remapped_lines,
                play_in_method="llm_author_fallback",
                card_catalog=catalog,
            )
        except PuzzleValidationError as exc:
            last_error = f"Upgraded puzzle validation error: {exc}"
            if attempt < max_llm_attempts:
                messages.append(
                    ChatMessage(
                        role="user",
                        content=json.dumps({"attempt_failed": attempt, "error": last_error}, indent=2),
                    )
                )
            continue

        verify_res = engine_mcp.call_tool(
            "verify_puzzle",
            {
                "puzzle_json": upgraded,
                "adversary_sims": int(adversary_sims),
                "seeds": int(verify_seeds),
            },
        )
        v_passed, v_reason = evaluate_verify_puzzle_result(verify_res, solver_seat=solver_seat)
        if not v_passed:
            last_error = f"verify_puzzle failed after play-in upgrade: {v_reason}"
            if attempt < max_llm_attempts:
                messages.append(
                    ChatMessage(
                        role="user",
                        content=json.dumps(
                            {
                                "attempt_failed": attempt,
                                "error": last_error,
                                "verify_puzzle_result": verify_res,
                            },
                            indent=2,
                        ),
                    )
                )
            continue

        dcgo_yaml = render_symbolic_dcgo_yaml(
            cand_decks,
            executed_prefix,
            seed=seed,
            rest_deck_p0=rest_deck_p0,
            rest_deck_p1=rest_deck_p1,
            title=base_doc["id"],
        )
        if out_puzzle_path is not None:
            out_puzzle_path.parent.mkdir(parents=True, exist_ok=True)
            out_puzzle_path.write_text(json.dumps(upgraded, indent=2) + "\n", encoding="utf-8")
        if out_dcgo_yaml_path is not None:
            out_dcgo_yaml_path.parent.mkdir(parents=True, exist_ok=True)
            out_dcgo_yaml_path.write_text(dcgo_yaml, encoding="utf-8")

        return PlayInCompileResult(
            ok=True,
            matched=True,
            strict_zones=bool(strict_zones),
            used_llm_fallback=True,
            llm_attempts_used=attempt,
            prefix_actions=executed_prefix,
            stacked_decks=cand_decks,
            remapped_lines=upgraded["lines"],
            distance_score=0.0,
            dcgo_yaml=dcgo_yaml,
            upgraded_puzzle=upgraded,
            out_puzzle_path=out_puzzle_path,
            out_dcgo_yaml_path=out_dcgo_yaml_path,
            verify_result=verify_res,
            mcp_compile_result=mcp_res,
        )

    return PlayInCompileResult(
        ok=True,
        matched=False,
        strict_zones=bool(strict_zones),
        used_llm_fallback=True,
        llm_attempts_used=int(max_llm_attempts),
        distance_score=last_distance,
        mcp_compile_result=mcp_res,
        error=last_error or "Exhausted max_llm_attempts without reaching verified functional equivalence",
    )
