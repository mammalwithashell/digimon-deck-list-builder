"""SFT & Verifiable-Reward GRPO Dataset Exporter (`dataset_exporter`).

Scans `qa/puzzles/` (`studio-puzzle-v1`) and `qa/search-exams/` (`search-exam-v1`),
enriches every card in play/hand/trash/breeding/security with printed `effect_text`,
`inherited_text`, `security_text`, `link_text`, `traits`, and `evo_costs` via
`CardTextCatalog` (`digimon_engine.CardDatabase` + `data/cards.json`), attaches
concise timing/keyword rules excerpts from `docs/RULES_CONTEXT.md` (and its
canonical `docs/digimon-rules/` tables), and emits two JSONL datasets:

1. `sft_train.jsonl`: OpenAI / TRL `SFTTrainer` chat-messages rows covering:
   - **Task Mode 1 (`solve_position`)**: Given staged board + enriched card texts +
     decoded legal actions, emit `<think>...</think>` + `tactician-solve-v1` JSON
     identifying the winning `target` line and refuted `avoid` trap line(s).
   - **Task Mode 2 (`compose_counter_puzzle`)**: Given Defender Boss board +
     Challenger decklist & card texts + `refutation_triggers`, emit `<think>...</think>`
     + `studio-puzzle-v1` counter-puzzle JSON (Challenger zones + `target`/`avoid` lines).
2. `grpo_prompts.jsonl`: TRL `GRPOTrainer`-ready rows (`prompt`, `task_kind`,
   `ground_truth_meta` carrying `allowed_decks`, `defender_zones`, `base_puzzle`,
   and `refutation_triggers` for `DigimonVerifiableReward`).
"""

from __future__ import annotations

import copy
from dataclasses import dataclass, field
import json
from pathlib import Path
import re
from typing import Any, Mapping, Optional, Sequence

import numpy as np

from digimon_gym.agents.studio_puzzles import _advance_interrupts, _normalize_lines
from tools.puzzle_lab import (
    PUZZLE_SCHEMA,
    SEARCH_EXAM_SCHEMA,
    CardTextCatalog,
    PrintedCardInfo,
    resolve_repo_root,
)

from .verifiable_reward import (
    TACTICIAN_SOLVE_SCHEMA,
    TASK_COMPOSE_COUNTER_PUZZLE,
    TASK_SOLVE_POSITION,
    _build_staged_headless_game,
    _convert_search_exam_to_puzzle_dict,
    score_single_completion,
)

TACTICIAN_SYSTEM_PROMPT = (
    "You are Digimon Tactician, an engine-grounded tactical solver and puzzle composer "
    "for the Digimon Trading Card Game. Always verify memory math, digivolution requirements, "
    "keyword timings (<Blocker>, <Jamming>, <Delay>, <Training>, <Raid>, <Piercing>, <Blitz>, "
    "DNA Digivolution), and opponent [All Turns] / WhenWouldLeave interrupts before committing "
    "to an action sequence. Reason concisely inside <think>...</think>, then output strictly "
    "valid JSON with no trailing commentary."
)

_ANGLE_KEYWORD_RE = re.compile(r"[<＜]([^<>＞]+)[>＞]")

_FALLBACK_RULES_BY_TOPIC: dict[str, str] = {
    "core_victory_and_memory": (
        "Rules §1-2 / §6-1 / §6-5: Victory requires attacking the player at 0 security with a Digimon "
        "capable of >=1 security check (or opponent deck-out). Turn ends once memory moves to >=1 on "
        "opponent's side AND all triggered/pending effects finish resolving. Passing sets opponent memory to 3."
    ),
    "simultaneous_and_immediate_interrupts": (
        "Rules §15-4 / §15-8-5: Immediate-type 'when X would leave/be deleted' effects interrupt BEFORE "
        "removal completes. Simultaneous triggers resolve turn-player first, then non-turn player; newly "
        "derived triggers resolve before older pending triggers."
    ),
    "blocker": (
        "Rules §16-5 <Blocker>: Persistent/Optional in BlockTiming — defender may suspend 1 unsuspended "
        "<Blocker> Digimon to redirect the attack to it (max 1 block per attack)."
    ),
    "jamming": (
        "Rules §16-9 <Jamming>: Persistent — this Digimon is not deleted in battle with Security Digimon."
    ),
    "delay": (
        "Rules §16-17 <Delay>: Optional — trash an Option in the battle area (after its placing turn) to "
        "activate its <Delay> effect without paying color/memory cost."
    ),
    "training": (
        "Rules §16-41 <Training>: Optional cost -> Mandatory — during Main phase, by suspending this Digimon, "
        "place top deck card at bottom of its digivolution cards."
    ),
    "dna_digivolution": (
        "Rules §8-2 DNA Digivolution: Stacks two specified field Digimon into a new unsuspended Digimon "
        "(resets summoning sickness and [Once Per Turn] counts; draws 1 card)."
    ),
    "raid": (
        "Rules §16-23 <Raid>: Optional when attacking — may switch attack target to opponent's unsuspended "
        "Digimon with the highest DP."
    ),
    "piercing": (
        "Rules §16-7 <Piercing>: Mandatory — when deleting opponent's Digimon in battle while attacking, "
        "performs security check(s) after [On Deletion] triggers resolve."
    ),
    "blitz": (
        "Rules §16-16 <Blitz>: Optional — after digivolving when memory moves to >=1 on opponent's side, "
        "this Digimon may declare an attack before turn ends."
    ),
}


