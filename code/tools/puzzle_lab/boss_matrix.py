"""Parameterized Boss-Board Template × Challenger Archetype Counter-Matrix (`boss_matrix`).

Sweeps a reusable Defender / Boss-Board Template (such as the built-in
`"ex12-erlangmon-invitation"` preset: `EX12-034` Erlangmon + `TOKEN_KOTENKEN`
9000 DP Blocker + `EX12-071` Saneiketsu Invitation `<Delay>` + Lv.5 `[SW]` &
Lv.6 `[Saneiketsu]` hand/sources + `refutation_triggers`) across:
- Knob axes: `security_counts` (e.g. `[1, 2]`) × `memories` (e.g. `[2, 4]`)
- Challenger Archetypes (from `data/deck_library.json` or a `train_snapshot.json`
  training pool)

For each `(challenger, security_count, memory)` cell, `Role.AUTHOR` composes
Seat 1's mid-game zones strictly from Challenger's admitted decklist and
discovers either a `"lethal_answer"` (`playout_to_win`) or a
`"safe_setup_or_bypass"` (`bounded_horizon`) line contrasted against an `avoid`
trap-triggering line. Candidate puzzles are verified via `digimon-engine-mcp`
(`load_puzzle` -> `score_line` -> `verify_puzzle`) with up to `max_retries`
refutation-feedback turns if Erlangmon's interrupt refutes an attempt.
"""

from __future__ import annotations

import copy
import json
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Mapping, Optional, Sequence

from . import (
    CardTextCatalog,
    PuzzleValidationError,
    evaluate_verify_puzzle_result,
    normalize_studio_puzzle_doc,
    resolve_repo_root,
    safe_under,
    slugify_puzzle_id,
    validate_folder,
    validate_studio_puzzle_doc,
    write_studio_puzzle,
)
from .llm_router import ChatMessage, LLMRouter, Role
from .mcp_client import StdioMcpClient
from .synthesize_archetype import ArchetypeContext, load_archetype_context

VALID_ANSWER_MODES = frozenset({"lethal_answer", "safe_setup_or_bypass", "no_viable_puzzle"})

BOSS_CELL_PROPOSAL_SCHEMA: dict[str, Any] = {
    "type": "object",
    "properties": {
        "answer_mode": {
            "type": "string",
            "enum": ["lethal_answer", "safe_setup_or_bypass", "no_viable_puzzle"],
        },
        "title": {"type": "string"},
        "description": {"type": "string"},
        "tags": {"type": "array", "items": {"type": "string"}},
        "challenger_zones": {"type": "object"},
        "lines": {"type": "array", "items": {"type": "object"}},
        "expectations": {"type": "array", "items": {"type": "object"}},
        "assertions": {"type": "object"},
        "rl_goal": {"type": "object"},
    },
    "required": ["answer_mode", "title", "description", "challenger_zones", "lines"],
    "additionalProperties": False,
}


