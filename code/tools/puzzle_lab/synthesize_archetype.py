"""Workflow 2: `synthesize_archetype` — Proactive Archetype & Trap Puzzle Synthesis.

Loads an archetype's admitted decklist from `data/deck_library.json` and full
printed card metadata (`effect_text`, `inherited_text`, `security_text`,
`link_text`, `traits`, `evo_costs`, `xros_req`) from `CardDatabase` /
`data/cards.json`, optionally inspects `card_embedding_neighbors` via
`digimon-training-mcp` to surface starved embeddings, prompts `Role.AUTHOR` for
both offensive lethal combos (DNA / Training / Link) and defensive `[All Turns]`
interrupt traps (e.g. `EX12-034` Erlangmon + `EX12-071` Saneiketsu Invitation
with `refutation_triggers`), stages & verifies every blueprint via
`digimon-engine-mcp` (`load_puzzle` -> `score_line` -> `verify_puzzle` ->
optional `plasticity_probe`), and writes ONLY engine-verified `studio-puzzle-v1`
files to `qa/puzzles/`.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Mapping, Optional, Sequence

from . import (
    DEFAULT_FILLER_DECK,
    CardTextCatalog,
    PuzzleValidationError,
    evaluate_verify_puzzle_result,
    normalize_studio_puzzle_doc,
    resolve_repo_root,
    slugify_puzzle_id,
    validate_studio_puzzle_doc,
    write_studio_puzzle,
)
from .llm_router import ChatMessage, LLMRouter, Role, ToolCall
from .mcp_client import StdioMcpClient
from .mine_replays import build_author_tool_specs

SOURCE_PRIORITY = {
    "digilab": 0,
    "digimonmeta": 1,
    "egman": 2,
    "digimoncard_io": 3,
    "file": 4,
    "manual": 5,
    "test": 6,
}

SYNTHESIS_BLUEPRINTS_SCHEMA: dict[str, Any] = {
    "type": "object",
    "properties": {
        "blueprints": {
            "type": "array",
            "items": {"type": "object"},
        }
    },
    "required": ["blueprints"],
    "additionalProperties": False,
}


@dataclass
class ArchetypeContext:
    archetype_name: str
    deck_id: str
    deck_source: str
    deck_cards: list[str]
    distinct_cards: list[dict[str, Any]]
    opponent_archetype: Optional[str] = None
    opponent_deck_cards: list[str] = field(default_factory=lambda: list(DEFAULT_FILLER_DECK))


@dataclass
class StarvedEmbeddingReport:
    checkpoint_path: str
    starved_card_ids: list[str] = field(default_factory=list)
    card_metrics: list[dict[str, Any]] = field(default_factory=list)


@dataclass
class SynthesizedPuzzleOutcome:
    blueprint_index: int
    puzzle_id: str
    category: str
    accepted: bool
    rejection_reason: str = ""
    puzzle_path: Optional[Path] = None
    search_exam_path: Optional[Path] = None
    score_target_result: Optional[dict[str, Any]] = None
    score_avoid_results: list[dict[str, Any]] = field(default_factory=list)
    verify_result: Optional[dict[str, Any]] = None
    plasticity_result: Optional[dict[str, Any]] = None
    refined_turns: int = 0


@dataclass
class SynthesizeArchetypeReport:
    archetype_name: str
    deck_id: str
    blueprints_proposed: int
    puzzles_written: int
    starved_embeddings: Optional[StarvedEmbeddingReport] = None
    outcomes: list[SynthesizedPuzzleOutcome] = field(default_factory=list)


def _normalize_quotes(text: str) -> str:
    return (
        text.replace("’", "'")
        .replace("‘", "'")
        .replace("“", '"')
        .replace("”", '"')
    )


def _parse_decklist_field(raw: Any) -> list[str]:
    if isinstance(raw, list):
        return [str(c).strip() for c in raw if str(c).strip()]
    if isinstance(raw, str):
        s = raw.strip()
        if not s:
            return []
        if s.startswith("["):
            try:
                loaded = json.loads(s)
                if isinstance(loaded, list):
                    return [str(c).strip() for c in loaded if str(c).strip() and not str(c).startswith("Exported")]
            except Exception:
                pass
        try:
            import digimon_engine as de

            if s.startswith("["):
                return list(de.parse_tts(s))
            return list(de.parse_text(s))
        except Exception:
            return [tok.strip() for tok in s.splitlines() if tok.strip() and not tok.startswith("#")]
    return []


def _load_admissible_set(
    repo_root: Path,
    *,
    require_admissible: bool,
) -> tuple[Optional[set[str]], Optional[set[str]]]:
    impl_ids: Optional[set[str]] = None
    try:
        import digimon_engine as de

        impl_ids = set(de.load_implemented_card_ids())
    except Exception:
        impl_ids = None

    admissible_ids: Optional[set[str]] = None
    adm_path = repo_root / "data" / "card_admissibility.json"
    if require_admissible and adm_path.exists():
        try:
            adm_doc = json.loads(adm_path.read_text(encoding="utf-8"))
            cards_map = adm_doc.get("cards") or {}
            if isinstance(cards_map, Mapping):
                admissible_ids = {
                    str(cid)
                    for cid, entry in cards_map.items()
                    if isinstance(entry, Mapping) and entry.get("admissible") is True
                }
        except Exception:
            admissible_ids = None

    return impl_ids, admissible_ids


def _resolve_archetype_entry(
    archetypes_map: Mapping[str, Any],
    wanted_name: str,
    aliases_path: Optional[Path] = None,
) -> tuple[str, Mapping[str, Any]]:
    norm_wanted = _normalize_quotes(wanted_name).strip()
    if norm_wanted in archetypes_map:
        return norm_wanted, archetypes_map[norm_wanted]

    lower_map = {_normalize_quotes(k).lower(): k for k in archetypes_map}
    if norm_wanted.lower() in lower_map:
        canon = lower_map[norm_wanted.lower()]
        return canon, archetypes_map[canon]

    if aliases_path is not None and aliases_path.exists():
        try:
            raw_aliases = json.loads(aliases_path.read_text(encoding="utf-8"))
            for canonical, aliases in raw_aliases.items():
                if str(canonical).startswith("_"):
                    continue
                all_names = [canonical, *(aliases or [])]
                if any(_normalize_quotes(str(a)).lower() == norm_wanted.lower() for a in all_names):
                    if canonical in archetypes_map:
                        return canonical, archetypes_map[canonical]
                    if _normalize_quotes(canonical).lower() in lower_map:
                        k = lower_map[_normalize_quotes(canonical).lower()]
                        return k, archetypes_map[k]
        except Exception:
            pass

    # Substring fallback if unambiguous
    matches = [k for k in archetypes_map if norm_wanted.lower() in _normalize_quotes(k).lower()]
    if len(matches) == 1:
        return matches[0], archetypes_map[matches[0]]

    raise KeyError(f"Archetype {wanted_name!r} not found in deck_library.json")


def _select_best_decklist(
    entry: Mapping[str, Any],
    *,
    impl_ids: Optional[set[str]],
    admissible_ids: Optional[set[str]],
    require_admissible: bool,
) -> tuple[str, str, list[str]]:
    raw_lists = list(entry.get("decklists") or [])
    if not raw_lists:
        raise ValueError(f"Archetype {entry.get('archetype_name')!r} has no decklists")

    def _source_rank(item: Mapping[str, Any]) -> tuple[int, int]:
        src = str(item.get("source") or "").lower()
        top_cut = 0 if item.get("is_top_cut") else 1
        return (SOURCE_PRIORITY.get(src, 99), top_cut)

    sorted_lists = sorted(
        [d for d in raw_lists if isinstance(d, Mapping)],
        key=_source_rank,
    )

    first_playable: Optional[tuple[str, str, list[str]]] = None
    for d in sorted_lists:
        cards = _parse_decklist_field(d.get("decklist"))
        if len(cards) < 40:
            continue
        if impl_ids is not None and not all(cid in impl_ids for cid in cards):
            continue
        deck_id = str(d.get("deck_id") or "deck-0")
        source = str(d.get("source") or "unknown")
        if first_playable is None:
            first_playable = (deck_id, source, cards)
        if admissible_ids is None or all(cid in admissible_ids for cid in cards):
            return deck_id, source, cards

    if not require_admissible and first_playable is not None:
        return first_playable
    if first_playable is not None and admissible_ids is not None:
        # Surface clear error if strict admissibility was requested and no list passed
        raise ValueError(
            f"Archetype {entry.get('archetype_name')!r} has implemented decklists, "
            "but none passed data/card_admissibility.json (pass require_admissible=False to allow implemented-only)."
        )
    raise ValueError(f"Archetype {entry.get('archetype_name')!r} has no fully implemented decklist")


def load_archetype_context(
    archetype_name: str,
    *,
    opponent_archetype: Optional[str] = None,
    deck_library_path: Optional[Path] = None,
    aliases_path: Optional[Path] = None,
    cards_json_path: Optional[Path] = None,
    require_admissible: bool = True,
    card_catalog: Optional[CardTextCatalog] = None,
) -> ArchetypeContext:
    """Load an archetype's admitted decklist + printed card metadata."""
    root = resolve_repo_root()
    lib_path = deck_library_path or (root / "data" / "deck_library.json")
    als_path = aliases_path or (root / "data" / "archetype_aliases.json")
    catalog = card_catalog or CardTextCatalog(cards_json_path=cards_json_path)

    lib_doc = json.loads(lib_path.read_text(encoding="utf-8"))
    archetypes_map = lib_doc.get("archetypes") or {}
    impl_ids, admissible_ids = _load_admissible_set(
        lib_path.parent.parent if lib_path.parent.name == "data" else root,
        require_admissible=require_admissible,
    )

    canon_name, entry = _resolve_archetype_entry(archetypes_map, archetype_name, als_path)
    deck_id, deck_source, deck_cards = _select_best_decklist(
        entry,
        impl_ids=impl_ids,
        admissible_ids=admissible_ids,
        require_admissible=require_admissible,
    )

    opp_name: Optional[str] = None
    opp_cards = list(DEFAULT_FILLER_DECK)
    if opponent_archetype:
        opp_canon, opp_entry = _resolve_archetype_entry(archetypes_map, opponent_archetype, als_path)
        _, _, opp_cards = _select_best_decklist(
            opp_entry,
            impl_ids=impl_ids,
            admissible_ids=admissible_ids,
            require_admissible=require_admissible,
        )
        opp_name = opp_canon

    distinct_cids = list(dict.fromkeys(deck_cards))
    distinct_cards = catalog.describe_cards(distinct_cids)

    return ArchetypeContext(
        archetype_name=canon_name,
        deck_id=deck_id,
        deck_source=deck_source,
        deck_cards=deck_cards,
        distinct_cards=distinct_cards,
        opponent_archetype=opp_name,
        opponent_deck_cards=opp_cards,
    )