@dataclass
class DatasetExportSummary:
    """Summary of an `export_tactician_datasets` run."""

    sft_path: Path
    grpo_path: Path
    source_files_scanned: int
    puzzles_exported: int
    sft_rows: int
    grpo_rows: int
    solve_rows: int
    compose_rows: int
    skipped_unlabeled_ids: list[str] = field(default_factory=list)
    skipped_duplicate_ids: list[str] = field(default_factory=list)

    def to_dict(self) -> dict[str, Any]:
        return {
            "sft_path": str(self.sft_path),
            "grpo_path": str(self.grpo_path),
            "source_files_scanned": self.source_files_scanned,
            "puzzles_exported": self.puzzles_exported,
            "sft_rows": self.sft_rows,
            "grpo_rows": self.grpo_rows,
            "solve_rows": self.solve_rows,
            "compose_rows": self.compose_rows,
            "skipped_unlabeled_ids": list(self.skipped_unlabeled_ids),
            "skipped_duplicate_ids": list(self.skipped_duplicate_ids),
        }


def card_info_to_enriched_dict(info: PrintedCardInfo) -> dict[str, Any]:
    """Serialize `PrintedCardInfo` with explicit text/trait/evo fields always present."""
    return {
        "card_id": info.card_id,
        "card_name": info.card_name,
        "kind": info.card_kind,
        "colors": list(info.colors),
        "level": info.level,
        "play_cost": info.play_cost,
        "dp": info.dp,
        "traits": list(info.traits),
        "evo_costs": [dict(ec) for ec in info.evo_costs],
        "effect_text": info.effect_text,
        "inherited_text": info.inherited_text,
        "security_text": info.security_text,
        "link_text": info.link_text,
    }


def enrich_card_catalog_entries(
    card_ids: Sequence[str],
    *,
    catalog: Optional[CardTextCatalog] = None,
) -> list[dict[str, Any]]:
    """Return deduplicated, order-preserving enriched card metadata dicts for `card_ids`."""
    cat = catalog or CardTextCatalog()
    seen: set[str] = set()
    out: list[dict[str, Any]] = []
    for raw_cid in card_ids:
        cid = str(raw_cid or "").strip()
        if not cid or cid in seen:
            continue
        seen.add(cid)
        info = cat.get(cid)
        if info is not None:
            out.append(card_info_to_enriched_dict(info))
        else:
            out.append(
                {
                    "card_id": cid,
                    "card_name": cid,
                    "kind": "Digimon",
                    "colors": [],
                    "level": None,
                    "play_cost": None,
                    "dp": None,
                    "traits": [],
                    "evo_costs": [],
                    "effect_text": "",
                    "inherited_text": "",
                    "security_text": "",
                    "link_text": "",
                }
            )
    return out


def _load_keyword_table_from_docs(
    repo_root: Path,
    rules_context_path: Optional[Path] = None,
) -> tuple[str, dict[str, str]]:
    """Read `docs/RULES_CONTEXT.md` (and `docs/digimon-rules/keyword-semantics.md` if present)."""
    rc_path = rules_context_path or (repo_root / "docs" / "RULES_CONTEXT.md")
    header_note = ""
    if rc_path.exists():
        raw_rc = rc_path.read_text(encoding="utf-8")
        first_lines = [ln.strip() for ln in raw_rc.splitlines() if ln.strip() and not ln.startswith("#")]
        if first_lines:
            header_note = first_lines[0]

    kw_map: dict[str, str] = {}
    kw_sem_path = repo_root / "docs" / "digimon-rules" / "keyword-semantics.md"
    if kw_sem_path.exists():
        for line in kw_sem_path.read_text(encoding="utf-8").splitlines():
            stripped = line.strip()
            if not stripped.startswith("| `<"):
                continue
            cols = [c.strip() for c in stripped.strip("|").split("|")]
            if len(cols) >= 5:
                kw_col, kind_col, when_col, sem_col, sec_col = cols[:5]
                m = _ANGLE_KEYWORD_RE.search(kw_col)
                if m:
                    base_kw = m.group(1).split()[0].split("(")[0].strip().lower()
                    kw_map[base_kw] = f"§{sec_col} <{m.group(1)}> ({kind_col}; {when_col}): {sem_col}"
    return header_note, kw_map


