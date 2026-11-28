"""Workflow 1: `mine_replays` — Replay & Card-Eval Mining into Verified Studio Puzzles.

Three-stage pipeline:
1. `find_candidate_moments` (Zero-LLM Python pre-filter): scans native
   `GameRecorder` JSONs + optional `.winprob.json` sidecars + `CardEvalCollector`
   reports to surface:
   - Non-RNG value drops (`<= -0.25`) from the acting player's perspective.
   - Policy-vs-search top-action disagreements.
   - Stranded `[Main]` / `DNA` / `<Delay>` cards with high exposure and 0 choices.
2. `Role.SCANNER` pass: sends compact board state + legal decoded actions +
   `inspect_position` summary + printed card texts to filter false positives.
3. `Role.AUTHOR` bounded MCP tool loop: minimizes board clutter, checks lines via
   `inspect_stack` / `inspect_position` / `score_line` / `position_saliency` /
   `verify_puzzle` / `plasticity_probe` / `save_puzzle`, and emits ONLY
   engine-verified `studio-puzzle-v1` files under `qa/puzzles/`.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Mapping, Optional, Sequence

from . import (
    CardTextCatalog,
    PuzzleValidationError,
    evaluate_verify_puzzle_result,
    normalize_studio_puzzle_doc,
    resolve_repo_root,
    slugify_puzzle_id,
    validate_studio_puzzle_doc,
    write_studio_puzzle,
)
from .llm_router import ChatMessage, LLMRouter, Role, ToolCall, ToolSpec
from .mcp_client import StdioMcpClient

TRIVIAL_PHASES = frozenset({"Mulligan", "Draw", "Start", "End"})
STRANDED_KEYWORDS = ("[main]", "dna digivolve", "<delay>", "delay")


@dataclass
class CandidateMoment:
    """A high-signal replay decision point surfaced by the zero-LLM pre-filter."""

    recording_path: Path
    replay_step_index: int  # 0-based index into non-Mulligan replayable actions
    recorded_step: int
    turn: int
    phase: str
    seat: int  # 1 or 2 (player to move)
    played_action_id: int
    kind: str  # "value_drop" | "policy_search_disagreement" | "stranded_card"
    severity: float
    reason: str
    value_before: Optional[float] = None
    value_after: Optional[float] = None
    value_delta: Optional[float] = None
    played_prior: Optional[float] = None
    policy_top_action_id: Optional[int] = None
    search_top_action_id: Optional[int] = None
    focus_card_ids: list[str] = field(default_factory=list)
    scenario: dict[str, Any] = field(default_factory=dict)
    legal_actions: list[dict[str, Any]] = field(default_factory=list)


@dataclass
class ScannerVerdict:
    keep: bool
    category: str
    reason: str
    focus_cards: list[str] = field(default_factory=list)
    candidate_target_actions: list[int] = field(default_factory=list)
    candidate_avoid_actions: list[int] = field(default_factory=list)


@dataclass
class MinedPuzzleOutcome:
    candidate: CandidateMoment
    scanner_verdict: Optional[ScannerVerdict] = None
    accepted: bool = False
    rejection_reason: str = ""
    puzzle_id: Optional[str] = None
    puzzle_path: Optional[Path] = None
    search_exam_path: Optional[Path] = None
    verify_result: Optional[dict[str, Any]] = None
    plasticity_result: Optional[dict[str, Any]] = None
    tool_turns_used: int = 0


@dataclass
class MineReplaysReport:
    candidates_found: int
    scanner_kept: int
    puzzles_written: int
    outcomes: list[MinedPuzzleOutcome] = field(default_factory=list)


SCANNER_TRIAGE_SCHEMA: dict[str, Any] = {
    "type": "object",
    "properties": {
        "keep": {"type": "boolean"},
        "category": {"type": "string"},
        "reason": {"type": "string"},
        "focus_cards": {"type": "array", "items": {"type": "string"}},
        "candidate_target_actions": {"type": "array", "items": {"type": "integer"}},
        "candidate_avoid_actions": {"type": "array", "items": {"type": "integer"}},
    },
    "required": [
        "keep",
        "category",
        "reason",
        "focus_cards",
        "candidate_target_actions",
        "candidate_avoid_actions",
    ],
    "additionalProperties": False,
}

AUTHOR_PUZZLE_SCHEMA: dict[str, Any] = {
    "type": "object",
    "properties": {
        "discard": {"type": "boolean"},
        "discard_reason": {"type": "string"},
        "puzzle": {"type": "object"},
    },
    "required": ["discard", "puzzle"],
    "additionalProperties": False,
}


def build_author_tool_specs(*, include_training_tools: bool = False) -> list[ToolSpec]:
    """Bounded MCP tool declarations exposed to `Role.AUTHOR`."""
    specs = [
        ToolSpec(
            name="load_puzzle",
            description="Stage a candidate studio-puzzle-v1 object into digimon-engine-mcp and return game_id.",
            parameters={
                "type": "object",
                "properties": {
                    "puzzle_json": {"type": "object"},
                    "mode": {"type": "string", "enum": ["author", "play"]},
                },
            },
        ),
        ToolSpec(
            name="inspect_stack",
            description="Inspect top card, inherited sources, linked cards, DP, and modifiers on a battle/breeding slot.",
            parameters={
                "type": "object",
                "properties": {
                    "game_id": {"type": "string"},
                    "player": {"type": "integer"},
                    "zone": {"type": "string", "enum": ["battle", "breeding"]},
                    "slot": {"type": "integer"},
                },
                "required": ["game_id", "player"],
            },
        ),
        ToolSpec(
            name="inspect_position",
            description="Evaluate policy priors, value, and determinized MCTS plans on a loaded game_id.",
            parameters={
                "type": "object",
                "properties": {
                    "game_id": {"type": "string"},
                    "model_id": {"type": "string"},
                    "simulations": {"type": "integer"},
                    "worlds": {"type": "integer"},
                    "top_k": {"type": "integer"},
                },
                "required": ["game_id"],
            },
        ),
        ToolSpec(
            name="score_line",
            description="Roll forward an action sequence on game_id (with optional adversary_sims and refutation_triggers) and report per-step value + final_winner.",
            parameters={
                "type": "object",
                "properties": {
                    "game_id": {"type": "string"},
                    "model_id": {"type": "string"},
                    "actions": {"type": "array", "items": {"type": "integer"}},
                    "adversary_sims": {"type": "integer"},
                    "refutation_triggers": {"type": "array", "items": {"type": "object"}},
                },
                "required": ["game_id", "actions"],
            },
        ),
        ToolSpec(
            name="verify_puzzle",
            description="Deterministically verify that target lines solve and avoid lines fail/get refuted across seeds.",
            parameters={
                "type": "object",
                "properties": {
                    "puzzle_json": {"type": "object"},
                    "model_id": {"type": "string"},
                    "adversary_sims": {"type": "integer"},
                    "seeds": {"type": "integer"},
                },
                "required": ["puzzle_json"],
            },
        ),
        ToolSpec(
            name="save_puzzle",
            description="Submit finalized studio-puzzle-v1 metadata, minimized board setup, target/avoid lines, expectations, and rl_goal.",
            parameters={
                "type": "object",
                "properties": {
                    "game_id": {"type": "string"},
                    "id": {"type": "string"},
                    "title": {"type": "string"},
                    "description": {"type": "string"},
                    "folder": {"type": "string"},
                    "tags": {"type": "array", "items": {"type": "string"}},
                    "seat": {"type": "integer"},
                    "setup": {"type": "object"},
                    "decks": {"type": "object"},
                    "lines": {"type": "array", "items": {"type": "object"}},
                    "expectations": {"type": "array", "items": {"type": "object"}},
                    "assertions": {"type": "object"},
                    "rl_goal": {"type": "object"},
                    "refutation_triggers": {"type": "array", "items": {"type": "object"}},
                },
                "required": ["id", "title", "lines"],
            },
        ),
    ]
    if include_training_tools:
        specs.extend(
            [
                ToolSpec(
                    name="position_saliency",
                    description="Compute input-gradient feature attribution on the checkpoint for target vs avoid action.",
                    parameters={
                        "type": "object",
                        "properties": {
                            "scenario_json": {"type": "object"},
                            "target_action_id": {"type": "integer"},
                            "avoid_action_id": {"type": "integer"},
                            "top_k": {"type": "integer"},
                        },
                    },
                ),
                ToolSpec(
                    name="plasticity_probe",
                    description="Run a few local gradient steps on a cloned checkpoint to verify the position is learnable (not tensor-blind).",
                    parameters={
                        "type": "object",
                        "properties": {
                            "scenario_json": {"type": "object"},
                            "target_action_id": {"type": "integer"},
                            "avoid_action_ids": {"type": "array", "items": {"type": "integer"}},
                            "steps": {"type": "integer"},
                        },
                        "required": ["target_action_id"],
                    },
                ),
            ]
        )
    return specs


def _discover_json_files(paths: Sequence[Path | str], *, exclude_winprob: bool = False) -> list[Path]:
    out: list[Path] = []
    for raw in paths:
        p = Path(raw)
        if not p.exists():
            continue
        if p.is_file():
            if exclude_winprob and p.name.endswith(".winprob.json"):
                continue
            out.append(p)
        elif p.is_dir():
            for child in sorted(p.rglob("*.json")):
                if exclude_winprob and child.name.endswith(".winprob.json"):
                    continue
                if child.name == "_library.json":
                    continue
                out.append(child)
    return out


def _unwrap_recording(doc: Mapping[str, Any]) -> Optional[dict[str, Any]]:
    if isinstance(doc.get("actions"), list):
        return dict(doc)
    inner = doc.get("recording")
    if isinstance(inner, Mapping) and isinstance(inner.get("actions"), list):
        return dict(inner)
    return None


def _card_has_tactical_cue(card_id: str, catalog: CardTextCatalog) -> bool:
    info = catalog.get(card_id)
    if info is None:
        return True
    blob = " ".join(
        [info.effect_text, info.inherited_text, info.link_text, info.xros_req]
    ).lower()
    return any(kw in blob for kw in STRANDED_KEYWORDS)


def _extract_stranded_cards_from_eval_reports(
    eval_paths: Sequence[Path | str],
    *,
    min_opportunities: int,
    catalog: CardTextCatalog,
) -> dict[str, dict[str, Any]]:
    """Return `{card_id: {opportunities, route, reason}}` from `CardEvalCollector` JSONs."""
    agg_opps: dict[tuple[str, str], int] = {}
    agg_choices: dict[tuple[str, str], int] = {}
    hand_seen: dict[str, int] = {}
    hand_entered: dict[str, int] = {}

    for p in _discover_json_files(eval_paths):
        try:
            doc = json.loads(p.read_text(encoding="utf-8"))
        except Exception:
            continue
        if not isinstance(doc, Mapping):
            continue

        for r in doc.get("routes") or []:
            if not isinstance(r, Mapping):
                continue
            cid = str(r.get("card_id") or "")
            route = str(r.get("route") or "play")
            if not cid:
                continue
            key = (cid, route)
            agg_opps[key] = agg_opps.get(key, 0) + int(r.get("opportunities", 0))
            agg_choices[key] = agg_choices.get(key, 0) + int(r.get("choices", 0))

        for ef in doc.get("effect_sequences") or []:
            if not isinstance(ef, Mapping):
                continue
            cid = str(ef.get("card_id") or "")
            if not cid:
                continue
            key = (cid, "effect_sequence")
            agg_opps[key] = agg_opps.get(key, 0) + int(ef.get("opportunities", 0))
            completed = int(ef.get("dna_completed", 0)) + int(ef.get("target_accepted", 0))
            agg_choices[key] = agg_choices.get(key, 0) + completed

        for c in doc.get("cards") or []:
            if not isinstance(c, Mapping):
                continue
            cid = str(c.get("card_id") or "")
            if not cid:
                continue
            hand_seen[cid] = hand_seen.get(cid, 0) + int(c.get("games_seen_in_hand", 0))
            hand_entered[cid] = hand_entered.get(cid, 0) + int(c.get("games_entered", 0))

    stranded: dict[str, dict[str, Any]] = {}
    for (cid, route), opps in agg_opps.items():
        choices = agg_choices.get((cid, route), 0)
        if opps >= min_opportunities and choices == 0:
            if route in ("dna_digivolve", "effect_sequence") or _card_has_tactical_cue(cid, catalog):
                prev = stranded.get(cid)
                if prev is None or opps > prev["opportunities"]:
                    stranded[cid] = {
                        "card_id": cid,
                        "route": route,
                        "opportunities": opps,
                        "reason": f"Card {cid} had {opps} legal '{route}' exposures and 0 selections",
                    }

    for cid, seen in hand_seen.items():
        if cid in stranded:
            continue
        if seen >= min_opportunities and hand_entered.get(cid, 0) == 0 and _card_has_tactical_cue(cid, catalog):
            stranded[cid] = {
                "card_id": cid,
                "route": "hand_stranded",
                "opportunities": seen,
                "reason": f"Tactical card {cid} seen in hand in {seen} games with 0 board entries",
            }

    return stranded


def _snapshot_at_replay_index(
    recording: Mapping[str, Any],
    replay_index: int,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    """Reconstruct `(scenario_dict, legal_decoded_actions)` at `replay_index` before action execution."""
    # Allow lightweight embedded snapshots in synthetic unit-test recordings first
    replayable = [a for a in (recording.get("actions") or []) if a.get("phase") != "Mulligan"]
    if 0 <= replay_index < len(replayable):
        act = replayable[replay_index]
        if isinstance(act.get("snapshot"), Mapping):
            snap = dict(act["snapshot"])
            legal = list(act.get("legal_actions") or [])
            return snap, legal

    try:
        import digimon_engine as de

        raw_json = json.dumps(recording)
        rs = de.RustReplaySession.from_native_json(raw_json)
        rs.seek(int(replay_index))
        scen = dict(rs.to_scenario())
        decks = scen.get("decks") or {}
        d1 = list(decks.get("1") or [])
        d2 = list(decks.get("2") or [])
        legal: list[dict[str, Any]] = []
        if d1 and d2:
            hg = de.RustHeadlessGame(d1, d2, seed=int(scen.get("seed", 0)))
            hg.skip_mulligan()
            hg.apply_scenario(json.dumps({"state": scen["state"], "zones": scen["zones"]}))
            legal = list(hg.legal_decoded_actions())
        return scen, legal
    except Exception:
        # Fallback minimal scenario constructed from `initial_state` if replay session cannot reconstruct
        init = recording.get("initial_state") or {}
        p1 = init.get("player1") or {}
        p2 = init.get("player2") or {}
        act = replayable[replay_index] if 0 <= replay_index < len(replayable) else {}
        scen = {
            "decks": {
                "1": list(p1.get("deck_list") or p1.get("deck") or []),
                "2": list(p2.get("deck_list") or p2.get("deck") or []),
            },
            "seed": int(init.get("seed", 0)),
            "state": {
                "memory": int(act.get("memory_before", 0)),
                "phase": str(act.get("phase") or "Main"),
                "turn": max(1, int(act.get("turn", 1))),
                "first_player": int(init.get("first_player_id", 1)),
            },
            "zones": {
                "1": dict(p1.get("zones") or {}),
                "2": dict(p2.get("zones") or {}),
            },
        }
        return scen, list(act.get("legal_actions") or [])


def _visible_cards_in_scenario(scenario: Mapping[str, Any]) -> list[str]:
    cards: list[str] = []
    zones = scenario.get("zones") or {}
    for pk in ("1", "2"):
        z = zones.get(pk) or {}
        for k in ("hand", "trash"):
            cards.extend(str(c) for c in (z.get(k) or []) if isinstance(c, str))
        b = z.get("breeding")
        if isinstance(b, list):
            cards.extend(str(c) for c in b if isinstance(c, str))
        for perm in z.get("field") or []:
            if isinstance(perm, Mapping):
                cards.extend(str(c) for c in (perm.get("stack") or []) if isinstance(c, str))
                for lc in perm.get("linked_cards") or []:
                    if isinstance(lc, str):
                        cards.append(lc)
                    elif isinstance(lc, Mapping) and lc.get("card_id"):
                        cards.append(str(lc["card_id"]))
    return cards


def find_candidate_moments(
    recording_paths: Sequence[Path | str],
    *,
    winprob_paths: Optional[Sequence[Path | str]] = None,
    card_eval_paths: Optional[Sequence[Path | str]] = None,
    value_drop_threshold: float = -0.25,
    min_stranded_opportunities: int = 3,
    max_candidates: int = 25,
    card_catalog: Optional[CardTextCatalog] = None,
) -> list[CandidateMoment]:
    """Zero-LLM pre-filter surfacing tactical candidate moments from replays & telemetry."""
    catalog = card_catalog or CardTextCatalog()
    rec_files = _discover_json_files(recording_paths, exclude_winprob=True)

    explicit_winprobs: dict[str, dict[str, Any]] = {}
    for wp in _discover_json_files(winprob_paths or []):
        try:
            wdoc = json.loads(wp.read_text(encoding="utf-8"))
            if isinstance(wdoc, Mapping):
                stem = wp.name.removesuffix(".winprob.json").removesuffix(".json")
                explicit_winprobs[stem] = dict(wdoc)
                if wdoc.get("recording"):
                    explicit_winprobs[Path(str(wdoc["recording"])).stem] = dict(wdoc)
        except Exception:
            continue

    stranded_cards = _extract_stranded_cards_from_eval_reports(
        card_eval_paths or [],
        min_opportunities=min_stranded_opportunities,
        catalog=catalog,
    )

    moments: list[CandidateMoment] = []
    seen_keys: set[tuple[str, int, str]] = set()

    for rec_path in rec_files:
        try:
            raw_doc = json.loads(rec_path.read_text(encoding="utf-8"))
        except Exception:
            continue
        if not isinstance(raw_doc, Mapping):
            continue
        recording = _unwrap_recording(raw_doc)
        if recording is None:
            continue

        replayable = [
            dict(a)
            for a in (recording.get("actions") or [])
            if isinstance(a, Mapping) and a.get("phase") != "Mulligan"
        ]
        if not replayable:
            continue

        # Locate matching winprob sidecar if available.
        sidecar = explicit_winprobs.get(rec_path.stem)
        if sidecar is None:
            sibling = rec_path.with_name(f"{rec_path.stem}.winprob.json")
            if sibling.exists():
                try:
                    loaded = json.loads(sibling.read_text(encoding="utf-8"))
                    if isinstance(loaded, Mapping):
                        sidecar = dict(loaded)
                except Exception:
                    sidecar = None

        decisions = list((sidecar or {}).get("decisions") or [])

        for idx, act in enumerate(replayable):
            dec = dict(decisions[idx]) if idx < len(decisions) and isinstance(decisions[idx], Mapping) else {}
            merged = {**act, **dec}
            phase = str(merged.get("phase") or "Main")
            seat = int(merged.get("player_id", 1))
            action_id = int(merged.get("action_id", 0))
            step_num = int(merged.get("step", idx + 1))
            turn_num = int(merged.get("turn", 1))

            if phase in TRIVIAL_PHASES:
                continue
            if merged.get("mask_empty") or merged.get("is_rng") or merged.get("rng_only"):
                continue
            played_prior = merged.get("played_prior")
            if played_prior is not None and float(played_prior) >= 0.999 and merged.get("legal_count", 2) <= 1:
                continue

            # 1. Non-RNG Value Drop check (`delta <= value_drop_threshold`, e.g. -0.25)
            val_before = merged.get("value", merged.get("value_before"))
            val_after_mover: Optional[float] = None
            if "value_after" in merged and merged["value_after"] is not None:
                val_after_mover = float(merged["value_after"])
            elif val_before is not None and (idx + 1) < len(decisions):
                next_dec = decisions[idx + 1]
                if isinstance(next_dec, Mapping) and next_dec.get("value") is not None:
                    next_val = float(next_dec["value"])
                    next_pid = int(next_dec.get("player_id", seat))
                    val_after_mover = next_val if next_pid == seat else -next_val

            if val_before is not None and val_after_mover is not None:
                v_b = float(val_before)
                delta = val_after_mover - v_b
                # Ignore forced single-move steps indicated by played_prior ~= 1.0 unless explicit legal_actions > 1
                is_forced_single = (
                    played_prior is not None
                    and float(played_prior) >= 0.999
                    and not merged.get("search_top_action_id")
                )
                if not is_forced_single and delta <= value_drop_threshold:
                    key = (str(rec_path), idx, "value_drop")
                    if key not in seen_keys:
                        seen_keys.add(key)
                        scen, legal = _snapshot_at_replay_index(recording, idx)
                        moments.append(
                            CandidateMoment(
                                recording_path=rec_path,
                                replay_step_index=idx,
                                recorded_step=step_num,
                                turn=turn_num,
                                phase=phase,
                                seat=seat,
                                played_action_id=action_id,
                                kind="value_drop",
                                severity=abs(delta),
                                reason=(
                                    f"Mover P{seat} value dropped by {delta:+.3f} "
                                    f"({v_b:+.3f} -> {val_after_mover:+.3f}) on action {action_id} in {phase}"
                                ),
                                value_before=v_b,
                                value_after=val_after_mover,
                                value_delta=delta,
                                played_prior=float(played_prior) if played_prior is not None else None,
                                scenario=scen,
                                legal_actions=legal,
                            )
                        )

            # 2. Policy-vs-Search top-action disagreement check
            pol_top = merged.get("policy_top_action_id")
            srch_top = merged.get("search_top_action_id", merged.get("teacher_action_id"))
            if pol_top is not None and srch_top is not None and int(pol_top) != int(srch_top):
                key = (str(rec_path), idx, "policy_search_disagreement")
                if key not in seen_keys:
                    seen_keys.add(key)
                    scen, legal = _snapshot_at_replay_index(recording, idx)
                    gap = abs(float(merged.get("search_value", 0.5)) - float(val_before or 0.0))
                    moments.append(
                        CandidateMoment(
                            recording_path=rec_path,
                            replay_step_index=idx,
                            recorded_step=step_num,
                            turn=turn_num,
                            phase=phase,
                            seat=seat,
                            played_action_id=action_id,
                            kind="policy_search_disagreement",
                            severity=max(0.35, gap),
                            reason=(
                                f"Policy top action ({int(pol_top)}) disagreed with search top action "
                                f"({int(srch_top)}) at step {step_num}"
                            ),
                            value_before=float(val_before) if val_before is not None else None,
                            played_prior=float(played_prior) if played_prior is not None else None,
                            policy_top_action_id=int(pol_top),
                            search_top_action_id=int(srch_top),
                            scenario=scen,
                            legal_actions=legal,
                        )
                    )

            # 3. Stranded card match on Main / EndOfTurnAction steps
            if stranded_cards and phase in ("Main", "EndOfTurnAction"):
                scen, legal = _snapshot_at_replay_index(recording, idx)
                mover_zone = (scen.get("zones") or {}).get(str(seat)) or {}
                mover_cards = set(str(c) for c in (mover_zone.get("hand") or []))
                for perm in mover_zone.get("field") or []:
                    if isinstance(perm, Mapping):
                        mover_cards.update(str(c) for c in (perm.get("stack") or []))
                legal_cids = {
                    str(la.get("card_id"))
                    for la in legal
                    if isinstance(la, Mapping) and la.get("card_id")
                }
                candidate_pool = mover_cards | legal_cids
                matched = [cid for cid in stranded_cards if cid in candidate_pool]
                # Check that the played action did NOT already use the stranded card
                played_card = next(
                    (
                        str(la.get("card_id"))
                        for la in legal
                        if isinstance(la, Mapping) and int(la.get("action_id", -1)) == action_id and la.get("card_id")
                    ),
                    None,
                )
                matched = [cid for cid in matched if cid != played_card]
                if matched:
                    primary = matched[0]
                    # Emit at most one stranded_card moment per (recording, card_id)
                    skey = (str(rec_path), -hash(primary), "stranded_card")
                    if skey not in seen_keys:
                        seen_keys.add(skey)
                        meta = stranded_cards[primary]
                        moments.append(
                            CandidateMoment(
                                recording_path=rec_path,
                                replay_step_index=idx,
                                recorded_step=step_num,
                                turn=turn_num,
                                phase=phase,
                                seat=seat,
                                played_action_id=action_id,
                                kind="stranded_card",
                                severity=0.30 + min(0.50, meta["opportunities"] * 0.02),
                                reason=meta["reason"],
                                focus_card_ids=matched,
                                scenario=scen,
                                legal_actions=legal,
                            )
                        )

    moments.sort(key=lambda m: (-m.severity, str(m.recording_path), m.replay_step_index))
    return moments[: max(0, int(max_candidates))]


def triage_candidate_moment(
    moment: CandidateMoment,
    *,
    router: LLMRouter,
    engine_mcp: Optional[StdioMcpClient] = None,
    model_id: Optional[str] = None,
    card_catalog: Optional[CardTextCatalog] = None,
) -> ScannerVerdict:
    """Run `Role.SCANNER` to filter false positives using board state + `inspect_position` + card text."""
    catalog = card_catalog or CardTextCatalog()

    inspect_summary: dict[str, Any] = {}
    if engine_mcp is not None and engine_mcp.has_tool("load_puzzle") and engine_mcp.has_tool("inspect_position"):
        try:
            temp_puzzle = normalize_studio_puzzle_doc(
                {
                    "id": "scanner-probe",
                    "title": "Scanner Probe",
                    "seat": moment.seat,
                    "decks": moment.scenario.get("decks") or {},
                    "seed": int(moment.scenario.get("seed", 0)),
                    "setup": {
                        "kind": "staged",
                        "state": moment.scenario.get("state") or {},
                        "zones": moment.scenario.get("zones") or {"1": {}, "2": {}},
                    },
                },
                default_source_kind="recording",
                card_catalog=catalog,
            )
            loaded = engine_mcp.call_tool("load_puzzle", {"puzzle_json": temp_puzzle, "mode": "author"})
            gid = loaded.get("game_id")
            if gid:
                insp_args: dict[str, Any] = {"game_id": gid, "top_k": 8}
                if model_id:
                    insp_args["model_id"] = model_id
                inspect_summary = engine_mcp.call_tool("inspect_position", insp_args)
                if engine_mcp.has_tool("close_game"):
                    engine_mcp.call_tool("close_game", {"game_id": gid})
        except Exception:
            inspect_summary = {}

    visible_cids = list(moment.focus_card_ids) + _visible_cards_in_scenario(moment.scenario)
    for la in moment.legal_actions:
        if isinstance(la, Mapping) and la.get("card_id"):
            visible_cids.append(str(la["card_id"]))
    printed_cards = catalog.describe_cards(visible_cids)

    scanner_payload = {
        "signal_kind": moment.kind,
        "signal_reason": moment.reason,
        "recording": moment.recording_path.name,
        "turn": moment.turn,
        "phase": moment.phase,
        "seat": moment.seat,
        "played_action_id": moment.played_action_id,
        "value_before": moment.value_before,
        "value_after": moment.value_after,
        "value_delta": moment.value_delta,
        "policy_top_action_id": moment.policy_top_action_id,
        "search_top_action_id": moment.search_top_action_id,
        "board_state": {
            "state": moment.scenario.get("state"),
            "zones": {
                pk: {
                    "hand": (moment.scenario.get("zones", {}).get(pk) or {}).get("hand", []),
                    "field": (moment.scenario.get("zones", {}).get(pk) or {}).get("field", []),
                    "breeding": (moment.scenario.get("zones", {}).get(pk) or {}).get("breeding"),
                    "security_count": len(
                        (moment.scenario.get("zones", {}).get(pk) or {}).get("security", [])
                    ),
                    "trash": (moment.scenario.get("zones", {}).get(pk) or {}).get("trash", []),
                }
                for pk in ("1", "2")
            },
        },
        "legal_actions": moment.legal_actions[:24],
        "inspect_position": inspect_summary,
        "printed_cards": printed_cards,
    }

    messages = [
        ChatMessage(
            role="system",
            content=(
                "You are Role.SCANNER for the Digimon TCG Tactical Puzzle Lab. "
                "Decide whether this candidate replay moment represents a genuine tactical puzzle "
                "(e.g. missed DNA/Delay/Training lethal, sequencing error, Blocker/All-Turns trap, "
                "or stranded combo card) versus pure hidden-card RNG or a dead lost board. "
                "Return strict JSON matching the requested schema."
            ),
        ),
        ChatMessage(role="user", content=json.dumps(scanner_payload, indent=2)),
    ]

    parsed = router.complete_structured(
        Role.SCANNER,
        messages,
        json_schema=SCANNER_TRIAGE_SCHEMA,
        schema_name="scanner_verdict",
    )
    return ScannerVerdict(
        keep=bool(parsed.get("keep", False)),
        category=str(parsed.get("category") or moment.kind),
        reason=str(parsed.get("reason") or ""),
        focus_cards=[str(c) for c in (parsed.get("focus_cards") or [])],
        candidate_target_actions=[int(a) for a in (parsed.get("candidate_target_actions") or [])],
        candidate_avoid_actions=[int(a) for a in (parsed.get("candidate_avoid_actions") or [])],
    )


def author_puzzle_from_moment(
    moment: CandidateMoment,
    verdict: ScannerVerdict,
    *,
    router: LLMRouter,
    engine_mcp: StdioMcpClient,
    training_mcp: Optional[StdioMcpClient] = None,
    checkpoint_path: Optional[Path] = None,
    model_id: Optional[str] = None,
    puzzles_dir: Optional[Path] = None,
    search_exams_dir: Optional[Path] = None,
    folder: str = "",
    max_tool_turns: int = 10,
    adversary_sims: int = 64,
    verify_seeds: int = 4,
    require_plasticity: bool = False,
    validate_with_engine: bool = False,
    card_catalog: Optional[CardTextCatalog] = None,
) -> MinedPuzzleOutcome:
    """Run `Role.AUTHOR` bounded MCP tool loop and enforce engine verification before saving."""
    catalog = card_catalog or CardTextCatalog()
    root = resolve_repo_root()
    out_puzzles_dir = puzzles_dir or (root / "qa" / "puzzles")

    has_training = training_mcp is not None and checkpoint_path is not None
    tools = build_author_tool_specs(include_training_tools=has_training)

    visible_cids = list(
        dict.fromkeys(
            list(verdict.focus_cards)
            + list(moment.focus_card_ids)
            + _visible_cards_in_scenario(moment.scenario)
            + [
                str(la["card_id"])
                for la in moment.legal_actions
                if isinstance(la, Mapping) and la.get("card_id")
            ]
        )
    )
    printed_cards = catalog.describe_cards(visible_cids)

    saved_draft: dict[str, Any] = {}
    last_staged_puzzle: dict[str, Any] = {}
    last_plasticity: Optional[dict[str, Any]] = None

    def _execute_tool(tc: ToolCall) -> dict[str, Any]:
        nonlocal saved_draft, last_staged_puzzle, last_plasticity
        args = dict(tc.arguments)

        if tc.name == "load_puzzle":
            pjson = args.get("puzzle_json")
            if isinstance(pjson, Mapping):
                last_staged_puzzle = dict(pjson)
            return engine_mcp.call_tool("load_puzzle", args)

        if tc.name in ("inspect_stack", "inspect_position", "score_line", "verify_puzzle"):
            if tc.name in ("inspect_position", "score_line", "verify_puzzle") and model_id and "model_id" not in args:
                args["model_id"] = model_id
            return engine_mcp.call_tool(tc.name, args)

        if tc.name == "save_puzzle":
            saved_draft = dict(args)
            return {"ok": True, "staged_for_final_verification": True, "id": args.get("id")}

        if tc.name in ("position_saliency", "plasticity_probe"):
            if training_mcp is None or checkpoint_path is None:
                return {"ok": False, "error": "training_mcp / checkpoint_path not configured"}
            args.setdefault("checkpoint_path", str(checkpoint_path))
            if "scenario_json" not in args:
                base_setup = (last_staged_puzzle.get("setup") or moment.scenario)
                args["scenario_json"] = {
                    "state": base_setup.get("state", moment.scenario.get("state")),
                    "zones": base_setup.get("zones", moment.scenario.get("zones")),
                }
            res = training_mcp.call_tool(tc.name, args)
            if tc.name == "plasticity_probe":
                last_plasticity = res
            return res

        return {"ok": False, "error": f"Unsupported tool {tc.name!r}"}

    default_id = slugify_puzzle_id(
        f"{moment.recording_path.stem}-t{moment.turn}-s{moment.recorded_step}-{verdict.category}"
    )
    author_prompt = {
        "default_id": default_id,
        "folder": folder,
        "scanner_verdict": {
            "category": verdict.category,
            "reason": verdict.reason,
            "focus_cards": verdict.focus_cards,
            "candidate_target_actions": verdict.candidate_target_actions,
            "candidate_avoid_actions": verdict.candidate_avoid_actions,
        },
        "replay_moment": {
            "recording": moment.recording_path.name,
            "step": moment.recorded_step,
            "turn": moment.turn,
            "phase": moment.phase,
            "seat": moment.seat,
            "played_action_id": moment.played_action_id,
            "scenario": moment.scenario,
            "legal_actions": moment.legal_actions,
        },
        "printed_cards": printed_cards,
    }

    messages = [
        ChatMessage(
            role="system",
            content=(
                "You are Role.AUTHOR for the Digimon TCG Tactical Puzzle Lab. "
                "Given a triaged replay moment and printed card texts:\n"
                "1. Minimize irrelevant clutter in hand/trash/field while keeping exact tactical legality.\n"
                "2. Use `load_puzzle`, `inspect_stack`, `inspect_position`, and `score_line` to test "
                "`target` (winning/optimal) and `avoid` (played blunder / trap) lines.\n"
                "3. Call `verify_puzzle` (and `plasticity_probe` when available) to confirm `target` solves "
                "and `avoid` fails.\n"
                "4. Submit the puzzle via `save_puzzle` or final JSON (`{\"discard\": false, \"puzzle\": {...}}`). "
                "If the position is unfixable RNG, return `{\"discard\": true, \"discard_reason\": \"...\", \"puzzle\": {}}`."
            ),
        ),
        ChatMessage(role="user", content=json.dumps(author_prompt, indent=2)),
    ]

    loop_res = router.run_tool_loop(
        Role.AUTHOR,
        messages,
        tools=tools,
        tool_executor=_execute_tool,
        max_turns=max_tool_turns,
        final_json_schema=None if saved_draft else AUTHOR_PUZZLE_SCHEMA,
        schema_name="author_puzzle_output",
        stop_condition=lambda tc, out: tc.name == "save_puzzle" and bool(out.get("ok")),
    )

    raw_candidate: dict[str, Any] = {}
    if saved_draft:
        # Merge base scenario / staged setup if `save_puzzle` omitted full `setup`/`decks`.
        merged = dict(last_staged_puzzle)
        merged.update(saved_draft)
        if "setup" not in merged:
            merged["setup"] = {
                "kind": "staged",
                "state": moment.scenario.get("state") or {},
                "zones": moment.scenario.get("zones") or {"1": {}, "2": {}},
                "prefix_actions": [],
            }
        if "decks" not in merged:
            merged["decks"] = moment.scenario.get("decks") or {}
        merged.setdefault("seat", moment.seat)
        raw_candidate = merged
    else:
        parsed_final = loop_res.final_response.parsed_json or {}
        if parsed_final.get("discard"):
            return MinedPuzzleOutcome(
                candidate=moment,
                scanner_verdict=verdict,
                accepted=False,
                rejection_reason=str(parsed_final.get("discard_reason") or "author discarded moment"),
                tool_turns_used=loop_res.turns_used,
            )
        if isinstance(parsed_final.get("puzzle"), Mapping) and parsed_final["puzzle"]:
            raw_candidate = dict(parsed_final["puzzle"])
        elif parsed_final.get("schema") == "studio-puzzle-v1" or parsed_final.get("lines"):
            raw_candidate = dict(parsed_final)

    if not raw_candidate:
        return MinedPuzzleOutcome(
            candidate=moment,
            scanner_verdict=verdict,
            accepted=False,
            rejection_reason="Role.AUTHOR produced no candidate puzzle",
            tool_turns_used=loop_res.turns_used,
        )

    raw_candidate.setdefault("id", default_id)
    if folder and not raw_candidate.get("folder"):
        raw_candidate["folder"] = folder
    raw_candidate.setdefault(
        "source",
        {
            "kind": "recording",
            "recording": moment.recording_path.name,
            "step": moment.recorded_step,
            "signal": moment.kind,
        },
    )

    try:
        puzzle_doc = normalize_studio_puzzle_doc(
            raw_candidate,
            default_source_kind="recording",
            card_catalog=catalog,
        )
        validate_studio_puzzle_doc(
            puzzle_doc,
            require_target_line=True,
            validate_with_engine=validate_with_engine,
        )
    except PuzzleValidationError as exc:
        return MinedPuzzleOutcome(
            candidate=moment,
            scanner_verdict=verdict,
            accepted=False,
            rejection_reason=f"Schema validation failed: {exc}",
            tool_turns_used=loop_res.turns_used,
        )

    # Mandatory engine verification gate via `digimon-engine-mcp` `verify_puzzle`.
    verify_args: dict[str, Any] = {
        "puzzle_json": puzzle_doc,
        "adversary_sims": adversary_sims,
        "seeds": verify_seeds,
    }
    if model_id:
        verify_args["model_id"] = model_id
    verify_res = engine_mcp.call_tool("verify_puzzle", verify_args)
    passed, reason = evaluate_verify_puzzle_result(verify_res, solver_seat=int(puzzle_doc["seat"]))
    if not passed:
        return MinedPuzzleOutcome(
            candidate=moment,
            scanner_verdict=verdict,
            accepted=False,
            rejection_reason=f"verify_puzzle rejected puzzle: {reason}",
            puzzle_id=puzzle_doc["id"],
            verify_result=verify_res,
            plasticity_result=last_plasticity,
            tool_turns_used=loop_res.turns_used,
        )

    # Optional plasticity learnability gate via `digimon-training-mcp`.
    if has_training and training_mcp is not None and checkpoint_path is not None:
        if last_plasticity is None and training_mcp.has_tool("plasticity_probe"):
            target_root = puzzle_doc["lines"][0]["actions"][0]
            avoid_roots = [
                ln["actions"][0]
                for ln in puzzle_doc["lines"]
                if ln["role"] == "avoid" and ln["actions"]
            ]
            last_plasticity = training_mcp.call_tool(
                "plasticity_probe",
                {
                    "checkpoint_path": str(checkpoint_path),
                    "scenario_json": {
                        "state": puzzle_doc["setup"].get("state"),
                        "zones": puzzle_doc["setup"].get("zones"),
                    },
                    "target_action_id": target_root,
                    "avoid_action_ids": avoid_roots,
                },
            )
        if require_plasticity and last_plasticity is not None and last_plasticity.get("learnable") is False:
            diag = last_plasticity.get("diagnosis") or "not learnable"
            return MinedPuzzleOutcome(
                candidate=moment,
                scanner_verdict=verdict,
                accepted=False,
                rejection_reason=f"plasticity_probe failed: {diag}",
                puzzle_id=puzzle_doc["id"],
                verify_result=verify_res,
                plasticity_result=last_plasticity,
                tool_turns_used=loop_res.turns_used,
            )

    puzzle_path, exam_path = write_studio_puzzle(
        puzzle_doc,
        out_puzzles_dir,
        search_exams_dir=search_exams_dir,
        require_target_line=True,
        validate_with_engine=validate_with_engine,
    )
    return MinedPuzzleOutcome(
        candidate=moment,
        scanner_verdict=verdict,
        accepted=True,
        puzzle_id=puzzle_doc["id"],
        puzzle_path=puzzle_path,
        search_exam_path=exam_path,
        verify_result=verify_res,
        plasticity_result=last_plasticity,
        tool_turns_used=loop_res.turns_used,
    )


def mine_replays(
    recording_paths: Sequence[Path | str],
    *,
    router: LLMRouter,
    engine_mcp: StdioMcpClient,
    training_mcp: Optional[StdioMcpClient] = None,
    winprob_paths: Optional[Sequence[Path | str]] = None,
    card_eval_paths: Optional[Sequence[Path | str]] = None,
    checkpoint_path: Optional[Path] = None,
    model_id: Optional[str] = None,
    puzzles_dir: Optional[Path] = None,
    search_exams_dir: Optional[Path] = None,
    folder: str = "",
    value_drop_threshold: float = -0.25,
    min_stranded_opportunities: int = 3,
    max_candidates: int = 15,
    max_puzzles: int = 5,
    max_tool_turns: int = 10,
    adversary_sims: int = 64,
    verify_seeds: int = 4,
    require_plasticity: bool = False,
    validate_with_engine: bool = False,
    card_catalog: Optional[CardTextCatalog] = None,
) -> MineReplaysReport:
    """End-to-end replay-mining workflow (`find_candidate_moments` -> `Scanner` -> `Author` -> verify)."""
    catalog = card_catalog or CardTextCatalog()
    candidates = find_candidate_moments(
        recording_paths,
        winprob_paths=winprob_paths,
        card_eval_paths=card_eval_paths,
        value_drop_threshold=value_drop_threshold,
        min_stranded_opportunities=min_stranded_opportunities,
        max_candidates=max_candidates,
        card_catalog=catalog,
    )

    outcomes: list[MinedPuzzleOutcome] = []
    scanner_kept = 0
    written = 0

    for moment in candidates:
        if written >= max_puzzles:
            break
        verdict = triage_candidate_moment(
            moment,
            router=router,
            engine_mcp=engine_mcp,
            model_id=model_id,
            card_catalog=catalog,
        )
        if not verdict.keep:
            outcomes.append(
                MinedPuzzleOutcome(
                    candidate=moment,
                    scanner_verdict=verdict,
                    accepted=False,
                    rejection_reason=f"Scanner rejected: {verdict.reason}",
                )
            )
            continue

        scanner_kept += 1
        outcome = author_puzzle_from_moment(
            moment,
            verdict,
            router=router,
            engine_mcp=engine_mcp,
            training_mcp=training_mcp,
            checkpoint_path=checkpoint_path,
            model_id=model_id,
            puzzles_dir=puzzles_dir,
            search_exams_dir=search_exams_dir,
            folder=folder,
            max_tool_turns=max_tool_turns,
            adversary_sims=adversary_sims,
            verify_seeds=verify_seeds,
            require_plasticity=require_plasticity,
            validate_with_engine=validate_with_engine,
            card_catalog=catalog,
        )
        if outcome.accepted:
            written += 1
        outcomes.append(outcome)

    return MineReplaysReport(
        candidates_found=len(candidates),
        scanner_kept=scanner_kept,
        puzzles_written=written,
        outcomes=outcomes,
    )
