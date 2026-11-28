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

from collections import Counter
import copy
import json
import random
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Optional, Sequence

import gymnasium
import numpy as np

ACTION_PASS = 62
_CARD_DB_CACHE: Any = None


def _get_card_db() -> Any:
    global _CARD_DB_CACHE
    if _CARD_DB_CACHE is None:
        try:
            import digimon_engine

            _CARD_DB_CACHE = digimon_engine.CardDatabase()
        except Exception:
            _CARD_DB_CACHE = False
    return _CARD_DB_CACHE if _CARD_DB_CACHE is not False else None


def _is_digi_egg(card_id: str) -> bool:
    db = _get_card_db()
    if db is None:
        return False
    try:
        card = db.get_card(card_id)
    except Exception:
        return False
    if card is None:
        return False
    if getattr(card, "level", None) == 2:
        return True
    return "DigiEgg" in str(getattr(card, "card_kind", ""))


def _card_name_for(card_id: str) -> Optional[str]:
    db = _get_card_db()
    if db is None:
        return None
    try:
        card = db.get_card(card_id)
    except Exception:
        return None
    return getattr(card, "card_name", None) if card is not None else None


def _normalize_lines(
    raw_lines: Any,
    expectations: Optional[Sequence[dict[str, Any]]] = None,
) -> list[dict[str, Any]]:
    if isinstance(raw_lines, list):
        out: list[dict[str, Any]] = []
        for idx, entry in enumerate(raw_lines, 1):
            if not isinstance(entry, dict):
                continue
            lid = str(entry.get("id") or entry.get("name") or f"line-{idx}")
            out.append(
                {
                    "id": lid,
                    "name": str(entry.get("name") or lid),
                    "role": str(entry.get("role") or "target"),
                    "actions": [int(a) for a in (entry.get("actions") or [])],
                }
            )
        return out
    if isinstance(raw_lines, dict):
        chosen_roots: set[int] = set()
        avoid_roots: set[int] = set()
        for exp in expectations or []:
            if not isinstance(exp, dict):
                continue
            plan = [int(a) for a in (exp.get("plan") or [])]
            if not plan:
                continue
            kind = str(exp.get("kind") or "")
            if kind in ("chosen", "choose", "visit_share_at_least", "visits"):
                chosen_roots.add(plan[0])
            elif kind in ("not_chosen", "avoid"):
                avoid_roots.add(plan[0])
        out_dict: list[dict[str, Any]] = []
        has_target = False
        for idx, (name, acts) in enumerate(raw_lines.items(), 1):
            if not isinstance(acts, list):
                continue
            int_acts = [int(a) for a in acts]
            if int_acts and int_acts[0] in avoid_roots:
                role = "avoid"
            elif int_acts and int_acts[0] in chosen_roots:
                role = "target"
                has_target = True
            elif not chosen_roots and not has_target:
                role = "target"
                has_target = True
            else:
                role = "alternative"
            out_dict.append(
                {
                    "id": str(name) or f"line-{idx}",
                    "name": str(name) or f"line-{idx}",
                    "role": role,
                    "actions": int_acts,
                }
            )
        return out_dict
    return []


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
    lines: list[dict[str, Any]] = field(default_factory=list)
    expectations: list[dict[str, Any]] = field(default_factory=list)
    refutation_triggers: list[dict[str, Any]] = field(default_factory=list)

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
    expectations = [dict(e) for e in (doc.get("expectations") or []) if isinstance(e, dict)]
    lines = _normalize_lines(doc.get("lines"), expectations=expectations)
    refutation_triggers = [
        dict(t) for t in (doc.get("refutation_triggers") or []) if isinstance(t, dict)
    ]
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
        lines=lines,
        expectations=expectations,
        refutation_triggers=refutation_triggers,
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