def extract_rules_excerpts(
    enriched_cards: Sequence[Mapping[str, Any]],
    *,
    repo_root: Optional[Path] = None,
    rules_context_path: Optional[Path] = None,
    max_excerpts: int = 8,
) -> list[str]:
    """Select concise rules & keyword timing excerpts matching the cards in a position."""
    root = repo_root or resolve_repo_root()
    _, doc_kw_map = _load_keyword_table_from_docs(root, rules_context_path=rules_context_path)

    excerpts: list[str] = [
        _FALLBACK_RULES_BY_TOPIC["core_victory_and_memory"],
        _FALLBACK_RULES_BY_TOPIC["simultaneous_and_immediate_interrupts"],
    ]

    combined_blobs: list[str] = []
    for card in enriched_cards:
        for field_key in ("effect_text", "inherited_text", "security_text", "link_text"):
            val = str(card.get(field_key) or "")
            if val:
                combined_blobs.append(val)
    full_text = "\n".join(combined_blobs)
    full_lower = full_text.lower()

    if "dna digivolv" in full_lower or "dna digivolution" in full_lower:
        excerpts.append(_FALLBACK_RULES_BY_TOPIC["dna_digivolution"])

    seen_kws: set[str] = set()
    for m in _ANGLE_KEYWORD_RE.finditer(full_text):
        kw_token = m.group(1).split()[0].split("(")[0].strip().lower()
        if not kw_token or kw_token in seen_kws:
            continue
        seen_kws.add(kw_token)
        if kw_token in doc_kw_map:
            excerpts.append(doc_kw_map[kw_token])
        elif kw_token in _FALLBACK_RULES_BY_TOPIC:
            excerpts.append(_FALLBACK_RULES_BY_TOPIC[kw_token])
        if len(excerpts) >= max_excerpts:
            break

    return excerpts[:max_excerpts]


def _extract_visible_zone_card_ids(zones: Mapping[str, Any], solver_seat: int) -> list[str]:
    """Collect all card IDs in play, breeding, hand, trash, and solver security for enrichment."""
    ordered: list[str] = []

    def _collect_seat(z: Mapping[str, Any], *, include_hand_and_sec: bool) -> None:
        br = z.get("breeding")
        if isinstance(br, list):
            ordered.extend(str(c) for c in br if c)
        for perm in z.get("field") or []:
            if isinstance(perm, Mapping):
                ordered.extend(str(c) for c in (perm.get("stack") or []) if c)
                for lc in perm.get("linked_cards") or []:
                    cid = lc.get("card_id") if isinstance(lc, Mapping) else lc
                    if cid:
                        ordered.append(str(cid))
        if include_hand_and_sec:
            ordered.extend(str(c) for c in (z.get("hand") or []) if isinstance(c, str) and c)
            for s in z.get("security") or []:
                cid = s.get("card_id") if isinstance(s, Mapping) else s
                if cid:
                    ordered.append(str(cid))
        else:
            # In open-list tactical puzzles (such as EX12 Nezhamon / Takutoumon hand traps),
            # defender hand cards are part of the tactical read when staged!
            ordered.extend(str(c) for c in (z.get("hand") or []) if isinstance(c, str) and c)
        ordered.extend(str(c) for c in (z.get("trash") or []) if isinstance(c, str) and c)

    s_key = str(solver_seat)
    d_key = "2" if solver_seat == 1 else "1"
    _collect_seat(zones.get(s_key) or {}, include_hand_and_sec=True)
    _collect_seat(zones.get(d_key) or {}, include_hand_and_sec=False)
    return list(dict.fromkeys(ordered))


def _summarize_public_zones_for_prompt(zones: Mapping[str, Any]) -> dict[str, Any]:
    """Format `zones` for LLM prompts (summarizing library counts instead of 40-card `deck_top` arrays)."""
    out: dict[str, Any] = {}
    for sk in ("1", "2"):
        z = dict(zones.get(sk) or {})
        out[sk] = {
            "breeding": z.get("breeding"),
            "field": z.get("field") or [],
            "hand": list(z.get("hand") or []),
            "trash": list(z.get("trash") or []),
            "security_count": len(z.get("security") or []),
            "deck_count": len(z.get("deck_top") or []),
            "egg_deck_count": len(z.get("egg_deck") or []),
        }
    return out