@dataclass
class BossBoardTemplate:
    """Reusable Seat-2 Defender Boss-Board specification with parameterized security/memory knobs."""

    id: str
    title: str
    description: str
    defender_archetype: str
    defender_deck: list[str]
    defender_zones: dict[str, Any]
    security_pool: list[str]
    refutation_triggers: list[dict[str, Any]] = field(default_factory=list)
    default_turn: int = 4
    default_phase: str = "Main"

    def materialize_defender_zones(self, security_count: int) -> dict[str, Any]:
        """Return a deep copy of Seat 2's zones with `security` populated to `security_count`."""
        zones = copy.deepcopy(self.defender_zones)
        sec_n = max(0, int(security_count))
        pool = list(self.security_pool)
        if not pool:
            pool = [c for c in self.defender_deck if not c.startswith("TOKEN_")]
        security_cards: list[str] = []
        if pool and sec_n > 0:
            for idx in range(sec_n):
                security_cards.append(pool[idx % len(pool)])
        zones["security"] = security_cards
        return zones

    def defender_visible_card_ids(self) -> list[str]:
        cids: list[str] = []
        z = self.defender_zones
        for k in ("hand", "trash"):
            cids.extend(str(c) for c in (z.get(k) or []) if c)
        b = z.get("breeding")
        if isinstance(b, list):
            cids.extend(str(c) for c in b if c)
        for perm in z.get("field") or []:
            if isinstance(perm, Mapping):
                cids.extend(str(c) for c in (perm.get("stack") or []) if c)
        for rtr in self.refutation_triggers:
            if rtr.get("source_card_id"):
                cids.append(str(rtr["source_card_id"]))
        cids.extend(self.security_pool[:2])
        return list(dict.fromkeys(cids))

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> "BossBoardTemplate":
        tid = slugify_puzzle_id(str(data.get("id") or "boss-board"), "boss-board")
        return cls(
            id=tid,
            title=str(data.get("title") or tid),
            description=str(data.get("description") or ""),
            defender_archetype=str(data.get("defender_archetype") or "Defender"),
            defender_deck=[str(c) for c in (data.get("defender_deck") or [])],
            defender_zones=dict(data.get("defender_zones") or {}),
            security_pool=[str(c) for c in (data.get("security_pool") or [])],
            refutation_triggers=list(data.get("refutation_triggers") or []),
            default_turn=int(data.get("default_turn", 4)),
            default_phase=str(data.get("default_phase") or "Main"),
        )


def _make_ex12_erlangmon_invitation_template() -> BossBoardTemplate:
    defender_deck = (
        ["EX12-002"] * 4
        + ["EX12-022"] * 12
        + ["EX12-025"] * 12
        + ["EX12-029"] * 10
        + ["EX12-034"] * 4
        + ["EX12-057"] * 4
        + ["EX12-019"] * 4
        + ["EX12-071"] * 4
    )
    defender_zones = {
        "breeding": None,
        "hand": ["EX12-029", "EX12-057"],  # Lv.5 [SW] Sagomon + Lv.6 [Saneiketsu] Takutoumon
        "field": [
            {
                "stack": ["EX12-022", "EX12-025", "EX12-029", "EX12-034"],
                "is_suspended": False,
                "turn_played": 2,
            },
            {
                "stack": ["TOKEN_KOTENKEN"],
                "is_suspended": False,
                "turn_played": 2,
            },
            {
                "stack": ["EX12-071"],
                "is_suspended": False,
                "turn_played": 2,
            },
        ],
        "trash": ["EX12-022"],
        "egg_deck": ["EX12-002"] * 4,
    }
    refutation_triggers = [
        {
            "source_card_id": "EX12-034",
            "phase": None,
            "accept_optional": True,
            "prefer_label_contains": "Play",
        },
        {
            "source_card_id": "EX12-071",
            "phase": None,
            "accept_optional": True,
            "prefer_label_contains": None,
        },
    ]
    return BossBoardTemplate(
        id="ex12-erlangmon-invitation",
        title="EX12 Erlangmon + Kotenken Blocker + Saneiketsu Invitation Trap",
        description=(
            "Defender controls EX12-034 Erlangmon (12000 DP, Lv.5 [SW] Sagomon in sources; "
            "[All Turns] WhenWouldLeave free-plays a Lv.5 or lower [SW] and bounces Challenger's "
            "lowest-level Digimon), an unsuspended 9000 DP [Kotenken] <Blocker> Token, and "
            "EX12-071 Saneiketsu Invitation (<Delay> free-digivolves a played [SW] Digimon into "
            "Lv.6 [Saneiketsu] EX12-057 Takutoumon in hand)."
        ),
        defender_archetype="Saneiketsu",
        defender_deck=defender_deck,
        defender_zones=defender_zones,
        security_pool=["EX12-022", "EX12-025", "EX12-022", "EX12-025", "EX12-029"],
        refutation_triggers=refutation_triggers,
        default_turn=4,
        default_phase="Main",
    )


BUILTIN_BOSS_TEMPLATES: dict[str, Callable[[], BossBoardTemplate]] = {
    "ex12-erlangmon-invitation": _make_ex12_erlangmon_invitation_template,
}


from typing import Callable  # noqa: E402