def select_refutation_action(
    runner: Any,
    mask: Any,
    triggers: Sequence[dict[str, Any]],
) -> Optional[int]:
    """Match opponent interrupt rules (`refutation_triggers` §2.1) against legal actions."""
    if not triggers or runner is None:
        return None

    if mask is None and hasattr(runner, "get_action_mask"):
        mask = runner.get_action_mask()
    if mask is not None:
        mask_arr = np.asarray(mask)
        legal_ids = {int(i) for i in np.flatnonzero(mask_arr > 0.5)}
    else:
        legal_ids = set()

    raw_decoded = (
        list(runner.legal_decoded_actions())
        if hasattr(runner, "legal_decoded_actions") and callable(runner.legal_decoded_actions)
        else []
    )
    pending = (
        runner.get_pending_selection()
        if hasattr(runner, "get_pending_selection") and callable(runner.get_pending_selection)
        else None
    )
    if not isinstance(pending, dict):
        pending = None

    ui_doc: Optional[dict[str, Any]] = None
    ui_pending: Optional[dict[str, Any]] = None
    if hasattr(runner, "to_ui_json") and callable(runner.to_ui_json):
        try:
            ui = runner.to_ui_json()
            if isinstance(ui, dict):
                ui_doc = ui
                if isinstance(ui.get("pendingSelection"), dict):
                    ui_pending = ui["pendingSelection"]
        except Exception:
            ui_pending = None

    choice_card_by_action: dict[int, str] = {}
    choice_label_by_action: dict[int, str] = {}
    for src_pending in (pending, ui_pending):
        if not src_pending:
            continue
        for ch in src_pending.get("effectChoices") or []:
            if not isinstance(ch, dict) or ch.get("actionId") is None:
                continue
            aid = int(ch["actionId"])
            cid = ch.get("cardId") or ch.get("card_id")
            if cid:
                choice_card_by_action[aid] = str(cid)
            if ch.get("label"):
                choice_label_by_action[aid] = str(ch["label"])

    if ui_doc is not None and legal_ids:
        sel_pid = int(
            (pending or {}).get("selectingPlayer")
            or (ui_pending or {}).get("selectingPlayer")
            or getattr(runner, "current_player_id", 0)
            or 0
        )
        p_ui = ui_doc.get(f"player{sel_pid}") if sel_pid in (1, 2) else None
        if isinstance(p_ui, dict):
            hand_ids = list(p_ui.get("handIds") or [])
            ba = list(p_ui.get("battleArea") or [])
            cur_phase_hint = ""
            if hasattr(runner, "get_rl_state") and callable(runner.get_rl_state):
                try:
                    cur_phase_hint = str((runner.get_rl_state() or {}).get("phase") or "").lower()
                except Exception:
                    cur_phase_hint = ""
            pend_kind = str((pending or {}).get("kind") or "").lower()
            for aid in legal_ids:
                if aid in choice_card_by_action or aid == ACTION_PASS:
                    continue
                if 400 <= aid < 1000:
                    h_idx = (aid - 400) // 15
                    if 0 <= h_idx < len(hand_ids) and hand_ids[h_idx]:
                        choice_card_by_action[aid] = str(hand_ids[h_idx])
                elif 0 <= aid < 60 and (cur_phase_hint in ("countertiming", "selecthand", "main") or pend_kind == "hand"):
                    h_idx = aid if aid < 30 else aid - 30
                    if 0 <= h_idx < len(hand_ids) and hand_ids[h_idx]:
                        choice_card_by_action[aid] = str(hand_ids[h_idx])
                elif 1000 <= aid < 1150:
                    s_idx = (aid - 1000) // 10
                    if 0 <= s_idx < len(ba) and isinstance(ba[s_idx], dict) and ba[s_idx].get("topCardId"):
                        choice_card_by_action[aid] = str(ba[s_idx]["topCardId"])
                elif 100 <= aid < 114 and cur_phase_hint == "blocktiming":
                    s_idx = aid - 100
                    if 0 <= s_idx < len(ba) and isinstance(ba[s_idx], dict) and ba[s_idx].get("topCardId"):
                        choice_card_by_action[aid] = str(ba[s_idx]["topCardId"])

    decoded: list[dict[str, Any]] = []
    for d in raw_decoded:
        if not isinstance(d, dict) or d.get("action_id") is None:
            continue
        aid = int(d["action_id"])
        if legal_ids and aid not in legal_ids:
            continue
        entry = dict(d)
        entry["action_id"] = aid
        if not entry.get("card_id") and aid in choice_card_by_action:
            entry["card_id"] = choice_card_by_action[aid]
        if not entry.get("label") and aid in choice_label_by_action:
            entry["label"] = choice_label_by_action[aid]
        decoded.append(entry)

    if not decoded and legal_ids:
        for aid in sorted(legal_ids):
            decoded.append(
                {
                    "action_id": aid,
                    "card_id": choice_card_by_action.get(aid),
                    "label": choice_label_by_action.get(
                        aid, "Pass / decline" if aid == ACTION_PASS else f"Action {aid}"
                    ),
                    "phase": None,
                }
            )
    if not decoded:
        return None
    if not legal_ids:
        legal_ids = {int(d["action_id"]) for d in decoded}

    rl_phase: Optional[str] = None
    if hasattr(runner, "get_rl_state") and callable(runner.get_rl_state):
        try:
            st = runner.get_rl_state()
            if isinstance(st, dict) and st.get("phase") is not None:
                rl_phase = str(st["phase"])
        except Exception:
            rl_phase = None

    phase_tokens: set[str] = set()
    if rl_phase:
        phase_tokens.add(rl_phase.lower())
    for d in decoded:
        if d.get("phase"):
            phase_tokens.add(str(d["phase"]).lower())
    if pending:
        for k in ("phase", "kind"):
            v = pending.get(k)
            if isinstance(v, str) and v:
                phase_tokens.add(v.lower())
                phase_tokens.add(f"select{v.lower()}")

    prompt_source_cards: set[str] = set()
    for src_pending in (pending, ui_pending):
        if not src_pending:
            continue
        for k in ("source_card_id", "sourceCardId"):
            if src_pending.get(k):
                prompt_source_cards.add(str(src_pending[k]))
    prompt_text = str((pending or {}).get("prompt") or "")

    for rule in triggers:
        if not isinstance(rule, dict):
            continue
        rule_phase = rule.get("phase")
        if rule_phase is not None and str(rule_phase).strip():
            if str(rule_phase).strip().lower() not in phase_tokens:
                continue

        rule_card = rule.get("source_card_id")
        if rule_card is not None and str(rule_card).strip():
            wanted_card = str(rule_card).strip()
            card_actions = [
                d
                for d in decoded
                if d["action_id"] != ACTION_PASS and d.get("card_id") == wanted_card
            ]
            prompt_matched = wanted_card in prompt_source_cards or (
                bool(prompt_text) and wanted_card.lower() in prompt_text.lower()
            )
            if not prompt_matched and prompt_text:
                cname = _card_name_for(wanted_card)
                if cname and cname.lower() in prompt_text.lower():
                    prompt_matched = True
            if not card_actions and not prompt_matched:
                continue
            candidate_non_pass = (
                card_actions
                if card_actions
                else [d for d in decoded if d["action_id"] != ACTION_PASS]
            )
        else:
            candidate_non_pass = [d for d in decoded if d["action_id"] != ACTION_PASS]

        prefer = rule.get("prefer_label_contains")
        if prefer is not None and str(prefer).strip():
            needle = str(prefer).strip().lower()
            for d in candidate_non_pass:
                lbl = str(d.get("label") or "")
                if needle in lbl.lower():
                    return int(d["action_id"])

        accept_optional = bool(rule.get("accept_optional", True))
        if accept_optional:
            if candidate_non_pass:
                return int(candidate_non_pass[0]["action_id"])
        else:
            if ACTION_PASS in legal_ids:
                return ACTION_PASS

    return None