def _trace_line_on_game(puzzle: Mapping[str, Any], actions: Sequence[int]) -> dict[str, Any]:
    """Step `actions` on `RustHeadlessGame` and collect a human-readable tactical trace."""
    solver_seat = int(puzzle["seat"])
    triggers = list(puzzle.get("refutation_triggers") or [])
    game = _build_staged_headless_game(puzzle)
    _advance_interrupts(game, solver_seat, triggers)

    steps_desc: list[str] = []
    for idx, act_raw in enumerate(actions):
        act = int(act_raw)
        if idx > 0:
            prev_player = int(game.current_player_id)
            _advance_interrupts(game, solver_seat, triggers)
            if prev_player != solver_seat:
                steps_desc.append("opponent interrupt resolved")
        if game.is_game_over:
            break
        mask = np.asarray(game.get_action_mask())
        if int(game.current_player_id) != solver_seat or act < 0 or act >= len(mask) or mask[act] <= 0.5:
            steps_desc.append(f"action {act} (illegal)")
            break
        decoded_map = {
            int(d["action_id"]): str(d.get("label") or f"Action {d['action_id']}")
            for d in (game.legal_decoded_actions() or [])
            if isinstance(d, Mapping) and d.get("action_id") is not None
        }
        lbl = decoded_map.get(act, f"Action {act}")
        game.step(act)
        mem = int((game.get_rl_state() or {}).get("memory", 0))
        steps_desc.append(f"{act} ({lbl} -> mem={mem})")

    _advance_interrupts(game, solver_seat, triggers)
    rl = game.get_rl_state() or {}
    ui = game.to_ui_json()
    p_ui = ui.get(f"player{solver_seat}", {})
    return {
        "steps_summary": " -> ".join(steps_desc),
        "is_game_over": bool(game.is_game_over),
        "winner_id": int(game.winner_id) if game.winner_id is not None else None,
        "final_memory": int(rl.get("memory", 0)),
        "solver_field_count": len(p_ui.get("battleArea") or []),
        "solver_hand_count": len(p_ui.get("handIds") or []),
    }


def _ensure_verifiable_rl_goal(puzzle: dict[str, Any]) -> dict[str, Any]:
    """Ensure converted `search-exam-v1` drills without terminal wins carry bounded_horizon assertions."""
    out = copy.deepcopy(puzzle)
    target_lines = [ln for ln in out.get("lines") or [] if ln.get("role") == "target" and ln.get("actions")]
    if not target_lines:
        return out

    rl_goal = dict(out.get("rl_goal") or {"mode": "playout_to_win", "win_assertions": []})
    if rl_goal.get("mode") == "bounded_horizon" and rl_goal.get("win_assertions"):
        return out

    trace = _trace_line_on_game(out, target_lines[0]["actions"])
    solver_seat = int(out["seat"])
    if trace["is_game_over"] and trace["winner_id"] == solver_seat:
        out["rl_goal"] = {
            "mode": "playout_to_win",
            "max_turns": None,
            "max_actions": None,
            "win_assertions": [{"kind": "winner", "player": solver_seat}],
        }
        return out

    # Non-terminal tactical drill (e.g. choke play or Training digivolve efficiency exam):
    # synthesize deterministic bounded_horizon assertions from the target line's resulting state.
    out["rl_goal"] = {
        "mode": "bounded_horizon",
        "max_turns": 2,
        "max_actions": len(target_lines[0]["actions"]) + 2,
        "win_assertions": [
            {"kind": "memory_equals", "value": trace["final_memory"]},
            {"kind": "zone_count", "player": solver_seat, "zone": "field", "value": trace["solver_field_count"]},
            {"kind": "zone_count", "player": solver_seat, "zone": "hand", "value": trace["solver_hand_count"]},
        ],
    }
    return out


def _build_live_staged_snapshot_puzzle(puzzle: Mapping[str, Any]) -> dict[str, Any]:
    """Materialize a post-prefix `staged` snapshot (`prefix_actions=[]`) at the decision root."""
    game = _build_staged_headless_game(puzzle)
    solver_seat = int(puzzle["seat"])
    _advance_interrupts(game, solver_seat, puzzle.get("refutation_triggers") or [])
    sc = game.to_scenario()
    state = dict(sc.get("state") or {})
    state["first_player"] = int(game.current_player_id)

    snap = copy.deepcopy(dict(puzzle))
    snap["setup"] = {
        "kind": "staged",
        "state": state,
        "zones": copy.deepcopy(sc.get("zones") or {}),
        "prefix_actions": [],
    }
    return snap