def load_boss_template(spec: str | Path | Mapping[str, Any] | BossBoardTemplate) -> BossBoardTemplate:
    """Resolve a `BossBoardTemplate` from a preset name, JSON file path, dict, or instance."""
    if isinstance(spec, BossBoardTemplate):
        return spec
    if isinstance(spec, Mapping):
        return BossBoardTemplate.from_dict(spec)
    if isinstance(spec, Path) or (isinstance(spec, str) and Path(spec).exists()):
        p = Path(spec)
        return BossBoardTemplate.from_dict(json.loads(p.read_text(encoding="utf-8")))
    key = str(spec).strip().lower()
    if key in BUILTIN_BOSS_TEMPLATES:
        return BUILTIN_BOSS_TEMPLATES[key]()
    raise KeyError(
        f"Unknown boss template {spec!r}; available built-in presets: {sorted(BUILTIN_BOSS_TEMPLATES)}"
    )


@dataclass
class BossMatrixModelEval:
    model_id: str
    chosen_action_id: Optional[int]
    chose_target: bool
    chose_avoid: bool
    policy_value: Optional[float] = None
    search_value: Optional[float] = None
    expectations_passed: Optional[bool] = None


@dataclass
class BossMatrixCellOutcome:
    challenger_archetype: str
    challenger_deck_id: str
    security_count: int
    memory: int
    puzzle_id: str
    answer_mode: str = ""
    accepted: bool = False
    attempts_used: int = 0
    rejection_reason: str = ""
    puzzle_path: Optional[Path] = None
    search_exam_path: Optional[Path] = None
    verify_result: Optional[dict[str, Any]] = None
    model_evals: list[BossMatrixModelEval] = field(default_factory=list)


@dataclass
class BossMatrixReport:
    boss_id: str
    boss_title: str
    security_counts: list[int]
    memories: list[int]
    challengers: list[str]
    cells_total: int
    puzzles_written: int
    summary_path: Optional[Path] = None
    cells: list[BossMatrixCellOutcome] = field(default_factory=list)

    def to_summary_dict(self) -> dict[str, Any]:
        return {
            "schema": "boss-counter-matrix-v1",
            "boss_id": self.boss_id,
            "boss_title": self.boss_title,
            "security_counts": list(self.security_counts),
            "memories": list(self.memories),
            "challengers": list(self.challengers),
            "cells_total": self.cells_total,
            "puzzles_written": self.puzzles_written,
            "cells": [
                {
                    "challenger_archetype": c.challenger_archetype,
                    "challenger_deck_id": c.challenger_deck_id,
                    "security_count": c.security_count,
                    "memory": c.memory,
                    "puzzle_id": c.puzzle_id,
                    "answer_mode": c.answer_mode,
                    "accepted": c.accepted,
                    "attempts_used": c.attempts_used,
                    "rejection_reason": c.rejection_reason or None,
                    "puzzle_path": str(c.puzzle_path) if c.puzzle_path else None,
                    "search_exam_path": str(c.search_exam_path) if c.search_exam_path else None,
                    "model_evals": [
                        {
                            "model_id": me.model_id,
                            "chosen_action_id": me.chosen_action_id,
                            "chose_target": me.chose_target,
                            "chose_avoid": me.chose_avoid,
                            "policy_value": me.policy_value,
                            "search_value": me.search_value,
                            "expectations_passed": me.expectations_passed,
                        }
                        for me in c.model_evals
                    ],
                }
                for c in self.cells
            ],
        }