def _perm_signature(perm: dict[str, Any]) -> tuple[Any, ...]:
    sources = tuple(
        s.get("cardId") if isinstance(s, dict) else str(s)
        for s in (perm.get("sources") or [])
    )
    return (
        perm.get("topCardId"),
        sources,
        bool(perm.get("isSuspended", False)),
        int(perm.get("dp") or 0),
        int(perm.get("turnPlayed") or 0),
        tuple(perm.get("linkedCardIds") or []),
    )


def _match_live_field_map(
    orig_ba: list[dict[str, Any]],
    perm_ba: list[dict[str, Any]],
    prev_map: dict[int, int],
) -> Optional[dict[int, int]]:
    if len(orig_ba) != len(perm_ba):
        return None
    used: set[int] = set()
    new_map: dict[int, int] = {}
    orig_sigs = [_perm_signature(p) for p in orig_ba]
    perm_sigs = [_perm_signature(p) for p in perm_ba]

    # First pass: honor previous slot assignment when the stack signature still matches.
    for o_idx, o_sig in enumerate(orig_sigs):
        pref = prev_map.get(o_idx)
        if pref is not None and 0 <= pref < len(perm_sigs) and pref not in used and perm_sigs[pref] == o_sig:
            new_map[o_idx] = pref
            used.add(pref)

    # Second pass: match remaining stacks (e.g. newly played or DNA-merged stacks).
    for o_idx, o_sig in enumerate(orig_sigs):
        if o_idx in new_map:
            continue
        matched = None
        for p_idx, p_sig in enumerate(perm_sigs):
            if p_idx not in used and p_sig == o_sig:
                matched = p_idx
                break
        if matched is None:
            return None
        new_map[o_idx] = matched
        used.add(matched)
    return new_map


def _match_live_hand_map(
    orig_hand: list[str],
    perm_hand: list[str],
    prev_map: dict[int, int],
) -> Optional[dict[int, int]]:
    used: set[int] = set()
    new_map: dict[int, int] = {}
    for o_idx, cid in enumerate(orig_hand):
        pref = prev_map.get(o_idx)
        if pref is not None and 0 <= pref < len(perm_hand) and pref not in used and perm_hand[pref] == cid:
            new_map[o_idx] = pref
            used.add(pref)
    # Match newly drawn cards from the end (draws append to hand tail in both games).
    for o_idx in range(len(orig_hand) - 1, -1, -1):
        if o_idx in new_map:
            continue
        cid = orig_hand[o_idx]
        matched = None
        for p_idx in range(len(perm_hand) - 1, -1, -1):
            if p_idx not in used and perm_hand[p_idx] == cid:
                matched = p_idx
                break
        if matched is None:
            return None
        new_map[o_idx] = matched
        used.add(matched)
    return new_map


def _remap_single_action(
    action_id: int,
    *,
    phase: str,
    pending: Optional[dict[str, Any]],
    perm_pending: Optional[dict[str, Any]],
    hand_map: dict[int, int],
    field_map: dict[int, int],
    trash_map: dict[int, int],
) -> Optional[int]:
    a = int(action_id)
    if a in (60, 61, 62, 93, 94, 95):
        return a

    # Owned physical paged card-choice menu (actions 30..57).
    if (
        isinstance(pending, dict)
        and isinstance(pending.get("cardChoice"), dict)
        and isinstance(perm_pending, dict)
        and isinstance(perm_pending.get("cardChoice"), dict)
        and 30 <= a <= 57
    ):
        orig_rows = pending["cardChoice"].get("rows") or []
        perm_rows = perm_pending["cardChoice"].get("rows") or []
        orig_row = next(
            (r for r in orig_rows if isinstance(r, dict) and int(r.get("action_id", -1)) == a),
            None,
        )
        if orig_row is not None:
            zone = orig_row.get("zone")
            cid = orig_row.get("card_id")
            c_slot = orig_row.get("carrier_slot")
            idx = orig_row.get("index")
            mapped_c_slot = field_map.get(int(c_slot), int(c_slot)) if c_slot is not None else None
            mapped_idx = idx
            if zone in (4, "Hand", "hand") and idx is not None:
                mapped_idx = hand_map.get(int(idx))
            elif zone in (5, "Trash", "trash") and idx is not None:
                mapped_idx = trash_map.get(int(idx))
            elif zone in (6, "BattleTop", "battle_top") and c_slot is not None:
                mapped_idx = mapped_c_slot
            for pr in perm_rows:
                if not isinstance(pr, dict) or pr.get("action_id") is None:
                    continue
                if pr.get("zone") != zone or pr.get("card_id") != cid:
                    continue
                if mapped_c_slot is not None and pr.get("carrier_slot") != mapped_c_slot:
                    continue
                if mapped_idx is not None and pr.get("index") is not None and pr.get("index") != mapped_idx:
                    continue
                return int(pr["action_id"])

    p_kind = str((pending or {}).get("kind") or "")
    phase_norm = str(phase or "Main")

    # Selection / interrupt phases.
    if p_kind or phase_norm not in ("Main", "Breeding", "Mulligan", "EndOfTurnAction"):
        if phase_norm in ("BlockTiming", "AllianceTiming"):
            if 100 <= a <= 113:
                mapped = field_map.get(a - 100)
                return 100 + mapped if mapped is not None else None
            return a
        if phase_norm == "CounterTiming":
            if 400 <= a <= 999:
                h_idx, f_idx = divmod(a - 400, 15)
                nh = hand_map.get(h_idx)
                nf = 14 if f_idx == 14 else field_map.get(f_idx)
                if nh is None or nf is None:
                    return None
                return 400 + nh * 15 + nf
            return a
        if 2000 <= a < 2168:
            f_idx, s_idx = divmod(a - 2000, 12)
            nf = field_map.get(f_idx)
            return (2000 + nf * 12 + s_idx) if nf is not None else None
        if 2168 <= a < 2192:
            return a
        if phase_norm in ("SelectMaterial", "SelectSource") or p_kind in ("Material", "Source"):
            if 0 <= a < 14:
                return field_map.get(a)
            return a
        if phase_norm == "SelectHand" or p_kind == "Hand":
            if 0 <= a < 30:
                return hand_map.get(a)
            if 30 <= a < 60:
                nh = hand_map.get(a - 30)
                return (30 + nh) if nh is not None else None
            return a
        if phase_norm == "SelectTrash" or p_kind == "Trash":
            if 130 <= a <= 179:
                nt = trash_map.get(a - 130)
                return (130 + nt) if nt is not None else None
            return a
        if 100 <= a <= 113:
            nf = field_map.get(a - 100)
            return (100 + nf) if nf is not None else None
        if 114 <= a <= 127:
            return a
        return a

    # Main / EndOfTurnAction / Breeding root ranges.
    if 0 <= a < 30:
        return hand_map.get(a)
    if 30 <= a < 60:
        nh = hand_map.get(a - 30)
        return (30 + nh) if nh is not None else None
    if 63 <= a <= 92:
        nh = hand_map.get(a - 63)
        return (63 + nh) if nh is not None else None
    if 100 <= a <= 399:
        slot, target = divmod(a - 100, 15)
        ns = field_map.get(slot)
        return (100 + ns * 15 + target) if ns is not None else None
    if 400 <= a <= 999:
        h_idx, slot = divmod(a - 400, 15)
        nh = hand_map.get(h_idx)
        ns = 14 if slot == 14 else field_map.get(slot)
        if nh is None or ns is None:
            return None
        return 400 + nh * 15 + ns
    if 1000 <= a <= 1149:
        perm_idx, eff_idx = divmod(a - 1000, 10)
        np_idx = 14 if perm_idx == 14 else field_map.get(perm_idx)
        return (1000 + np_idx * 10 + eff_idx) if np_idx is not None else None
    if 1150 <= a <= 1194:
        nt = trash_map.get(a - 1150)
        return (1150 + nt) if nt is not None else None
    return a