def _load_source_puzzles(
    puzzles_dir: Path,
    search_exams_dir: Path,
    *,
    deduplicate_ids: bool = True,
) -> tuple[list[tuple[dict[str, Any], str]], int, list[str], list[str]]:
    """Scan `qa/puzzles/` and `qa/search-exams/`, returning normalized labeled puzzles."""
    seen_ids: set[str] = set()
    skipped_unlabeled: list[str] = []
    skipped_duplicates: list[str] = []
    loaded: list[tuple[dict[str, Any], str]] = []
    files_scanned = 0

    if puzzles_dir.exists():
        for p in sorted(puzzles_dir.rglob("*.json")):
            if p.name == "_library.json":
                continue
            files_scanned += 1
            try:
                raw = json.loads(p.read_text(encoding="utf-8"))
            except Exception:
                continue
            if raw.get("schema") != PUZZLE_SCHEMA:
                continue
            pid = str(raw.get("id") or p.stem)
            exps = [dict(e) for e in (raw.get("expectations") or []) if isinstance(e, Mapping)]
            lines = _normalize_lines(raw.get("lines"), expectations=exps)
            if not any(ln.get("role") == "target" and ln.get("actions") for ln in lines):
                skipped_unlabeled.append(pid)
                continue
            norm = copy.deepcopy(raw)
            norm["lines"] = lines
            norm = _ensure_verifiable_rl_goal(norm)
            seen_ids.add(pid)
            loaded.append((norm, PUZZLE_SCHEMA))

    if search_exams_dir.exists():
        for p in sorted(search_exams_dir.rglob("*.json")):
            if p.name == "_library.json":
                continue
            files_scanned += 1
            try:
                raw = json.loads(p.read_text(encoding="utf-8"))
            except Exception:
                continue
            if raw.get("schema") != SEARCH_EXAM_SCHEMA:
                continue
            pid = str(raw.get("id") or p.stem)
            if deduplicate_ids and pid in seen_ids:
                skipped_duplicates.append(pid)
                continue
            conv = _convert_search_exam_to_puzzle_dict(raw)
            if not any(ln.get("role") == "target" and ln.get("actions") for ln in conv.get("lines") or []):
                skipped_unlabeled.append(pid)
                continue
            conv = _ensure_verifiable_rl_goal(conv)
            seen_ids.add(pid)
            loaded.append((conv, SEARCH_EXAM_SCHEMA))

    return loaded, files_scanned, skipped_unlabeled, skipped_duplicates