def resolve_challenger_contexts(
    archetypes: Optional[Sequence[str]],
    *,
    training_pool_snapshot: Optional[Path] = None,
    deck_library_path: Optional[Path] = None,
    aliases_path: Optional[Path] = None,
    cards_json_path: Optional[Path] = None,
    require_admissible: bool = True,
    card_catalog: Optional[CardTextCatalog] = None,
) -> list[ArchetypeContext]:
    """Load Challenger `ArchetypeContext`s from a training pool snapshot or `deck_library.json`."""
    catalog = card_catalog or CardTextCatalog(cards_json_path=cards_json_path)

    if training_pool_snapshot is not None:
        snap_doc = json.loads(Path(training_pool_snapshot).read_text(encoding="utf-8"))
        decks_entries = list(snap_doc.get("decks") or [])
        by_arch: dict[str, dict[str, Any]] = {}
        for d in decks_entries:
            if not isinstance(d, Mapping):
                continue
            arch = str(d.get("archetype") or "").strip()
            if arch and arch not in by_arch:
                by_arch[arch] = dict(d)

        wanted = [a.strip() for a in (archetypes or []) if a.strip() and a.strip().lower() != "all"]
        if not wanted:
            wanted = list(snap_doc.get("eligible_archetypes") or by_arch.keys())

        lower_lookup = {k.lower(): k for k in by_arch}
        contexts: list[ArchetypeContext] = []
        for name in wanted:
            canon = by_arch.get(name) and name or lower_lookup.get(name.lower())
            if canon and canon in by_arch:
                entry = by_arch[canon]
                cids = [str(c) for c in (entry.get("card_ids") or entry.get("cards") or []) if c]
                distinct = catalog.describe_cards(list(dict.fromkeys(cids)))
                contexts.append(
                    ArchetypeContext(
                        archetype_name=canon,
                        deck_id=str(entry.get("deck_id") or f"{canon}-snapshot"),
                        deck_source=str(entry.get("source") or "train_snapshot"),
                        deck_cards=cids,
                        distinct_cards=distinct,
                    )
                )
            else:
                contexts.append(
                    load_archetype_context(
                        name,
                        deck_library_path=deck_library_path,
                        aliases_path=aliases_path,
                        cards_json_path=cards_json_path,
                        require_admissible=require_admissible,
                        card_catalog=catalog,
                    )
                )
        return contexts

    if not archetypes:
        raise ValueError("At least one challenger archetype (or --training-pool-snapshot) is required")

    return [
        load_archetype_context(
            name,
            deck_library_path=deck_library_path,
            aliases_path=aliases_path,
            cards_json_path=cards_json_path,
            require_admissible=require_admissible,
            card_catalog=catalog,
        )
        for name in archetypes
        if str(name).strip()
    ]


def validate_challenger_zones_against_deck(
    challenger_zones: Mapping[str, Any],
    deck_cards: Sequence[str],
) -> None:
    """Assert every non-token card staged in Seat 1's zones belongs to `deck_cards` within copy limits."""
    allowed = Counter(str(c) for c in deck_cards if c and not str(c).startswith("TOKEN_"))
    used: Counter[str] = Counter()

    def _count(cid: Any) -> None:
        s = str(cid or "").strip()
        if not s or s.startswith("TOKEN_"):
            return
        used[s] += 1

    b = challenger_zones.get("breeding")
    if isinstance(b, list):
        for c in b:
            _count(c)
    elif isinstance(b, Mapping):
        for c in b.get("stack") or []:
            _count(c)

    for perm in challenger_zones.get("field") or []:
        if isinstance(perm, Mapping):
            for c in perm.get("stack") or []:
                _count(c)
            for lc in perm.get("linked_cards") or []:
                _count(lc["card_id"] if isinstance(lc, Mapping) else lc)
        elif isinstance(perm, list):
            for c in perm:
                _count(c)

    for zone_key in ("hand", "trash", "security"):
        for c in challenger_zones.get(zone_key) or []:
            _count(c["card_id"] if isinstance(c, Mapping) else c)

    violations: list[str] = []
    for cid, cnt in sorted(used.items()):
        max_allowed = allowed.get(cid, 0)
        if max_allowed == 0:
            violations.append(f"{cid} is not in Challenger's decklist")
        elif cnt > max_allowed:
            violations.append(f"{cid} staged {cnt}x but Challenger decklist only contains {max_allowed}x")

    if violations:
        raise PuzzleValidationError("Challenger deck-purity check failed: " + "; ".join(violations))