def _make_staged_headless_game(puzzle: StudioPuzzle, profile: str = "standard_lite_v2") -> Any:
    import digimon_engine

    game = digimon_engine.RustHeadlessGame(
        list(puzzle.decks["1"]),
        list(puzzle.decks["2"]),
        seed=int(puzzle.seed),
        observation_profile=profile,
    )
    setup = puzzle.setup or {}
    prefix_actions = [int(a) for a in (setup.get("prefix_actions") or [])]
    if setup.get("kind", "staged") == "staged":
        game.skip_mulligan()
        game.apply_scenario(json.dumps(puzzle.to_scenario_dict()))
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


def _advance_interrupts(game: Any, seat: int, triggers: Sequence[dict[str, Any]]) -> bool:
    guard = 0
    while not game.is_game_over and int(game.current_player_id) != int(seat):
        if guard >= 64:
            return False
        guard += 1
        mask = game.get_action_mask()
        act = select_refutation_action(game, mask, triggers)
        if act is None:
            act = int(game.greedy_action())
        if mask[int(act)] <= 0.5:
            valid = np.flatnonzero(np.asarray(mask) > 0.5)
            if len(valid) == 0:
                return False
            act = int(valid[0])
        game.step(int(act))
    return True


def _collect_zone_cards(seat_zones: dict[str, Any]) -> Counter[str]:
    used: Counter[str] = Counter()
    for key in ("hand", "trash", "egg_deck"):
        for cid in seat_zones.get(key) or []:
            if isinstance(cid, str):
                used[cid] += 1
    for sec in seat_zones.get("security") or []:
        if isinstance(sec, str):
            used[sec] += 1
        elif isinstance(sec, dict) and isinstance(sec.get("card_id"), str):
            used[sec["card_id"]] += 1
    br = seat_zones.get("breeding")
    if isinstance(br, list):
        for cid in br:
            if isinstance(cid, str):
                used[cid] += 1
    for st in seat_zones.get("field") or []:
        if not isinstance(st, dict):
            continue
        for cid in st.get("stack") or []:
            if isinstance(cid, str):
                used[cid] += 1
        for cid in st.get("linked_cards") or []:
            if isinstance(cid, str):
                used[cid] += 1
    return used