def _build_solve_position_pair(
    puzzle: Mapping[str, Any],
    source_schema: str,
    *,
    catalog: CardTextCatalog,
    repo_root: Path,
    rules_context_path: Optional[Path] = None,
) -> tuple[dict[str, Any], dict[str, Any]]:
    """Build `(sft_row, grpo_row)` for Task Mode 1 (`solve_position`)."""
    solver_seat = int(puzzle["seat"])
    defender_seat = 2 if solver_seat == 1 else 1
    triggers = list(puzzle.get("refutation_triggers") or [])

    live_game = _build_staged_headless_game(puzzle)
    _advance_interrupts(live_game, solver_seat, triggers)
    live_sc = live_game.to_scenario()
    live_zones = live_sc.get("zones") or {}
    live_state = dict(live_sc.get("state") or {})

    decoded_actions = [
        {
            "action_id": int(d["action_id"]),
            "kind": d.get("kind"),
            "card_id": d.get("card_id"),
            "label": d.get("label"),
        }
        for d in (live_game.legal_decoded_actions() or [])
        if isinstance(d, Mapping) and d.get("action_id") is not None
    ]

    visible_cids = _extract_visible_zone_card_ids(live_zones, solver_seat)
    for rtr in triggers:
        if rtr.get("source_card_id") and rtr["source_card_id"] not in visible_cids:
            visible_cids.append(str(rtr["source_card_id"]))
    enriched_cards = enrich_card_catalog_entries(visible_cids, catalog=catalog)
    rules_excerpts = extract_rules_excerpts(
        enriched_cards,
        repo_root=repo_root,
        rules_context_path=rules_context_path,
    )

    user_payload = {
        "task_kind": TASK_SOLVE_POSITION,
        "puzzle_id": puzzle["id"],
        "title": puzzle.get("title", puzzle["id"]),
        "solver_seat": solver_seat,
        "defender_seat": defender_seat,
        "state": {
            "memory": int((live_game.get_rl_state() or {}).get("memory", live_state.get("memory", 0))),
            "phase": str((live_game.get_rl_state() or {}).get("phase", live_state.get("phase", "Main"))),
            "turn": int((live_game.get_rl_state() or {}).get("turn_count", live_state.get("turn", 1))),
        },
        "zones": _summarize_public_zones_for_prompt(live_zones),
        "decoded_legal_actions": decoded_actions,
        "card_texts": enriched_cards,
        "refutation_triggers": triggers,
        "rl_goal": puzzle.get("rl_goal"),
        "rules_excerpts": rules_excerpts,
        "output_contract": {
            "schema": TACTICIAN_SOLVE_SCHEMA,
            "required_fields": ["schema", "task_kind", "puzzle_id", "seat", "lines"],
        },
    }
    user_text = (
        "Task Mode 1 (Solve Position): Analyze the staged Digimon TCG board, printed card texts, "
        "opponent [All Turns] / blocker interrupts, and decoded legal actions. Identify the winning "
        "`target` action line and at least one legal `avoid` trap line that gets refuted.\n\n"
        + json.dumps(user_payload, indent=2, ensure_ascii=False)
    )

    target_lines = [ln for ln in puzzle["lines"] if ln["role"] == "target"]
    avoid_lines = [ln for ln in puzzle["lines"] if ln["role"] == "avoid"]
    t_trace = _trace_line_on_game(puzzle, target_lines[0]["actions"])
    a_trace = _trace_line_on_game(puzzle, avoid_lines[0]["actions"]) if avoid_lines else None

    reasoning_lines = [
        f"Position: {puzzle.get('title', puzzle['id'])} (Solver Seat {solver_seat}).",
    ]
    if puzzle.get("description"):
        reasoning_lines.append(f"Tactical read: {puzzle['description']}")
    reasoning_lines.append(
        f"Target line `{target_lines[0]['id']}`: {t_trace['steps_summary']} "
        f"(game_over={t_trace['is_game_over']}, winner={t_trace['winner_id']}, mem={t_trace['final_memory']})."
    )
    if a_trace is not None and avoid_lines:
        reasoning_lines.append(
            f"Refuted trap `{avoid_lines[0]['id']}`: {a_trace['steps_summary']} "
            f"(fails goal: game_over={a_trace['is_game_over']}, winner={a_trace['winner_id']})."
        )

    solve_response = {
        "schema": TACTICIAN_SOLVE_SCHEMA,
        "task_kind": TASK_SOLVE_POSITION,
        "puzzle_id": puzzle["id"],
        "seat": solver_seat,
        "lines": [
            {
                "id": ln["id"],
                "name": ln["name"],
                "role": ln["role"],
                "actions": list(ln["actions"]),
            }
            for ln in puzzle["lines"]
            if ln["role"] in ("target", "avoid")
        ],
    }
    assistant_text = (
        "<think>\n"
        + "\n".join(reasoning_lines)
        + "\n</think>\n"
        + json.dumps(solve_response, indent=2, ensure_ascii=False)
    )

    prompt_messages = [
        {"role": "system", "content": TACTICIAN_SYSTEM_PROMPT},
        {"role": "user", "content": user_text},
    ]
    sft_row = {
        "id": f"{puzzle['id']}::solve",
        "puzzle_id": puzzle["id"],
        "task_kind": TASK_SOLVE_POSITION,
        "source_schema": source_schema,
        "messages": [*prompt_messages, {"role": "assistant", "content": assistant_text}],
    }
    grpo_meta = {
        "puzzle_id": puzzle["id"],
        "task_kind": TASK_SOLVE_POSITION,
        "seat": solver_seat,
        "seed": int(puzzle.get("seed", 0)),
        "allowed_decks": {
            "1": list(puzzle["decks"]["1"]),
            "2": list(puzzle["decks"]["2"]),
        },
        "defender_zones": copy.deepcopy(dict(live_zones.get(str(defender_seat)) or {})),
        "enforce_defender_setup": False,
        "refutation_triggers": copy.deepcopy(triggers),
        "base_puzzle": copy.deepcopy(dict(puzzle)),
    }
    grpo_row = {
        "id": f"{puzzle['id']}::solve",
        "puzzle_id": puzzle["id"],
        "task_kind": TASK_SOLVE_POSITION,
        "prompt": prompt_messages,
        "ground_truth_meta": grpo_meta,
    }
    return sft_row, grpo_row