def _evaluate_models_on_puzzle(
    doc: Mapping[str, Any],
    eval_models: Sequence[str],
    *,
    engine_mcp: StdioMcpClient,
    simulations: int = 64,
) -> list[BossMatrixModelEval]:
    if not eval_models or not engine_mcp.has_tool("inspect_position") or not engine_mcp.has_tool("load_puzzle"):
        return []

    target_roots = {
        int(ln["actions"][0])
        for ln in (doc.get("lines") or [])
        if ln.get("role") == "target" and ln.get("actions")
    }
    avoid_roots = {
        int(ln["actions"][0])
        for ln in (doc.get("lines") or [])
        if ln.get("role") == "avoid" and ln.get("actions")
    }

    loaded = engine_mcp.call_tool("load_puzzle", {"puzzle_json": doc, "mode": "author"})
    gid = loaded.get("game_id")
    if not gid:
        return []

    results: list[BossMatrixModelEval] = []
    try:
        for raw_m in eval_models:
            m_str = str(raw_m).strip()
            if not m_str:
                continue
            model_id = m_str
            p = Path(m_str)
            if p.suffix.lower() == ".onnx" and engine_mcp.has_tool("load_model"):
                model_id = slugify_puzzle_id(p.stem, "eval-model")
                engine_mcp.call_tool("load_model", {"model_id": model_id, "onnx_path": str(p)})

            insp = engine_mcp.call_tool(
                "inspect_position",
                {
                    "game_id": gid,
                    "model_id": model_id,
                    "simulations": int(simulations),
                    "expectations": doc.get("expectations") or [],
                },
            )
            chosen = insp.get("chosen_action_id")
            chosen_int = int(chosen) if chosen is not None else None
            exp_res = insp.get("expectation_results")
            exp_passed: Optional[bool] = None
            if isinstance(exp_res, list) and exp_res:
                exp_passed = all(bool(e.get("passed", False)) for e in exp_res if isinstance(e, Mapping))

            results.append(
                BossMatrixModelEval(
                    model_id=model_id,
                    chosen_action_id=chosen_int,
                    chose_target=chosen_int in target_roots if chosen_int is not None else False,
                    chose_avoid=chosen_int in avoid_roots if chosen_int is not None else False,
                    policy_value=float(insp["policy_value"]) if insp.get("policy_value") is not None else None,
                    search_value=float(insp["search_value"]) if insp.get("search_value") is not None else None,
                    expectations_passed=exp_passed,
                )
            )
    finally:
        if engine_mcp.has_tool("close_game"):
            engine_mcp.call_tool("close_game", {"game_id": gid})

    return results