def permute_staged_puzzle(
    puzzle: StudioPuzzle,
    rng: random.Random,
    max_distractors: int = 2,
) -> StudioPuzzle:
    """Permute solver `hand` + `field` slots (and inject up to `max_distractors`
    remaining main-deck copies into `hand` or `trash`) while remapping lines,
    expectations, and `win_assertions` and verifying `target` lines on a
    scratch `RustHeadlessGame`."""
    setup = puzzle.setup or {}
    if setup.get("kind", "staged") != "staged" or setup.get("prefix_actions"):
        return copy.deepcopy(puzzle)
    zones = setup.get("zones") or {}
    seat_str = str(puzzle.seat)
    if seat_str not in zones or not isinstance(zones[seat_str], dict):
        return copy.deepcopy(puzzle)

    seat_zones_orig = zones[seat_str]
    orig_hand = [str(c) for c in (seat_zones_orig.get("hand") or [])]
    orig_field = copy.deepcopy(list(seat_zones_orig.get("field") or []))
    orig_trash = [str(c) for c in (seat_zones_orig.get("trash") or [])]

    if len(orig_hand) > 30 or len(orig_field) > 14:
        return copy.deepcopy(puzzle)

    win_assertions = list((puzzle.rl_goal or {}).get("win_assertions") or [])
    locked_zones = {
        str(a.get("zone"))
        for a in win_assertions
        if isinstance(a, dict)
        and a.get("kind") == "zone_count"
        and int(a.get("player", 1)) == int(puzzle.seat)
    }

    has_explicit_deck_top = isinstance(seat_zones_orig.get("deck_top"), list)
    orig_deck_top = [str(c) for c in (seat_zones_orig.get("deck_top") or [])]
    egg_set = set(str(c) for c in (seat_zones_orig.get("egg_deck") or []))

    remaining_pool: list[str] = []
    if not has_explicit_deck_top:
        deck_counts = Counter(puzzle.decks.get(seat_str) or [])
        used_counts = _collect_zone_cards(seat_zones_orig)
        for cid, cnt in deck_counts.items():
            rem = cnt - used_counts.get(cid, 0)
            if rem > 0 and cid not in egg_set and not _is_digi_egg(cid):
                remaining_pool.extend([cid] * rem)

    attempts = [True, True, False, False] if max_distractors > 0 else [False]
    for allow_dist in attempts:
        new_seat_zones = copy.deepcopy(seat_zones_orig)
        hand_distractors: list[str] = []
        trash_distractors: list[str] = []

        if allow_dist and max_distractors > 0:
            if has_explicit_deck_top and "deck" not in locked_zones and len(orig_deck_top) > 4:
                # Sample from the bottom region of deck_top so top draws/reveals are unchanged.
                tail_candidates = [
                    (idx, cid)
                    for idx in range(4, len(orig_deck_top))
                    for cid in (orig_deck_top[idx],)
                    if cid not in egg_set and not _is_digi_egg(cid)
                ]
                if tail_candidates:
                    lo = 1 if (len(orig_hand) <= 1 and len(orig_field) <= 1) else 0
                    k = rng.randint(lo, min(max_distractors, len(tail_candidates)))
                    picked = rng.sample(tail_candidates, k) if k > 0 else []
                    picked_indices = {idx for idx, _ in picked}
                    new_seat_zones["deck_top"] = [
                        c for idx, c in enumerate(orig_deck_top) if idx not in picked_indices
                    ]
                    chosen_cards = [cid for _, cid in picked]
                else:
                    chosen_cards = []
            elif not has_explicit_deck_top and remaining_pool:
                lo = 1 if (len(orig_hand) <= 1 and len(orig_field) <= 1) else 0
                k = rng.randint(lo, min(max_distractors, len(remaining_pool)))
                chosen_cards = rng.sample(remaining_pool, k) if k > 0 else []
            else:
                chosen_cards = []

            for cid in chosen_cards:
                can_hand = "hand" not in locked_zones and (len(orig_hand) + len(hand_distractors)) < 10
                can_trash = "trash" not in locked_zones
                if can_hand and (not can_trash or len(orig_hand) <= 1 or rng.random() < 0.6):
                    hand_distractors.append(cid)
                elif can_trash:
                    trash_distractors.append(cid)

        tagged_hand: list[tuple[Optional[int], str]] = [
            (i, c) for i, c in enumerate(orig_hand)
        ] + [(None, d) for d in hand_distractors]
        if len(tagged_hand) > 1 and len(tagged_hand) <= 30:
            rng.shuffle(tagged_hand)
        new_hand = [c for _, c in tagged_hand]
        hand_map = {orig_i: new_i for new_i, (orig_i, _) in enumerate(tagged_hand) if orig_i is not None}

        tagged_field = [(i, copy.deepcopy(st)) for i, st in enumerate(orig_field)]
        if len(tagged_field) > 1:
            rng.shuffle(tagged_field)
        new_field = [st for _, st in tagged_field]
        field_map = {orig_i: new_i for new_i, (orig_i, _) in enumerate(tagged_field)}

        new_trash = list(orig_trash) + trash_distractors
        trash_map = {i: i for i in range(len(orig_trash))}

        new_seat_zones["hand"] = new_hand
        new_seat_zones["field"] = new_field
        if "trash" in new_seat_zones or trash_distractors:
            new_seat_zones["trash"] = new_trash

        cand_setup = copy.deepcopy(setup)
        cand_setup["zones"] = copy.deepcopy(zones)
        cand_setup["zones"][seat_str] = new_seat_zones

        cand_puzzle = copy.deepcopy(puzzle)
        cand_puzzle.setup = cand_setup

        # Validate staging and remap lines/assertions on scratch RustHeadlessGame.
        try:
            orig_root_game = _make_staged_headless_game(puzzle)
            perm_root_game = _make_staged_headless_game(cand_puzzle)
        except Exception:
            continue

        root_phase = str((orig_root_game.get_rl_state() or {}).get("phase") or "Main")
        root_pending = orig_root_game.get_pending_selection()
        perm_root_pending = perm_root_game.get_pending_selection()
        perm_root_mask = np.asarray(perm_root_game.get_action_mask())

        remapped_lines: list[dict[str, Any]] = []
        target_post_field_map: Optional[dict[int, int]] = None
        valid_attempt = True

        for ln in puzzle.lines:
            role = str(ln.get("role") or "target")
            acts = [int(a) for a in (ln.get("actions") or [])]
            if not acts:
                remapped_lines.append(copy.deepcopy(ln))
                continue

            if role != "target" and len(acts) == 1:
                ra = _remap_single_action(
                    acts[0],
                    phase=root_phase,
                    pending=root_pending if isinstance(root_pending, dict) else None,
                    perm_pending=perm_root_pending if isinstance(perm_root_pending, dict) else None,
                    hand_map=hand_map,
                    field_map=field_map,
                    trash_map=trash_map,
                )
                if ra is None or ra < 0 or ra >= len(perm_root_mask) or perm_root_mask[ra] <= 0.5:
                    valid_attempt = False
                    break
                new_ln = copy.deepcopy(ln)
                new_ln["actions"] = [int(ra)]
                remapped_lines.append(new_ln)
                continue

            # Lockstep simulation for target lines and multi-step lines.
            try:
                og = _make_staged_headless_game(puzzle)
                pg = _make_staged_headless_game(cand_puzzle)
            except Exception:
                valid_attempt = False
                break

            cur_hand_map = dict(hand_map)
            cur_field_map = dict(field_map)
            cur_trash_map = dict(trash_map)
            new_acts: list[int] = []

            for step_i, orig_a in enumerate(acts):
                if not _advance_interrupts(og, puzzle.seat, puzzle.refutation_triggers):
                    valid_attempt = False
                    break
                if not _advance_interrupts(pg, puzzle.seat, puzzle.refutation_triggers):
                    valid_attempt = False
                    break
                if og.is_game_over or pg.is_game_over:
                    valid_attempt = False
                    break

                if step_i > 0:
                    o_ui = og.to_ui_json().get(f"player{puzzle.seat}", {})
                    p_ui = pg.to_ui_json().get(f"player{puzzle.seat}", {})
                    fm = _match_live_field_map(
                        o_ui.get("battleArea") or [],
                        p_ui.get("battleArea") or [],
                        cur_field_map,
                    )
                    hm = _match_live_hand_map(
                        o_ui.get("handIds") or [],
                        p_ui.get("handIds") or [],
                        cur_hand_map,
                    )
                    if fm is None or hm is None:
                        valid_attempt = False
                        break
                    cur_field_map = fm
                    cur_hand_map = hm

                o_phase = str((og.get_rl_state() or {}).get("phase") or "Main")
                o_pend = og.get_pending_selection()
                p_pend = pg.get_pending_selection()
                mapped_a = _remap_single_action(
                    orig_a,
                    phase=o_phase,
                    pending=o_pend if isinstance(o_pend, dict) else None,
                    perm_pending=p_pend if isinstance(p_pend, dict) else None,
                    hand_map=cur_hand_map,
                    field_map=cur_field_map,
                    trash_map=cur_trash_map,
                )
                o_mask = np.asarray(og.get_action_mask())
                p_mask = np.asarray(pg.get_action_mask())
                if (
                    mapped_a is None
                    or orig_a < 0
                    or orig_a >= len(o_mask)
                    or o_mask[orig_a] <= 0.5
                    or mapped_a < 0
                    or mapped_a >= len(p_mask)
                    or p_mask[mapped_a] <= 0.5
                ):
                    valid_attempt = False
                    break
                og.step(int(orig_a))
                pg.step(int(mapped_a))
                new_acts.append(int(mapped_a))

            if not valid_attempt:
                break

            if role == "target":
                _advance_interrupts(og, puzzle.seat, puzzle.refutation_triggers)
                _advance_interrupts(pg, puzzle.seat, puzzle.refutation_triggers)
                if og.is_game_over != pg.is_game_over or og.winner_id != pg.winner_id:
                    valid_attempt = False
                    break
                if not pg.is_game_over:
                    o_ui = og.to_ui_json().get(f"player{puzzle.seat}", {})
                    p_ui = pg.to_ui_json().get(f"player{puzzle.seat}", {})
                    fm = _match_live_field_map(
                        o_ui.get("battleArea") or [],
                        p_ui.get("battleArea") or [],
                        cur_field_map,
                    )
                    if fm is not None:
                        target_post_field_map = fm

            new_ln = copy.deepcopy(ln)
            new_ln["actions"] = new_acts
            remapped_lines.append(new_ln)

        if not valid_attempt:
            continue

        # Remap expectations at root.
        remapped_expectations: list[dict[str, Any]] = []
        for exp in puzzle.expectations:
            new_exp = copy.deepcopy(exp)
            plan = [int(a) for a in (exp.get("plan") or [])]
            if plan:
                ra0 = _remap_single_action(
                    plan[0],
                    phase=root_phase,
                    pending=root_pending if isinstance(root_pending, dict) else None,
                    perm_pending=perm_root_pending if isinstance(perm_root_pending, dict) else None,
                    hand_map=hand_map,
                    field_map=field_map,
                    trash_map=trash_map,
                )
                if ra0 is not None:
                    new_exp["plan"] = [int(ra0)] + plan[1:]
            remapped_expectations.append(new_exp)

        # Remap field_index in rl_goal.win_assertions for solver seat.
        eff_assertion_field_map = target_post_field_map if target_post_field_map is not None else field_map
        new_goal = copy.deepcopy(puzzle.rl_goal or {"mode": "playout_to_win"})
        if new_goal.get("win_assertions"):
            remapped_wa: list[dict[str, Any]] = []
            for a in new_goal["win_assertions"]:
                na = copy.deepcopy(a)
                if (
                    isinstance(na, dict)
                    and "field_index" in na
                    and int(na.get("player", 1)) == int(puzzle.seat)
                ):
                    old_fi = int(na["field_index"])
                    if old_fi in eff_assertion_field_map:
                        na["field_index"] = int(eff_assertion_field_map[old_fi])
                remapped_wa.append(na)
            new_goal["win_assertions"] = remapped_wa

        cand_puzzle.lines = remapped_lines
        cand_puzzle.expectations = remapped_expectations
        cand_puzzle.rl_goal = new_goal
        return cand_puzzle

    return copy.deepcopy(puzzle)