def _build_compose_counter_puzzle_pair(
    puzzle: Mapping[str, Any],
    source_schema: str,
    *,
    catalog: CardTextCatalog,
    repo_root: Path,
    rules_context_path: Optional[Path] = None,
) -> Optional[tuple[dict[str, Any], dict[str, Any]]]:
    """Build `(sft_row, grpo_row)` for Task Mode 2 (`compose_counter_puzzle`)."""
    snap = _build_live_staged_snapshot_puzzle(puzzle)
    # Confirm snapshot stages and verifies cleanly at >= 0.85 before emitting Mode 2 example.
    check_bd = score_single_completion(json.dumps(snap), adversary_seeds=1)
    if check_bd.stage_reached < 4:
        return None

    solver_seat = int(snap["seat"])
    defender_seat = 2 if solver_seat == 1 else 1
    s_key = str(solver_seat)
    d_key = str(defender_seat)

    zones = snap["setup"]["zones"]
    defender_zones = copy.deepcopy(zones[d_key])
    challenger_deck = list(snap["decks"][s_key])
    defender_deck = list(snap["decks"][d_key])
    triggers = list(snap.get("refutation_triggers") or [])

    defender_cids = _extract_visible_zone_card_ids({"1": {}, "2": defender_zones} if d_key == "2" else {"1": defender_zones, "2": {}}, solver_seat)
    for rtr in triggers:
        if rtr.get("source_card_id") and rtr["source_card_id"] not in defender_cids:
            defender_cids.append(str(rtr["source_card_id"]))
    challenger_unique_cids = list(dict.fromkeys(challenger_deck))

    defender_card_texts = enrich_card_catalog_entries(defender_cids, catalog=catalog)
    challenger_card_texts = enrich_card_catalog_entries(challenger_unique_cids, catalog=catalog)
    rules_excerpts = extract_rules_excerpts(
        [*defender_card_texts, *challenger_card_texts],
        repo_root=repo_root,
        rules_context_path=rules_context_path,
    )

    user_payload = {
        "task_kind": TASK_COMPOSE_COUNTER_PUZZLE,
        "puzzle_id": snap["id"],
        "solver_seat": solver_seat,
        "defender_seat": defender_seat,
        "state": copy.deepcopy(snap["setup"]["state"]),
        "defender_boss_zones": {
            "breeding": defender_zones.get("breeding"),
            "field": defender_zones.get("field") or [],
            "hand": defender_zones.get("hand") or [],
            "security": defender_zones.get("security") or [],
            "trash": defender_zones.get("trash") or [],
        },
        "refutation_triggers": triggers,
        "challenger_deck": challenger_deck,
        "defender_deck": defender_deck,
        "defender_card_texts": defender_card_texts,
        "challenger_card_texts": challenger_card_texts,
        "rl_goal": snap.get("rl_goal"),
        "rules_excerpts": rules_excerpts,
        "output_contract": {
            "schema": PUZZLE_SCHEMA,
            "note": (
                "Emit a complete studio-puzzle-v1 JSON preserving defender_boss_zones and "
                "staging Challenger zones strictly from challenger_deck with verified target and avoid lines."
            ),
        },
    }
    user_text = (
        "Task Mode 2 (Compose Counter-Puzzle): Given the Defender Boss board, [All Turns] "
        "refutation triggers, and Challenger's admitted decklist + card texts, compose Challenger's "
        "mid-game zones and a complete `studio-puzzle-v1` tactical puzzle featuring a verified "
        "`target` answer line and a refuted `avoid` trap line.\n\n"
        + json.dumps(user_payload, indent=2, ensure_ascii=False)
    )

    challenger_z = zones[s_key]
    target_lines = [ln for ln in snap["lines"] if ln["role"] == "target"]
    avoid_lines = [ln for ln in snap["lines"] if ln["role"] == "avoid"]
    reasoning_lines = [
        f"Composing Challenger (Seat {solver_seat}) counter-puzzle against Defender (Seat {defender_seat}) for `{snap['id']}`.",
        f"Stage Challenger hand={challenger_z.get('hand')}, field={[p.get('stack') for p in (challenger_z.get('field') or [])]} "
        f"within allowed deck copy counts.",
        f"Winning target line `{target_lines[0]['id']}` uses actions {target_lines[0]['actions']} to satisfy `{snap['rl_goal']['mode']}`.",
    ]
    if avoid_lines:
        reasoning_lines.append(
            f"Contrast with tempting avoid trap `{avoid_lines[0]['id']}` ({avoid_lines[0]['actions']}), "
            f"which gets refuted by Defender's board/interrupts."
        )

    composed_doc = copy.deepcopy(snap)
    composed_doc["lines"] = [
        {
            "id": ln["id"],
            "name": ln["name"],
            "role": ln["role"],
            "actions": list(ln["actions"]),
        }
        for ln in snap["lines"]
        if ln["role"] in ("target", "avoid")
    ]
    assistant_text = (
        "<think>\n"
        + "\n".join(reasoning_lines)
        + "\n</think>\n"
        + json.dumps(composed_doc, indent=2, ensure_ascii=False)
    )

    prompt_messages = [
        {"role": "system", "content": TACTICIAN_SYSTEM_PROMPT},
        {"role": "user", "content": user_text},
    ]
    sft_row = {
        "id": f"{snap['id']}::compose",
        "puzzle_id": snap["id"],
        "task_kind": TASK_COMPOSE_COUNTER_PUZZLE,
        "source_schema": source_schema,
        "messages": [*prompt_messages, {"role": "assistant", "content": assistant_text}],
    }
    grpo_meta = {
        "puzzle_id": snap["id"],
        "task_kind": TASK_COMPOSE_COUNTER_PUZZLE,
        "seat": solver_seat,
        "seed": int(snap.get("seed", 0)),
        "state": copy.deepcopy(snap["setup"]["state"]),
        "allowed_decks": {
            "1": list(snap["decks"]["1"]),
            "2": list(snap["decks"]["2"]),
        },
        "defender_zones": defender_zones,
        "enforce_defender_setup": True,
        "rl_goal": copy.deepcopy(snap.get("rl_goal") or {"mode": "playout_to_win"}),
        "refutation_triggers": copy.deepcopy(triggers),
        "base_puzzle": composed_doc,
    }
    grpo_row = {
        "id": f"{snap['id']}::compose",
        "puzzle_id": snap["id"],
        "task_kind": TASK_COMPOSE_COUNTER_PUZZLE,
        "prompt": prompt_messages,
        "ground_truth_meta": grpo_meta,
    }
    return sft_row, grpo_row