def compose_boss_matrix(
    boss_template: str | Path | Mapping[str, Any] | BossBoardTemplate,
    *,
    router: LLMRouter,
    engine_mcp: StdioMcpClient,
    archetypes: Optional[Sequence[str]] = None,
    training_pool_snapshot: Optional[Path] = None,
    security_counts: Sequence[int] = (1, 2),
    memories: Sequence[int] = (2, 4),
    puzzles_dir: Optional[Path] = None,
    search_exams_dir: Optional[Path] = None,
    matrix_folder: Optional[str] = None,
    summary_out_path: Optional[Path] = None,
    deck_library_path: Optional[Path] = None,
    aliases_path: Optional[Path] = None,
    cards_json_path: Optional[Path] = None,
    eval_models: Optional[Sequence[str]] = None,
    eval_simulations: int = 64,
    max_retries: int = 3,
    adversary_sims: int = 64,
    verify_seeds: int = 4,
    require_admissible: bool = True,
    validate_with_engine: bool = False,
    card_catalog: Optional[CardTextCatalog] = None,
) -> BossMatrixReport:
    """Compose & verify a `(challenger × security_count × memory)` counter-puzzle matrix."""
    root = resolve_repo_root()
    catalog = card_catalog or CardTextCatalog(cards_json_path=cards_json_path)
    template = load_boss_template(boss_template)
    out_puzzles_dir = puzzles_dir or (root / "qa" / "puzzles")

    resolved_folder = validate_folder(
        matrix_folder if matrix_folder is not None else f"boss-matrix/{template.id}"
    )

    challengers = resolve_challenger_contexts(
        archetypes,
        training_pool_snapshot=training_pool_snapshot,
        deck_library_path=deck_library_path,
        aliases_path=aliases_path,
        cards_json_path=cards_json_path,
        require_admissible=require_admissible,
        card_catalog=catalog,
    )

    defender_cards_info = catalog.describe_cards(template.defender_visible_card_ids())
    sec_list = [int(s) for s in security_counts]
    mem_list = [int(m) for m in memories]

    cell_outcomes: list[BossMatrixCellOutcome] = []
    written = 0

    for challenger in challengers:
        chal_slug = slugify_puzzle_id(challenger.archetype_name, "challenger")
        deck_copy_counts = dict(Counter(challenger.deck_cards))

        for sec_count in sec_list:
            defender_zones = template.materialize_defender_zones(sec_count)

            for mem in mem_list:
                puzzle_id = slugify_puzzle_id(
                    f"{template.id}-sec{sec_count}-mem{mem}-vs-{chal_slug}"
                )

                cell_prompt = {
                    "puzzle_id": puzzle_id,
                    "boss_template": {
                        "id": template.id,
                        "title": template.title,
                        "description": template.description,
                        "defender_archetype": template.defender_archetype,
                        "seat": 2,
                        "defender_zones": defender_zones,
                        "defender_printed_cards": defender_cards_info,
                        "refutation_triggers": template.refutation_triggers,
                    },
                    "knobs": {
                        "defender_security_count": sec_count,
                        "challenger_starting_memory": mem,
                        "turn": template.default_turn,
                        "phase": template.default_phase,
                    },
                    "challenger": {
                        "archetype": challenger.archetype_name,
                        "deck_id": challenger.deck_id,
                        "seat": 1,
                        "deck_copy_limits": deck_copy_counts,
                        "printed_cards": challenger.distinct_cards,
                    },
                    "instructions": [
                        "Compose a realistic mid-game Seat 1 (`challenger_zones`: hand, field, breeding, trash, security) using ONLY cards in `challenger.deck_copy_limits`.",
                        "Include at least one `role: \"target\"` line and one `role: \"avoid\"` line (where `avoid` walks into Defender's interrupt/Blocker trap).",
                        "Choose `answer_mode: \"lethal_answer\"` (`rl_goal.mode = \"playout_to_win\"`) when Challenger has a deterministic clean lethal at this security/memory knob.",
                        "Choose `answer_mode: \"safe_setup_or_bypass\"` (`rl_goal.mode = \"bounded_horizon\"` with non-empty `win_assertions`) when higher security (e.g. security_count=2) makes reckless removal a punt into Erlangmon + Saneiketsu Invitation.",
                    ],
                }

                messages: list[ChatMessage] = [
                    ChatMessage(
                        role="system",
                        content=(
                            "You are Role.AUTHOR for the Digimon TCG Boss-Board Counter-Matrix Lab. "
                            "Given a fixed Seat 2 Boss Board with `[All Turns]` `refutation_triggers` and a "
                            "Challenger archetype's admitted decklist, compose Seat 1's board state and "
                            "verified `target` vs `avoid` lines as strict JSON."
                        ),
                    ),
                    ChatMessage(role="user", content=json.dumps(cell_prompt, indent=2)),
                ]

                max_attempts = max(1, 1 + int(max_retries))
                accepted_doc: Optional[dict[str, Any]] = None
                last_verify: Optional[dict[str, Any]] = None
                last_reason = ""
                answer_mode = ""
                attempts_used = 0

                for attempt in range(1, max_attempts + 1):
                    attempts_used = attempt
                    proposal = router.complete_structured(
                        Role.AUTHOR,
                        messages,
                        json_schema=BOSS_CELL_PROPOSAL_SCHEMA,
                        schema_name="boss_cell_proposal",
                    )
                    messages.append(ChatMessage(role="assistant", content=json.dumps(proposal)))

                    answer_mode = str(proposal.get("answer_mode") or "lethal_answer")
                    if answer_mode == "no_viable_puzzle":
                        last_reason = str(proposal.get("description") or "Author reported no viable puzzle")
                        break

                    chal_zones = proposal.get("challenger_zones") or {}
                    try:
                        if not isinstance(chal_zones, Mapping):
                            raise PuzzleValidationError("challenger_zones must be an object")
                        validate_challenger_zones_against_deck(chal_zones, challenger.deck_cards)

                        default_rl_goal = (
                            {"mode": "playout_to_win", "max_turns": None, "max_actions": None, "win_assertions": []}
                            if answer_mode == "lethal_answer"
                            else {
                                "mode": "bounded_horizon",
                                "max_turns": 1,
                                "max_actions": 6,
                                "win_assertions": (proposal.get("rl_goal") or {}).get("win_assertions") or [],
                            }
                        )
                        rl_goal_raw = dict(proposal.get("rl_goal") or default_rl_goal)
                        if answer_mode == "safe_setup_or_bypass" and rl_goal_raw.get("mode") != "bounded_horizon":
                            rl_goal_raw["mode"] = "bounded_horizon"

                        tags = list(proposal.get("tags") or [])
                        for extra_tag in (
                            "boss-matrix",
                            template.id,
                            chal_slug,
                            f"sec-{sec_count}",
                            f"mem-{mem}",
                            answer_mode.replace("_", "-"),
                        ):
                            if extra_tag not in tags:
                                tags.append(extra_tag)

                        draft_raw: dict[str, Any] = {
                            "id": puzzle_id,
                            "title": str(proposal.get("title") or puzzle_id),
                            "description": str(proposal.get("description") or ""),
                            "folder": resolved_folder,
                            "tags": tags,
                            "source": {
                                "kind": "composed",
                                "generator": "puzzle_lab.compose_boss_matrix",
                                "boss_template": template.id,
                                "challenger_archetype": challenger.archetype_name,
                                "challenger_deck_id": challenger.deck_id,
                                "security_count": sec_count,
                                "memory": mem,
                                "answer_mode": answer_mode,
                            },
                            "decks": {
                                "1": list(challenger.deck_cards),
                                "2": list(template.defender_deck),
                            },
                            "seed": 0,
                            "seat": 1,
                            "setup": {
                                "kind": "staged",
                                "state": {
                                    "memory": mem,
                                    "phase": template.default_phase,
                                    "turn": template.default_turn,
                                    "first_player": 1,
                                },
                                "zones": {
                                    "1": dict(chal_zones),
                                    "2": copy.deepcopy(defender_zones),
                                },
                                "prefix_actions": [],
                            },
                            "lines": list(proposal.get("lines") or []),
                            "expectations": list(proposal.get("expectations") or []),
                            "assertions": proposal.get("assertions") or {"engine": [], "ui": []},
                            "rl_goal": rl_goal_raw,
                            "refutation_triggers": copy.deepcopy(template.refutation_triggers),
                        }

                        candidate_doc = normalize_studio_puzzle_doc(
                            draft_raw,
                            default_source_kind="composed",
                            card_catalog=catalog,
                        )
                        validate_studio_puzzle_doc(
                            candidate_doc,
                            require_target_line=True,
                            validate_with_engine=validate_with_engine,
                        )
                    except PuzzleValidationError as exc:
                        last_reason = f"Validation error: {exc}"
                        if attempt < max_attempts:
                            messages.append(
                                ChatMessage(
                                    role="user",
                                    content=json.dumps(
                                        {
                                            "attempt_failed": attempt,
                                            "error": last_reason,
                                            "hint": "Fix challenger_zones card-pool limits, rl_goal.win_assertions, or line formatting and retry.",
                                        },
                                        indent=2,
                                    ),
                                )
                            )
                        continue

                    # Stage & run optional `score_line` + mandatory `verify_puzzle` on `digimon-engine-mcp`
                    score_diag: Optional[dict[str, Any]] = None
                    if engine_mcp.has_tool("load_puzzle"):
                        load_res = engine_mcp.call_tool(
                            "load_puzzle",
                            {"puzzle_json": candidate_doc, "mode": "author"},
                        )
                        if load_res.get("ok") is False:
                            last_reason = f"load_puzzle failed: {load_res.get('error')}"
                            if attempt < max_attempts:
                                messages.append(
                                    ChatMessage(
                                        role="user",
                                        content=json.dumps({"attempt_failed": attempt, "error": last_reason}),
                                    )
                                )
                            continue
                        gid = load_res.get("game_id")
                        if gid and engine_mcp.has_tool("score_line"):
                            t_lines = [
                                ln for ln in candidate_doc["lines"] if ln["role"] == "target" and ln["actions"]
                            ]
                            if t_lines:
                                score_diag = engine_mcp.call_tool(
                                    "score_line",
                                    {
                                        "game_id": gid,
                                        "actions": t_lines[0]["actions"],
                                        "adversary_sims": adversary_sims,
                                        "refutation_triggers": candidate_doc.get("refutation_triggers"),
                                    },
                                )
                        if gid and engine_mcp.has_tool("close_game"):
                            engine_mcp.call_tool("close_game", {"game_id": gid})

                    last_verify = engine_mcp.call_tool(
                        "verify_puzzle",
                        {
                            "puzzle_json": candidate_doc,
                            "adversary_sims": adversary_sims,
                            "seeds": verify_seeds,
                        },
                    )
                    passed, v_reason = evaluate_verify_puzzle_result(last_verify, solver_seat=1)
                    if passed:
                        accepted_doc = candidate_doc
                        break

                    last_reason = f"verify_puzzle refuted/rejected candidate: {v_reason}"
                    if attempt < max_attempts:
                        feedback = {
                            "attempt_failed": attempt,
                            "failure_reason": last_reason,
                            "verify_puzzle_result": last_verify,
                            "target_score_line_trace": score_diag,
                            "hint": (
                                "Defender's interrupt (`refutation_triggers`) refuted your `target` line or "
                                "your `avoid` line failed to fail. Adjust Seat 1's staged setup, sequence "
                                "around EX12-034 / EX12-071, or switch `answer_mode` to `safe_setup_or_bypass` "
                                "with `bounded_horizon` `win_assertions`."
                            ),
                        }
                        messages.append(ChatMessage(role="user", content=json.dumps(feedback, indent=2)))

                if accepted_doc is None:
                    cell_outcomes.append(
                        BossMatrixCellOutcome(
                            challenger_archetype=challenger.archetype_name,
                            challenger_deck_id=challenger.deck_id,
                            security_count=sec_count,
                            memory=mem,
                            puzzle_id=puzzle_id,
                            answer_mode=answer_mode,
                            accepted=False,
                            attempts_used=attempts_used,
                            rejection_reason=last_reason or "unverified",
                            verify_result=last_verify,
                        )
                    )
                    continue

                model_evals = _evaluate_models_on_puzzle(
                    accepted_doc,
                    eval_models or [],
                    engine_mcp=engine_mcp,
                    simulations=eval_simulations,
                )

                puzzle_path, exam_path = write_studio_puzzle(
                    accepted_doc,
                    out_puzzles_dir,
                    search_exams_dir=search_exams_dir,
                    require_target_line=True,
                    validate_with_engine=validate_with_engine,
                )
                written += 1
                cell_outcomes.append(
                    BossMatrixCellOutcome(
                        challenger_archetype=challenger.archetype_name,
                        challenger_deck_id=challenger.deck_id,
                        security_count=sec_count,
                        memory=mem,
                        puzzle_id=puzzle_id,
                        answer_mode=answer_mode,
                        accepted=True,
                        attempts_used=attempts_used,
                        puzzle_path=puzzle_path,
                        search_exam_path=exam_path,
                        verify_result=last_verify,
                        model_evals=model_evals,
                    )
                )

    dest_summary = summary_out_path
    if dest_summary is None:
        reports_dir = out_puzzles_dir.resolve().parent / "qa-reports" / "boss-matrix"
        reports_dir.mkdir(parents=True, exist_ok=True)
        dest_summary = reports_dir / f"{template.id}-matrix-summary.json"

    report = BossMatrixReport(
        boss_id=template.id,
        boss_title=template.title,
        security_counts=sec_list,
        memories=mem_list,
        challengers=[c.archetype_name for c in challengers],
        cells_total=len(cell_outcomes),
        puzzles_written=written,
        summary_path=dest_summary,
        cells=cell_outcomes,
    )
    dest_summary.parent.mkdir(parents=True, exist_ok=True)
    dest_summary.write_text(json.dumps(report.to_summary_dict(), indent=2) + "\n", encoding="utf-8")
    return report