class AdaptivePuzzleScheduler:
    """Curriculum scheduler prioritizing unsolved puzzles via inverse EMA solve rate."""

    def __init__(
        self,
        puzzles: Sequence[StudioPuzzle],
        *,
        ema_alpha: float = 0.1,
        gamma: float = 2.0,
        floor: float = 0.05,
        seed: Optional[int] = None,
    ):
        if not puzzles:
            raise ValueError("AdaptivePuzzleScheduler requires at least one StudioPuzzle")
        if not (0.0 < ema_alpha <= 1.0):
            raise ValueError(f"ema_alpha must lie in (0, 1], got {ema_alpha}")
        if gamma < 0.0:
            raise ValueError(f"gamma must be >= 0, got {gamma}")
        if floor < 0.0:
            raise ValueError(f"floor must be >= 0, got {floor}")
        self._puzzles: list[StudioPuzzle] = list(puzzles)
        self.ema_alpha = float(ema_alpha)
        self.gamma = float(gamma)
        self.floor = float(floor)
        self._rng = random.Random(seed)
        self._stats: dict[str, dict[str, float | int]] = {
            p.id: {"attempts": 0, "solves": 0, "ema_solve_rate": 0.0}
            for p in self._puzzles
        }

    @property
    def puzzles(self) -> list[StudioPuzzle]:
        return list(self._puzzles)

    def weight_for(self, puzzle_id: str) -> float:
        st = self._stats.get(puzzle_id)
        ema = float(st["ema_solve_rate"]) if st is not None else 0.0
        return float((1.0 - max(0.0, min(1.0, ema))) ** self.gamma + self.floor)

    def weights(self) -> list[float]:
        return [self.weight_for(p.id) for p in self._puzzles]

    def sample(self, rng: Optional[random.Random] = None) -> StudioPuzzle:
        chooser = rng if rng is not None else self._rng
        ws = self.weights()
        return chooser.choices(self._puzzles, weights=ws, k=1)[0]

    def record_outcome(self, puzzle_id: str, solved: bool) -> None:
        st = self._stats.setdefault(
            puzzle_id, {"attempts": 0, "solves": 0, "ema_solve_rate": 0.0}
        )
        st["attempts"] = int(st["attempts"]) + 1
        if solved:
            st["solves"] = int(st["solves"]) + 1
        target = 1.0 if solved else 0.0
        ema = float(st["ema_solve_rate"])
        st["ema_solve_rate"] = (1.0 - self.ema_alpha) * ema + self.ema_alpha * target

    def update(self, puzzle_id: str, solved: bool) -> None:
        self.record_outcome(puzzle_id, solved)

    def get_stats(self) -> dict[str, dict[str, Any]]:
        out: dict[str, dict[str, Any]] = {}
        for p in self._puzzles:
            st = self._stats[p.id]
            out[p.id] = {
                "attempts": int(st["attempts"]),
                "solves": int(st["solves"]),
                "ema_solve_rate": float(st["ema_solve_rate"]),
                "weight": self.weight_for(p.id),
            }
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