def export_tactician_datasets(
    *,
    out_dir: Path,
    puzzles_dir: Optional[Path] = None,
    search_exams_dir: Optional[Path] = None,
    cards_json_path: Optional[Path] = None,
    rules_context_path: Optional[Path] = None,
    sft_filename: str = "sft_train.jsonl",
    grpo_filename: str = "grpo_prompts.jsonl",
    deduplicate_ids: bool = True,
) -> DatasetExportSummary:
    """Scan puzzles & search exams and write `sft_train.jsonl` + `grpo_prompts.jsonl`."""
    repo_root = resolve_repo_root()
    p_dir = puzzles_dir if puzzles_dir is not None else (repo_root / "qa" / "puzzles")
    se_dir = search_exams_dir if search_exams_dir is not None else (repo_root / "qa" / "search-exams")
    catalog = CardTextCatalog(cards_json_path=cards_json_path)

    loaded, scanned_count, skipped_unlabeled, skipped_dups = _load_source_puzzles(
        p_dir,
        se_dir,
        deduplicate_ids=deduplicate_ids,
    )

    sft_rows: list[dict[str, Any]] = []
    grpo_rows: list[dict[str, Any]] = []
    solve_count = 0
    compose_count = 0

    for puzzle, src_schema in loaded:
        sft_solve, grpo_solve = _build_solve_position_pair(
            puzzle,
            src_schema,
            catalog=catalog,
            repo_root=repo_root,
            rules_context_path=rules_context_path,
        )
        sft_rows.append(sft_solve)
        grpo_rows.append(grpo_solve)
        solve_count += 1

        compose_pair = _build_compose_counter_puzzle_pair(
            puzzle,
            src_schema,
            catalog=catalog,
            repo_root=repo_root,
            rules_context_path=rules_context_path,
        )
        if compose_pair is not None:
            sft_comp, grpo_comp = compose_pair
            sft_rows.append(sft_comp)
            grpo_rows.append(grpo_comp)
            compose_count += 1

    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    sft_path = out_dir / sft_filename
    grpo_path = out_dir / grpo_filename

    sft_path.write_text(
        "".join(json.dumps(r, ensure_ascii=False) + "\n" for r in sft_rows),
        encoding="utf-8",
    )
    grpo_path.write_text(
        "".join(json.dumps(r, ensure_ascii=False) + "\n" for r in grpo_rows),
        encoding="utf-8",
    )

    return DatasetExportSummary(
        sft_path=sft_path,
        grpo_path=grpo_path,
        source_files_scanned=scanned_count,
        puzzles_exported=len(loaded),
        sft_rows=len(sft_rows),
        grpo_rows=len(grpo_rows),
        solve_rows=solve_count,
        compose_rows=compose_count,
        skipped_unlabeled_ids=skipped_unlabeled,
        skipped_duplicate_ids=skipped_dups,
    )