def inspect_starved_embeddings(
    training_mcp: StdioMcpClient,
    checkpoint_zip: Path | str,
    card_ids: Sequence[str],
    *,
    top_k: int = 8,
    norm_threshold: float = 0.35,
    min_top_cosine: float = 0.25,
) -> StarvedEmbeddingReport:
    """Query `digimon-training-mcp` `card_embedding_neighbors` to surface starved embeddings."""
    distinct = list(dict.fromkeys(str(c) for c in card_ids if c))
    res = training_mcp.call_tool(
        "card_embedding_neighbors",
        {
            "checkpoint_path": str(checkpoint_zip),
            "card_ids": distinct,
            "top_k": int(top_k),
        },
    )
    cards_list = list(res.get("cards") or [])
    norms = [float(c["l2_norm"]) for c in cards_list if isinstance(c, Mapping) and c.get("l2_norm") is not None]
    median_norm = sorted(norms)[len(norms) // 2] if norms else 1.0
    relative_cutoff = min(norm_threshold, median_norm * 0.55) if norms else norm_threshold

    starved: list[str] = []
    metrics: list[dict[str, Any]] = []
    for c in cards_list:
        if not isinstance(c, Mapping):
            continue
        cid = str(c.get("card_id") or "")
        l2_norm = float(c.get("l2_norm", 1.0))
        neighbors = list(c.get("neighbors") or [])
        top_cos = (
            max((float(n.get("cosine_similarity", 0.0)) for n in neighbors if isinstance(n, Mapping)), default=0.0)
            if neighbors
            else 0.0
        )
        is_starved = (l2_norm <= max(norm_threshold, relative_cutoff)) or (
            bool(neighbors) and top_cos < min_top_cosine
        )
        if is_starved and cid:
            starved.append(cid)
        metrics.append(
            {
                "card_id": cid,
                "card_name": c.get("card_name", cid),
                "l2_norm": l2_norm,
                "top_neighbor_cosine": top_cos,
                "starved": is_starved,
            }
        )

    return StarvedEmbeddingReport(
        checkpoint_path=str(checkpoint_zip),
        starved_card_ids=starved,
        card_metrics=metrics,
    )


def _refine_blueprint_with_author_tools(
    initial_doc: Mapping[str, Any],
    failure_reason: str,
    *,
    router: LLMRouter,
    engine_mcp: StdioMcpClient,
    training_mcp: Optional[StdioMcpClient],
    checkpoint_zip: Optional[Path],
    model_id: Optional[str],
    max_turns: int,
    card_catalog: CardTextCatalog,
) -> tuple[Optional[dict[str, Any]], int]:
    """Allow `Role.AUTHOR` a bounded MCP tool-calling pass to repair a failing blueprint."""
    has_training = training_mcp is not None and checkpoint_zip is not None
    tools = build_author_tool_specs(include_training_tools=has_training)
    saved_override: dict[str, Any] = {}
    staged_override: dict[str, Any] = dict(initial_doc)

    def _exec(tc: ToolCall) -> dict[str, Any]:
        nonlocal saved_override, staged_override
        args = dict(tc.arguments)
        if tc.name == "load_puzzle":
            if isinstance(args.get("puzzle_json"), Mapping):
                staged_override = dict(args["puzzle_json"])
            return engine_mcp.call_tool("load_puzzle", args)
        if tc.name in ("inspect_stack", "inspect_position", "score_line", "verify_puzzle"):
            if model_id and tc.name != "inspect_stack" and "model_id" not in args:
                args["model_id"] = model_id
            return engine_mcp.call_tool(tc.name, args)
        if tc.name == "save_puzzle":
            saved_override = dict(args)
            return {"ok": True, "staged": True}
        if tc.name in ("position_saliency", "plasticity_probe") and training_mcp is not None and checkpoint_zip is not None:
            args.setdefault("checkpoint_path", str(checkpoint_zip))
            return training_mcp.call_tool(tc.name, args)
        return {"ok": False, "error": f"Tool {tc.name!r} unavailable"}

    messages = [
        ChatMessage(
            role="system",
            content=(
                "You are Role.AUTHOR repairing a tactical Digimon puzzle that failed engine verification. "
                "Use `load_puzzle`, `inspect_stack`, `inspect_position`, `score_line`, and `verify_puzzle` "
                "to fix illegal action IDs, memory, or zones, then call `save_puzzle`."
            ),
        ),
        ChatMessage(
            role="user",
            content=json.dumps(
                {"failure_reason": failure_reason, "candidate_puzzle": initial_doc},
                indent=2,
            ),
        ),
    ]
    res = router.run_tool_loop(
        Role.AUTHOR,
        messages,
        tools=tools,
        tool_executor=_exec,
        max_turns=max_turns,
        stop_condition=lambda tc, out: tc.name == "save_puzzle" and bool(out.get("ok")),
    )
    if saved_override:
        merged = dict(staged_override)
        merged.update(saved_override)
        return normalize_studio_puzzle_doc(merged, default_source_kind="composed", card_catalog=card_catalog), res.turns_used
    if res.final_response.parsed_json and isinstance(res.final_response.parsed_json.get("puzzle"), Mapping):
        return (
            normalize_studio_puzzle_doc(
                res.final_response.parsed_json["puzzle"],
                default_source_kind="composed",
                card_catalog=card_catalog,
            ),
            res.turns_used,
        )
    return None, res.turns_used


def synthesize_archetype(
    archetype_name: str,
    *,
    router: LLMRouter,
    engine_mcp: StdioMcpClient,
    training_mcp: Optional[StdioMcpClient] = None,
    opponent_archetype: Optional[str] = None,
    checkpoint_zip: Optional[Path | str] = None,
    model_id: Optional[str] = None,
    puzzles_dir: Optional[Path] = None,
    search_exams_dir: Optional[Path] = None,
    deck_library_path: Optional[Path] = None,
    aliases_path: Optional[Path] = None,
    cards_json_path: Optional[Path] = None,
    folder: Optional[str] = None,
    count: int = 4,
    include_interrupt_traps: bool = True,
    require_admissible: bool = True,
    require_plasticity: bool = False,
    validate_with_engine: bool = False,
    adversary_sims: int = 64,
    verify_seeds: int = 4,
    refinement_turns: int = 0,
    card_catalog: Optional[CardTextCatalog] = None,
) -> SynthesizeArchetypeReport:
    """Synthesize, stage, and engine-verify `studio-puzzle-v1` tactical puzzles for an archetype."""
    root = resolve_repo_root()
    catalog = card_catalog or CardTextCatalog(cards_json_path=cards_json_path)
    out_puzzles_dir = puzzles_dir or (root / "qa" / "puzzles")

    ctx = load_archetype_context(
        archetype_name,
        opponent_archetype=opponent_archetype,
        deck_library_path=deck_library_path,
        aliases_path=aliases_path,
        cards_json_path=cards_json_path,
        require_admissible=require_admissible,
        card_catalog=catalog,
    )

    starved_report: Optional[StarvedEmbeddingReport] = None
    ckpt_path = Path(checkpoint_zip) if checkpoint_zip is not None else None
    if ckpt_path is not None and training_mcp is not None and training_mcp.has_tool("card_embedding_neighbors"):
        distinct_ids = [c["card_id"] for c in ctx.distinct_cards if c.get("card_id")]
        starved_report = inspect_starved_embeddings(
            training_mcp,
            ckpt_path,
            distinct_ids,
        )

    # Include canonical `[All Turns]` interrupt trap cards (EX12-034 Erlangmon + EX12-071 Saneiketsu Invitation)
    # in the reference printed text block so `Role.AUTHOR` can craft adversary-refuted trap puzzles accurately.
    trap_reference_cards = (
        catalog.describe_cards(["EX12-034", "EX12-071"]) if include_interrupt_traps else []
    )
    opp_distinct_cards = (
        catalog.describe_cards(list(dict.fromkeys(ctx.opponent_deck_cards)))
        if ctx.opponent_archetype
        else []
    )

    target_folder = (
        folder
        if folder is not None
        else slugify_puzzle_id(ctx.archetype_name, "synthesized")
    )

    prompt_payload: dict[str, Any] = {
        "archetype": ctx.archetype_name,
        "deck_id": ctx.deck_id,
        "requested_count": int(count),
        "default_folder": target_folder,
        "archetype_cards": ctx.distinct_cards,
        "opponent_archetype": ctx.opponent_archetype,
        "opponent_cards": opp_distinct_cards[:30],
        "interrupt_trap_reference_cards": trap_reference_cards,
        "starved_card_ids": starved_report.starved_card_ids if starved_report else [],
        "design_requirements": [
            "Design minimal, deterministic `studio-puzzle-v1` tactical puzzles for solver `seat: 1`.",
            "Include offensive lethal / value sequencing puzzles (e.g. DNA digivolve, <Delay> Training memory boost, Link / unsuspend lethal).",
            (
                "Include at least one defensive `[All Turns]` interrupt trap puzzle (e.g. opponent EX12-034 Erlangmon "
                "WhenWouldLeave free-play + bounce and/or EX12-071 Saneiketsu Invitation <Delay> digivolve) with "
                "explicit `refutation_triggers` where the naive `avoid` line triggers the punish and `target` plays around it."
                if include_interrupt_traps
                else "Include contrasting `target` and `avoid` lines on every puzzle."
            ),
            "Prioritize starved_card_ids when non-empty so under-trained embeddings receive clean gradient signal.",
        ],
    }

    messages = [
        ChatMessage(
            role="system",
            content=(
                "You are Role.AUTHOR for the Digimon TCG Tactical Puzzle Lab (`synthesize-archetype`). "
                "Propose complete `studio-puzzle-v1` puzzle objects inside `{\"blueprints\": [...]}`. "
                "Each blueprint MUST specify `id`, `title`, `description`, `tags`, `seat` (1 or 2), "
                "`setup` (`kind: \"staged\"`, `state: {memory, phase, turn, first_player}`, `zones: {\"1\": ..., \"2\": ...}`), "
                "`lines` (at least one `role: \"target\"` and one `role: \"avoid\"` with integer action IDs), "
                "optional `refutation_triggers` (`[{source_card_id, phase, accept_optional, prefer_label_contains}]`), "
                "and `rl_goal` (`playout_to_win` or `bounded_horizon`)."
            ),
        ),
        ChatMessage(role="user", content=json.dumps(prompt_payload, indent=2)),
    ]

    structured = router.complete_structured(
        Role.AUTHOR,
        messages,
        json_schema=SYNTHESIS_BLUEPRINTS_SCHEMA,
        schema_name="synthesized_blueprints",
    )
    raw_blueprints = list(structured.get("blueprints") or [])

    outcomes: list[SynthesizedPuzzleOutcome] = []
    written = 0

    for idx, bp in enumerate(raw_blueprints):
        if not isinstance(bp, Mapping):
            continue
        bp_copy = dict(bp)
        bp_copy.setdefault(
            "id",
            slugify_puzzle_id(f"{ctx.archetype_name}-synth-{idx + 1}"),
        )
        if target_folder and "folder" not in bp_copy:
            bp_copy["folder"] = target_folder
        bp_copy.setdefault(
            "decks",
            {
                "1": list(ctx.deck_cards),
                "2": list(ctx.opponent_deck_cards),
            },
        )
        bp_copy.setdefault(
            "source",
            {
                "kind": "composed",
                "generator": "puzzle_lab.synthesize_archetype",
                "archetype": ctx.archetype_name,
                "deck_id": ctx.deck_id,
            },
        )
        tags = list(bp_copy.get("tags") or [])
        arch_tag = slugify_puzzle_id(ctx.archetype_name, "archetype")
        if arch_tag not in tags:
            tags.append(arch_tag)
        if bp_copy.get("refutation_triggers") and "interrupt-trap" not in tags:
            tags.append("interrupt-trap")
        bp_copy["tags"] = tags

        category = "interrupt_trap" if bp_copy.get("refutation_triggers") else str(bp_copy.get("category") or "tactical")

        try:
            doc = normalize_studio_puzzle_doc(
                bp_copy,
                default_source_kind="composed",
                card_catalog=catalog,
            )
            validate_studio_puzzle_doc(
                doc,
                require_target_line=True,
                validate_with_engine=validate_with_engine,
            )
        except PuzzleValidationError as exc:
            outcomes.append(
                SynthesizedPuzzleOutcome(
                    blueprint_index=idx,
                    puzzle_id=str(bp_copy.get("id") or f"blueprint-{idx}"),
                    category=category,
                    accepted=False,
                    rejection_reason=f"Schema validation failed: {exc}",
                )
            )
            continue

        # Stage through `digimon-engine-mcp`: `load_puzzle` -> `score_line` -> `verify_puzzle`.
        score_target_res: Optional[dict[str, Any]] = None
        score_avoid_list: list[dict[str, Any]] = []
        game_id: Optional[str] = None

        if engine_mcp.has_tool("load_puzzle"):
            load_res = engine_mcp.call_tool("load_puzzle", {"puzzle_json": doc, "mode": "author"})
            if load_res.get("ok") is False:
                outcomes.append(
                    SynthesizedPuzzleOutcome(
                        blueprint_index=idx,
                        puzzle_id=doc["id"],
                        category=category,
                        accepted=False,
                        rejection_reason=f"load_puzzle failed: {load_res.get('error')}",
                    )
                )
                continue
            game_id = str(load_res.get("game_id") or "")

        if game_id and engine_mcp.has_tool("score_line"):
            target_lines = [ln for ln in doc["lines"] if ln["role"] == "target" and ln["actions"]]
            avoid_lines = [ln for ln in doc["lines"] if ln["role"] == "avoid" and ln["actions"]]
            if target_lines:
                s_args: dict[str, Any] = {
                    "game_id": game_id,
                    "actions": target_lines[0]["actions"],
                    "adversary_sims": adversary_sims,
                }
                if model_id:
                    s_args["model_id"] = model_id
                if doc.get("refutation_triggers"):
                    s_args["refutation_triggers"] = doc["refutation_triggers"]
                score_target_res = engine_mcp.call_tool("score_line", s_args)

            for av in avoid_lines:
                a_args: dict[str, Any] = {
                    "game_id": game_id,
                    "actions": av["actions"],
                    "adversary_sims": adversary_sims,
                }
                if model_id:
                    a_args["model_id"] = model_id
                if doc.get("refutation_triggers"):
                    a_args["refutation_triggers"] = doc["refutation_triggers"]
                score_avoid_list.append(engine_mcp.call_tool("score_line", a_args))

        if game_id and engine_mcp.has_tool("close_game"):
            engine_mcp.call_tool("close_game", {"game_id": game_id})

        verify_args: dict[str, Any] = {
            "puzzle_json": doc,
            "adversary_sims": adversary_sims,
            "seeds": verify_seeds,
        }
        if model_id:
            verify_args["model_id"] = model_id
        verify_res = engine_mcp.call_tool("verify_puzzle", verify_args)
        passed, reason = evaluate_verify_puzzle_result(verify_res, solver_seat=int(doc["seat"]))

        refined_turns = 0
        if not passed and refinement_turns > 0:
            repaired_doc, refined_turns = _refine_blueprint_with_author_tools(
                doc,
                reason,
                router=router,
                engine_mcp=engine_mcp,
                training_mcp=training_mcp,
                checkpoint_zip=ckpt_path,
                model_id=model_id,
                max_turns=refinement_turns,
                card_catalog=catalog,
            )
            if repaired_doc is not None:
                try:
                    validate_studio_puzzle_doc(
                        repaired_doc,
                        require_target_line=True,
                        validate_with_engine=validate_with_engine,
                    )
                    doc = repaired_doc
                    verify_args["puzzle_json"] = doc
                    verify_res = engine_mcp.call_tool("verify_puzzle", verify_args)
                    passed, reason = evaluate_verify_puzzle_result(
                        verify_res, solver_seat=int(doc["seat"])
                    )
                except PuzzleValidationError as exc:
                    passed, reason = False, str(exc)

        if not passed:
            outcomes.append(
                SynthesizedPuzzleOutcome(
                    blueprint_index=idx,
                    puzzle_id=doc["id"],
                    category=category,
                    accepted=False,
                    rejection_reason=f"verify_puzzle rejected blueprint: {reason}",
                    score_target_result=score_target_res,
                    score_avoid_results=score_avoid_list,
                    verify_result=verify_res,
                    refined_turns=refined_turns,
                )
            )
            continue

        plasticity_res: Optional[dict[str, Any]] = None
        if ckpt_path is not None and training_mcp is not None and training_mcp.has_tool("plasticity_probe"):
            target_root = doc["lines"][0]["actions"][0]
            avoid_roots = [
                ln["actions"][0]
                for ln in doc["lines"]
                if ln["role"] == "avoid" and ln["actions"]
            ]
            plasticity_res = training_mcp.call_tool(
                "plasticity_probe",
                {
                    "checkpoint_path": str(ckpt_path),
                    "scenario_json": {
                        "state": doc["setup"].get("state"),
                        "zones": doc["setup"].get("zones"),
                    },
                    "target_action_id": target_root,
                    "avoid_action_ids": avoid_roots,
                },
            )
            if require_plasticity and plasticity_res.get("learnable") is False:
                outcomes.append(
                    SynthesizedPuzzleOutcome(
                        blueprint_index=idx,
                        puzzle_id=doc["id"],
                        category=category,
                        accepted=False,
                        rejection_reason=f"plasticity_probe failed: {plasticity_res.get('diagnosis')}",
                        score_target_result=score_target_res,
                        score_avoid_results=score_avoid_list,
                        verify_result=verify_res,
                        plasticity_result=plasticity_res,
                        refined_turns=refined_turns,
                    )
                )
                continue

        puzzle_path, exam_path = write_studio_puzzle(
            doc,
            out_puzzles_dir,
            search_exams_dir=search_exams_dir,
            require_target_line=True,
            validate_with_engine=validate_with_engine,
        )
        written += 1
        outcomes.append(
            SynthesizedPuzzleOutcome(
                blueprint_index=idx,
                puzzle_id=doc["id"],
                category=category,
                accepted=True,
                puzzle_path=puzzle_path,
                search_exam_path=exam_path,
                score_target_result=score_target_res,
                score_avoid_results=score_avoid_list,
                verify_result=verify_res,
                plasticity_result=plasticity_res,
                refined_turns=refined_turns,
            )
        )

    return SynthesizeArchetypeReport(
        archetype_name=ctx.archetype_name,
        deck_id=ctx.deck_id,
        blueprints_proposed=len(raw_blueprints),
        puzzles_written=written,
        starved_embeddings=starved_report,
        outcomes=outcomes,
    )