def _normalize_puzzle_to_seat_1(puzzle: StudioPuzzle) -> StudioPuzzle:
    """Swap seats 1 <-> 2 on a staged `seat == 2` puzzle when running inside single-agent
    `OpponentWrapper` (where Player 1 is always the learner and Player 2 is auto-played)."""
    if int(puzzle.seat) == 1 or (puzzle.setup or {}).get("kind", "staged") != "staged":
        return puzzle
    if (puzzle.setup or {}).get("prefix_actions"):
        return puzzle
    swapped = copy.deepcopy(puzzle)
    swapped.seat = 1
    swapped.decks = {
        "1": list(puzzle.decks["2"]),
        "2": list(puzzle.decks["1"]),
    }
    setup = copy.deepcopy(puzzle.setup or {})
    state = dict(setup.get("state") or {})
    fp = int(state.get("first_player", 2))
    state["first_player"] = 1 if fp == 2 else 2
    setup["state"] = state
    zones = dict(setup.get("zones") or {})
    z1 = copy.deepcopy(zones.get("2") or {})
    z2 = copy.deepcopy(zones.get("1") or {})
    setup["zones"] = {"1": z1, "2": z2}
    swapped.setup = setup
    goal = copy.deepcopy(puzzle.rl_goal or {"mode": "playout_to_win"})
    if goal.get("win_assertions"):
        new_wa = []
        for a in goal["win_assertions"]:
            na = copy.deepcopy(a)
            if isinstance(na, dict) and "player" in na:
                na["player"] = 1 if int(na["player"]) == 2 else 2
            new_wa.append(na)
        goal["win_assertions"] = new_wa
    swapped.rl_goal = goal
    return swapped


def _find_wrapper_by_name(env: gymnasium.Env, cls_name: str) -> Optional[Any]:
    cur: Any = env
    while cur is not None:
        if type(cur).__name__ == cls_name:
            return cur
        cur = getattr(cur, "env", None)
    return None


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
        permute: bool = False,
        adaptive: bool = True,
        seed: Optional[int] = None,
    ):
        super().__init__(env)
        if not puzzles:
            raise ValueError("PuzzleResetWrapper requires at least one StudioPuzzle")
        self._puzzles = list(puzzles)
        self._puzzle_prob = float(puzzle_prob)
        self._permute = bool(permute)
        self._adaptive = bool(adaptive)
        self._rng = random.Random(seed)
        self._scheduler: Optional[AdaptivePuzzleScheduler] = (
            AdaptivePuzzleScheduler(self._puzzles, seed=seed) if self._adaptive else None
        )
        self._active_puzzle: Optional[StudioPuzzle] = None
        self._active_puzzle_id: Optional[str] = None
        self._puzzle_actions_taken = 0
        self._start_turn = 1
        self._episode_env: gymnasium.Env = env

    @property
    def active_puzzle(self) -> Optional[StudioPuzzle]:
        return self._active_puzzle

    @property
    def scheduler(self) -> Optional[AdaptivePuzzleScheduler]:
        return self._scheduler

    def get_puzzle_stats(self) -> dict[str, dict[str, Any]]:
        if self._scheduler is not None:
            return self._scheduler.get_stats()
        return {
            p.id: {"attempts": 0, "solves": 0, "ema_solve_rate": 0.0, "weight": 1.0}
            for p in self._puzzles
        }

    def action_mask(self):
        return self.env.unwrapped.action_mask()

    def get_action_mask(self):
        return self.env.unwrapped.get_action_mask()

    def _finish_puzzle_episode(self, solved: bool) -> None:
        if self._scheduler is not None and self._active_puzzle_id is not None:
            self._scheduler.record_outcome(self._active_puzzle_id, bool(solved))
        self._active_puzzle = None
        self._active_puzzle_id = None
        setattr(self.env.unwrapped, "_active_studio_puzzle", None)

    def reset(self, *, seed: Optional[int] = None, options: Optional[dict[str, Any]] = None):
        if seed is not None:
            self._rng.seed(seed)
        self._active_puzzle = None
        self._active_puzzle_id = None
        self._puzzle_actions_taken = 0
        self._episode_env = self.env
        base_digimon = self.env.unwrapped
        setattr(base_digimon, "_active_studio_puzzle", None)

        if self._rng.random() > self._puzzle_prob:
            return self.env.reset(seed=seed, options=options)

        sampled = (
            self._scheduler.sample(self._rng)
            if self._scheduler is not None
            else self._rng.choice(self._puzzles)
        )
        base_id = sampled.id
        puzzle = permute_staged_puzzle(sampled, self._rng) if self._permute else sampled

        opp_wrapper = _find_wrapper_by_name(self.env, "OpponentWrapper")
        match_wrapper = _find_wrapper_by_name(self.env, "MatchEnv")
        if opp_wrapper is not None and puzzle.seat == 2:
            puzzle = _normalize_puzzle_to_seat_1(puzzle)

        self._active_puzzle = puzzle
        self._active_puzzle_id = base_id
        setattr(base_digimon, "_active_studio_puzzle", puzzle)

        # Single-game tactical puzzle episodes bypass BO3 MatchEnv / outer deck wrappers.
        if match_wrapper is not None:
            self._episode_env = match_wrapper.env
        elif opp_wrapper is not None:
            auto_forced = _find_wrapper_by_name(self.env, "AutoAdvanceForcedWrapper")
            self._episode_env = auto_forced if auto_forced is not None else opp_wrapper
        else:
            self._episode_env = self.env

        merged_opts = dict(options or {})
        merged_opts["deck1"] = list(puzzle.decks["1"])
        merged_opts["deck2"] = list(puzzle.decks["2"])

        inner_reset_target = opp_wrapper.env if opp_wrapper is not None else self._episode_env
        if opp_wrapper is not None:
            opp_reset = getattr(opp_wrapper.opponent_fn, "reset_state", None)
            if callable(opp_reset):
                opp_reset()

        obs, info = inner_reset_target.reset(
            seed=puzzle.seed if seed is None else seed,
            options=merged_opts,
        )
        game = getattr(base_digimon, "runner", None) or getattr(base_digimon, "game", None)
        if game is None or not hasattr(game, "apply_scenario"):
            raise RuntimeError(
                "DigimonEnv's RustHeadlessGame does not expose apply_scenario(); rebuild digimon_engine wheel"
            )

        prefix_actions = [int(a) for a in (puzzle.setup.get("prefix_actions") or [])]
        if puzzle.setup.get("kind", "staged") == "staged":
            if hasattr(game, "skip_mulligan"):
                game.skip_mulligan()
            game.apply_scenario(json.dumps(puzzle.to_scenario_dict()))
            for aid in prefix_actions:
                if game.is_game_over:
                    break
                game.step(aid)
        else:
            if not prefix_actions and hasattr(game, "skip_mulligan"):
                game.skip_mulligan()
            else:
                for aid in prefix_actions:
                    if game.is_game_over:
                        break
                    game.step(aid)

        if hasattr(game, "get_events_since_last_step"):
            try:
                game.get_events_since_last_step()
            except Exception:
                pass
        reward_bus = getattr(base_digimon, "_reward_event_bus", None)
        if reward_bus is not None and hasattr(reward_bus, "reset"):
            reward_bus.reset()

        rl_state = game.get_rl_state() if hasattr(game, "get_rl_state") else {}
        self._start_turn = int(rl_state.get("turn_count", 1))

        obs = game.get_board_tensor(1 if opp_wrapper is not None else None)
        mask = base_digimon.action_mask()
        info = dict(info)
        info["action_mask"] = mask

        if opp_wrapper is not None and not base_digimon.is_game_over and base_digimon.current_player_id != 1:
            obs, info = opp_wrapper._advance_opponent(obs, info)
        auto_forced = _find_wrapper_by_name(self._episode_env, "AutoAdvanceForcedWrapper")
        if auto_forced is not None and not base_digimon.is_game_over:
            forced_steps = 0
            while not base_digimon.is_game_over and forced_steps < auto_forced.MAX_FORCED_CHAIN:
                valid = np.flatnonzero(base_digimon.action_mask() > 0)
                if len(valid) != 1:
                    break
                obs, _, term, trunc, info = auto_forced.env.step(int(valid[0]))
                forced_steps += 1
                if term or trunc:
                    break

        info = dict(info)
        info["action_mask"] = base_digimon.action_mask()
        info["studio_puzzle_id"] = puzzle.id
        info["studio_puzzle_goal_mode"] = (puzzle.rl_goal or {}).get("mode", "playout_to_win")
        return obs, info

    def step(self, action):
        puzzle = self._active_puzzle
        if puzzle is None:
            return self.env.step(action)

        obs, reward, terminated, truncated, info = self._episode_env.step(action)
        self._puzzle_actions_taken += 1
        base_digimon = self.env.unwrapped
        game = getattr(base_digimon, "runner", None) or getattr(base_digimon, "game", None)
        goal = puzzle.rl_goal or {}

        if goal.get("mode") != "bounded_horizon":
            if terminated or truncated:
                winner_id = getattr(base_digimon, "winner_id", None)
                solved = winner_id == int(puzzle.seat)
                info = dict(info)
                info["studio_puzzle_solved"] = bool(solved)
                self._finish_puzzle_episode(solved=bool(solved))
            return obs, reward, terminated, truncated, info

        if game is None:
            if terminated or truncated:
                self._finish_puzzle_episode(solved=False)
            return obs, reward, terminated, truncated, info

        win_assertions = goal.get("win_assertions") or []
        if win_assertions and all(_evaluate_goal_assertion(game, a) for a in win_assertions):
            info = dict(info)
            info["studio_puzzle_solved"] = True
            self._finish_puzzle_episode(solved=True)
            return obs, 1.0, True, False, info

        if terminated or truncated:
            winner_id = getattr(base_digimon, "winner_id", None)
            solved = (not win_assertions) and (winner_id == int(puzzle.seat))
            info = dict(info)
            info["studio_puzzle_solved"] = bool(solved)
            self._finish_puzzle_episode(solved=bool(solved))
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
            self._finish_puzzle_episode(solved=False)
            return obs, -1.0, True, False, info

        return obs, reward, terminated, truncated, info

