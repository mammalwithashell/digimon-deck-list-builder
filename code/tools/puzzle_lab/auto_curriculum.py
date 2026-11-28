"""Air-Gapped Auto-Curriculum Sidecar Daemon (`auto_curriculum`).

Implements §1.3 of `2026-11-28-next-wave-belief-critic-grpo-lab-design.md`:
1. Ingests EITHER an `anchored_eval` JSON report OR a Specialist League
   `gates/gen<g>/*.json` directory (`specialist-gate-v1` / per-matchup win rates)
   plus optional recordings / candidate drills.
2. Selects the bottom-`K` worst-performing matchups (`win_rate < threshold`,
   default `0.45`).
3. Harvests candidate `studio-puzzle-v1` drills via `mine_replays` (on available
   recordings), `compose_boss_matrix` / `synthesize_archetype` (when `router` is
   supplied), and/or pre-staged `candidate_puzzles`.
4. Enforces the **Mandatory Dual Gate** on every candidate puzzle:
   - **Gate 1 (`verify_puzzle` via MCP)**: confirms `target` solves and `avoid`
     is refuted.
   - **Gate 2 (`plasticity_probe` when `checkpoint_zip` or `training_mcp` is supplied)**:
     - `learnable == True`: promoted to `<out_dir>/verified_puzzles/` and
       compiled into `<out_dir>/extra_shards/` (`selfplay-shards-v1` via
       `tools.emit_puzzle_shards.emit_puzzle_shards`) with a SHA256-stamped
       `<out_dir>/curriculum_manifest.json`!
     - `learnable == False`: quarantined to
       `<out_dir>/tensor_blindness_quarantine.json` with full diagnosis so
       unlearnable (`observation_blindness`) states never poison RL training!
"""

from __future__ import annotations

import hashlib
import json
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Mapping, Optional, Sequence

from . import (
    CardTextCatalog,
    PuzzleValidationError,
    evaluate_verify_puzzle_result,
    normalize_studio_puzzle_doc,
    resolve_repo_root,
    validate_studio_puzzle_doc,
    write_studio_puzzle,
)
from .boss_matrix import BUILTIN_BOSS_TEMPLATES, compose_boss_matrix
from .llm_router import LLMRouter
from .mcp_client import StdioMcpClient
from .mine_replays import mine_replays
from .synthesize_archetype import synthesize_archetype

CURRICULUM_MANIFEST_SCHEMA = "auto-curriculum-manifest-v1"
QUARANTINE_SCHEMA = "tensor-blindness-quarantine-v1"


def _sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def _ensure_code_paths() -> None:
    root = resolve_repo_root()
    for p in (root / "code", root / "code" / "digimon-training-mcp" / "src"):
        s = str(p)
        if p.exists() and s not in sys.path:
            sys.path.insert(0, s)


@dataclass
class MatchupWeakness:
    """Normalized per-matchup evaluation summary."""

    archetype: str
    opponent_archetype: str
    win_rate: float
    games: int
    wins: int = 0
    losses: int = 0
    draws: int = 0
    source_path: Optional[str] = None

    @property
    def label(self) -> str:
        return f"{self.archetype} vs {self.opponent_archetype}"

    def to_dict(self) -> dict[str, Any]:
        return {
            "archetype": self.archetype,
            "opponent_archetype": self.opponent_archetype,
            "matchup": self.label,
            "win_rate": float(self.win_rate),
            "games": int(self.games),
            "wins": int(self.wins),
            "losses": int(self.losses),
            "draws": int(self.draws),
            "source_path": self.source_path,
        }


@dataclass
class CandidateGateOutcome:
    """Dual-gate audit record for one candidate puzzle."""

    puzzle_id: str
    title: str
    matchup: str
    verify_passed: bool
    verify_reason: str
    plasticity_checked: bool = False
    learnable: Optional[bool] = None
    plasticity_diagnosis: str = ""
    promoted: bool = False
    quarantined: bool = False
    verified_puzzle_path: Optional[Path] = None
    verify_result: Optional[dict[str, Any]] = None
    plasticity_result: Optional[dict[str, Any]] = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "puzzle_id": self.puzzle_id,
            "title": self.title,
            "matchup": self.matchup,
            "verify_passed": self.verify_passed,
            "verify_reason": self.verify_reason,
            "plasticity_checked": self.plasticity_checked,
            "learnable": self.learnable,
            "plasticity_diagnosis": self.plasticity_diagnosis or None,
            "promoted": self.promoted,
            "quarantined": self.quarantined,
            "verified_puzzle_path": str(self.verified_puzzle_path) if self.verified_puzzle_path else None,
        }


@dataclass
class AutoCurriculumReport:
    """End-to-end summary of an `auto_curriculum` sidecar run."""

    out_dir: Path
    all_matchups: list[MatchupWeakness]
    selected_matchups: list[MatchupWeakness]
    candidates_evaluated: int
    promoted_count: int
    quarantined_count: int
    rejected_verify_count: int
    outcomes: list[CandidateGateOutcome] = field(default_factory=list)
    manifest_path: Optional[Path] = None
    quarantine_path: Optional[Path] = None
    extra_shards_dir: Optional[Path] = None
    bundle_sha256: Optional[str] = None
    manifest_dict: dict[str, Any] = field(default_factory=dict)


def _split_matchup_key(raw_key: str, default_arch: str = "Agent") -> tuple[str, str]:
    for sep in (" vs ", " VS ", " v. ", "::", "->"):
        if sep in raw_key:
            left, right = raw_key.split(sep, 1)
            return left.strip() or default_arch, right.strip() or "Opponent"
    return default_arch, raw_key.strip() or "Opponent"


def _clean_deck_arch_name(raw: str) -> str:
    return str(raw or "").split(":sha256:", 1)[0].strip()


def _record_from_stats_dict(
    arch: str,
    opp: str,
    stats: Mapping[str, Any],
    source_path: str,
) -> Optional[MatchupWeakness]:
    wins = int(stats.get("wins", 0))
    losses = int(stats.get("losses", 0))
    draws = int(stats.get("draws", 0))
    games = int(stats.get("games", wins + losses + draws))
    if "win_rate" in stats and stats["win_rate"] is not None:
        wr = float(stats["win_rate"])
        if games <= 0:
            games = max(1, wins + losses + draws)
    elif "score" in stats and stats["score"] is not None:
        wr = float(stats["score"])
        games = max(1, games)
    elif games > 0:
        wr = wins / float(games)
    else:
        return None

    return MatchupWeakness(
        archetype=arch,
        opponent_archetype=opp,
        win_rate=wr,
        games=games,
        wins=wins,
        losses=losses,
        draws=draws,
        source_path=source_path,
    )


def _extract_gate_challenger_shard_matchups(
    gate_file: Path,
    learner_arch: str,
) -> list[MatchupWeakness]:
    """Extract per-opponent win rates from `gates/gen<g>/<slug>/challenger/**/manifest.json` if present."""
    slug_guess = gate_file.stem
    cand_dir = gate_file.parent / slug_guess / "challenger"
    if not cand_dir.is_dir():
        return []

    agg: dict[str, dict[str, int]] = {}
    for mf in sorted(cand_dir.rglob("manifest.json")):
        try:
            doc = json.loads(mf.read_text(encoding="utf-8"))
        except Exception:
            continue
        if not isinstance(doc, Mapping):
            continue
        names = list(doc.get("deck_names") or [])
        pairs = list(doc.get("game_decks") or [])
        opp_games = list(doc.get("opponent_games") or [])
        for pair, gm in zip(pairs, opp_games):
            if not isinstance(pair, list) or len(pair) < 2 or not isinstance(gm, Mapping):
                continue
            seat = int(gm.get("learner_seat", 0))
            opp_seat = 1 - seat if seat in (0, 1) else 1
            opp_idx = int(pair[opp_seat]) if opp_seat < len(pair) else 0
            opp_name = _clean_deck_arch_name(names[opp_idx]) if 0 <= opp_idx < len(names) else "Opponent"
            bucket = agg.setdefault(opp_name, {"wins": 0, "losses": 0, "draws": 0, "games": 0})
            bucket["games"] += 1
            winner = gm.get("winner")
            if winner is None:
                bucket["draws"] += 1
            elif int(winner) == seat:
                bucket["wins"] += 1
            else:
                bucket["losses"] += 1

    out: list[MatchupWeakness] = []
    for opp_name, b in sorted(agg.items()):
        games = b["games"]
        if games <= 0:
            continue
        wr = (b["wins"] + 0.5 * b["draws"]) / float(games)
        out.append(
            MatchupWeakness(
                archetype=learner_arch,
                opponent_archetype=opp_name,
                win_rate=wr,
                games=games,
                wins=b["wins"],
                losses=b["losses"],
                draws=b["draws"],
                source_path=str(gate_file),
            )
        )
    return out


def _parse_single_eval_json(doc: Mapping[str, Any], src: Path) -> list[MatchupWeakness]:
    src_str = str(src)
    found: list[MatchupWeakness] = []

    # 1. Specialist League `specialist-gate-v1`
    if doc.get("schema") == "specialist-gate-v1":
        arch = str(doc.get("archetype") or src.stem)
        if isinstance(doc.get("by_matchup"), Mapping):
            for k, v in doc["by_matchup"].items():
                if isinstance(v, Mapping):
                    a_name, o_name = _split_matchup_key(str(k), default_arch=arch)
                    rec = _record_from_stats_dict(a_name, o_name, v, src_str)
                    if rec is not None:
                        found.append(rec)
        if isinstance(doc.get("matchups"), list):
            for item in doc["matchups"]:
                if isinstance(item, Mapping):
                    a_name = str(item.get("archetype") or arch)
                    o_name = str(item.get("opponent_archetype") or item.get("opponent") or "Opponent")
                    rec = _record_from_stats_dict(a_name, o_name, item, src_str)
                    if rec is not None:
                        found.append(rec)
        if not found:
            found.extend(_extract_gate_challenger_shard_matchups(src, arch))
        if not found and doc.get("challenger_score") is not None:
            games = int(doc.get("games", 1))
            wr = float(doc["challenger_score"])
            wins = int(round(wr * games))
            found.append(
                MatchupWeakness(
                    archetype=arch,
                    opponent_archetype=str(doc.get("field") or "field"),
                    win_rate=wr,
                    games=games,
                    wins=wins,
                    losses=max(0, games - wins),
                    draws=0,
                    source_path=src_str,
                )
            )
        return found

    # 2. Explicit `matchups` list (`[{"archetype": ..., "opponent_archetype": ..., "win_rate": ...}]`)
    if isinstance(doc.get("matchups"), list):
        default_arch = str(doc.get("archetype") or doc.get("candidate_archetype") or "Agent")
        for item in doc["matchups"]:
            if not isinstance(item, Mapping):
                continue
            if "matchup" in item and "archetype" not in item:
                a_name, o_name = _split_matchup_key(str(item["matchup"]), default_arch=default_arch)
            else:
                a_name = str(item.get("archetype") or item.get("agent_archetype") or default_arch)
                o_name = str(item.get("opponent_archetype") or item.get("opponent") or "Opponent")
            rec = _record_from_stats_dict(a_name, o_name, item, src_str)
            if rec is not None:
                found.append(rec)
        if found:
            return found

    # 3. `by_matchup` map (top-level or nested under anchor panels like `vs_greedy`, `anchored_eval`, `results`)
    def _scan_by_matchup(mapping_obj: Mapping[str, Any], default_arch: str) -> None:
        bm = mapping_obj.get("by_matchup")
        if isinstance(bm, Mapping):
            for m_key, m_stats in bm.items():
                if isinstance(m_stats, Mapping):
                    a_name, o_name = _split_matchup_key(str(m_key), default_arch=default_arch)
                    rec = _record_from_stats_dict(a_name, o_name, m_stats, src_str)
                    if rec is not None:
                        found.append(rec)

    top_arch = str(doc.get("archetype") or doc.get("candidate") or "Agent")
    _scan_by_matchup(doc, top_arch)
    for sub_val in doc.values():
        if isinstance(sub_val, Mapping):
            _scan_by_matchup(sub_val, top_arch)
    if found:
        return found

    # 4. `league_eval` matchup matrix (`{"matrix": {arch: {opp: {wins, losses, draws, win_rate}}}}`)
    matrix_obj = doc.get("matrix") if isinstance(doc.get("matrix"), Mapping) else None
    if matrix_obj is not None:
        for a_name, row in matrix_obj.items():
            if not isinstance(row, Mapping):
                continue
            for o_name, stats in row.items():
                if isinstance(stats, Mapping):
                    rec = _record_from_stats_dict(str(a_name), str(o_name), stats, src_str)
                    if rec is not None:
                        found.append(rec)
        if found:
            return found

    # 5. `CardEvalCollector` game-level log (`{"games": [{"agent_archetype": ..., "opponent_archetype": ..., "winner_id": 1|2}]}`)
    if isinstance(doc.get("games"), list) and doc["games"] and isinstance(doc["games"][0], Mapping):
        agg_g: dict[tuple[str, str], dict[str, int]] = {}
        for gm in doc["games"]:
            if not isinstance(gm, Mapping):
                continue
            a_name = str(gm.get("agent_archetype") or top_arch)
            o_name = str(gm.get("opponent_archetype") or gm.get("anchor") or "Opponent")
            b = agg_g.setdefault((a_name, o_name), {"wins": 0, "losses": 0, "draws": 0, "games": 0})
            b["games"] += 1
            w = gm.get("winner_id", gm.get("winner"))
            if w == 1:
                b["wins"] += 1
            elif w == 2:
                b["losses"] += 1
            else:
                b["draws"] += 1
        for (a_name, o_name), b in sorted(agg_g.items()):
            rec = _record_from_stats_dict(a_name, o_name, b, src_str)
            if rec is not None:
                found.append(rec)
        if found:
            return found

    # 6. Flat `{anchor_or_matchup: {wins, losses, draws, win_rate}}`
    container = doc.get("results") if isinstance(doc.get("results"), Mapping) else doc
    if isinstance(container, Mapping):
        for k, v in container.items():
            if isinstance(v, Mapping) and ("win_rate" in v or "wins" in v):
                a_name, o_name = _split_matchup_key(str(k), default_arch=top_arch)
                rec = _record_from_stats_dict(a_name, o_name, v, src_str)
                if rec is not None:
                    found.append(rec)

    return found


def ingest_eval_matchups(eval_input: Path | str | Sequence[Path | str]) -> list[MatchupWeakness]:
    """Load and aggregate per-matchup win rates from `anchored_eval` or `specialist-gate-v1` files/dirs."""
    raw_inputs = [eval_input] if isinstance(eval_input, (Path, str)) else list(eval_input)
    json_files: list[Path] = []
    for raw in raw_inputs:
        p = Path(raw)
        if p.is_file():
            json_files.append(p)
        elif p.is_dir():
            # For `gates/gen<g>/`, read top-level `*.json` gate files (excluding sub-pool manifests)
            direct = sorted(p.glob("*.json"))
            if direct:
                json_files.extend(direct)
            else:
                json_files.extend(sorted(p.rglob("*.json")))

    merged: dict[tuple[str, str], MatchupWeakness] = {}
    for jf in json_files:
        if jf.name.endswith("-pool.json") or jf.name == "_library.json":
            continue
        try:
            doc = json.loads(jf.read_text(encoding="utf-8"))
        except Exception:
            continue
        if not isinstance(doc, Mapping):
            continue
        for m in _parse_single_eval_json(doc, jf):
            key = (m.archetype, m.opponent_archetype)
            prev = merged.get(key)
            if prev is None:
                merged[key] = m
            else:
                tot_games = prev.games + m.games
                tot_wins = prev.wins + m.wins
                tot_losses = prev.losses + m.losses
                tot_draws = prev.draws + m.draws
                wr = (
                    (prev.win_rate * prev.games + m.win_rate * m.games) / float(tot_games)
                    if tot_games > 0
                    else 0.0
                )
                merged[key] = MatchupWeakness(
                    archetype=m.archetype,
                    opponent_archetype=m.opponent_archetype,
                    win_rate=wr,
                    games=tot_games,
                    wins=tot_wins,
                    losses=tot_losses,
                    draws=tot_draws,
                    source_path=prev.source_path or m.source_path,
                )

    return sorted(
        merged.values(),
        key=lambda x: (x.win_rate, -x.games, x.archetype, x.opponent_archetype),
    )


def select_bottom_k_matchups(
    matchups: Sequence[MatchupWeakness],
    *,
    threshold: float = 0.45,
    bottom_k: int = 3,
    min_games: int = 1,
) -> list[MatchupWeakness]:
    """Return the bottom-`K` matchups strictly below `threshold`."""
    eligible = [
        m
        for m in matchups
        if m.games >= int(min_games) and float(m.win_rate) < float(threshold)
    ]
    eligible.sort(key=lambda x: (x.win_rate, -x.games, x.archetype, x.opponent_archetype))
    return eligible[: max(0, int(bottom_k))]


def _filter_recordings_for_matchups(
    recordings_paths: Sequence[Path | str],
    weak_matchups: Sequence[MatchupWeakness],
) -> list[Path]:
    """Return recording JSON files matching `weak_matchups` (falling back to all recordings if untagged)."""
    all_recs: list[Path] = []
    for raw in recordings_paths:
        p = Path(raw)
        if p.is_file() and not p.name.endswith(".winprob.json"):
            all_recs.append(p)
        elif p.is_dir():
            for child in sorted(p.rglob("*.json")):
                if not child.name.endswith(".winprob.json") and child.name != "_library.json":
                    all_recs.append(child)

    if not weak_matchups or not all_recs:
        return all_recs

    wanted_tokens = {
        tok.lower()
        for m in weak_matchups
        for tok in (m.archetype, m.opponent_archetype)
        if tok and tok.lower() not in ("agent", "opponent", "field", "greedy")
    }
    if not wanted_tokens:
        return all_recs

    matched: list[Path] = []
    for rf in all_recs:
        name_low = rf.name.lower()
        if any(t in name_low for t in wanted_tokens):
            matched.append(rf)
            continue
        try:
            doc = json.loads(rf.read_text(encoding="utf-8"))
            blob = json.dumps(doc.get("metadata") or doc.get("initial_state") or {}).lower()
            if any(t in blob for t in wanted_tokens):
                matched.append(rf)
        except Exception:
            continue

    return matched or all_recs


def _extract_root_target_and_avoid(doc: Mapping[str, Any]) -> tuple[Optional[int], list[int]]:
    target_id: Optional[int] = None
    avoid_ids: list[int] = []
    for ln in doc.get("lines") or []:
        if not isinstance(ln, Mapping):
            continue
        acts = [int(a) for a in (ln.get("actions") or [])]
        if not acts:
            continue
        role = str(ln.get("role") or "")
        if role == "target" and target_id is None:
            target_id = acts[0]
        elif role == "avoid" and acts[0] not in avoid_ids:
            avoid_ids.append(acts[0])
    for exp in doc.get("expectations") or []:
        if not isinstance(exp, Mapping):
            continue
        plan = [int(a) for a in (exp.get("plan") or [])]
        if not plan:
            continue
        if exp.get("kind") in ("choose", "chosen") and target_id is None:
            target_id = plan[0]
        elif exp.get("kind") in ("avoid", "not_chosen") and plan[0] not in avoid_ids:
            avoid_ids.append(plan[0])
    if target_id is not None:
        avoid_ids = [a for a in avoid_ids if a != target_id]
    return target_id, avoid_ids


def _infer_boss_template_for_opponent(opponent_archetype: str) -> Optional[str]:
    low = opponent_archetype.lower()
    if any(k in low for k in ("saneiketsu", "erlangmon", "saiyu", "shambala")):
        if "ex12-erlangmon-invitation" in BUILTIN_BOSS_TEMPLATES:
            return "ex12-erlangmon-invitation"
    return None


def run_auto_curriculum(
    eval_input: Path | str | Sequence[Path | str],
    *,
    out_dir: Path | str,
    engine_mcp: StdioMcpClient,
    training_mcp: Optional[StdioMcpClient] = None,
    router: Optional[LLMRouter] = None,
    recordings_paths: Optional[Sequence[Path | str]] = None,
    candidate_puzzles: Optional[Sequence[Path | str | Mapping[str, Any]]] = None,
    checkpoint_zip: Optional[Path | str] = None,
    model_id: Optional[str] = None,
    win_rate_threshold: float = 0.45,
    bottom_k: int = 3,
    min_games: int = 1,
    enable_synthesis: bool = True,
    boss_template: Optional[str] = None,
    deck_library_path: Optional[Path] = None,
    permutations_per_puzzle: int = 16,
    observation_profile: str = "standard_lite_v2",
    seed: int = 0,
    adversary_sims: int = 64,
    verify_seeds: int = 4,
    plasticity_steps: int = 10,
    plasticity_lr: float = 1e-3,
    max_puzzles_per_matchup: int = 2,
    require_admissible: bool = True,
    card_catalog: Optional[CardTextCatalog] = None,
) -> AutoCurriculumReport:
    """Execute the air-gapped Auto-Curriculum loop with Mandatory Dual Gate & SHA256 shard emission."""
    _ensure_code_paths()
    from tools.emit_puzzle_shards import emit_puzzle_shards

    catalog = card_catalog or CardTextCatalog()
    out_root = Path(out_dir).resolve()
    verified_dir = out_root / "verified_puzzles"
    extra_shards_dir = out_root / "extra_shards"
    staging_dir = out_root / "_staging"
    out_root.mkdir(parents=True, exist_ok=True)
    verified_dir.mkdir(parents=True, exist_ok=True)

    all_matchups = ingest_eval_matchups(eval_input)
    weak_matchups = select_bottom_k_matchups(
        all_matchups,
        threshold=win_rate_threshold,
        bottom_k=bottom_k,
        min_games=min_games,
    )

    staged_candidates: list[tuple[dict[str, Any], str]] = []

    # 1. Load any explicitly supplied candidate_puzzles (paths, dirs, or dicts)
    for item in candidate_puzzles or []:
        if isinstance(item, Mapping):
            doc = normalize_studio_puzzle_doc(item, card_catalog=catalog)
            m_label = str((doc.get("source") or {}).get("matchup") or "supplied")
            staged_candidates.append((doc, m_label))
            continue
        p = Path(item)
        if p.is_file():
            raw = json.loads(p.read_text(encoding="utf-8"))
            doc = normalize_studio_puzzle_doc(raw, card_catalog=catalog)
            m_label = str((doc.get("source") or {}).get("matchup") or p.stem)
            staged_candidates.append((doc, m_label))
        elif p.is_dir():
            for child in sorted(p.rglob("*.json")):
                if child.name == "_library.json":
                    continue
                try:
                    raw = json.loads(child.read_text(encoding="utf-8"))
                    if isinstance(raw, Mapping) and raw.get("schema") == "studio-puzzle-v1":
                        doc = normalize_studio_puzzle_doc(raw, card_catalog=catalog)
                        m_label = str((doc.get("source") or {}).get("matchup") or child.stem)
                        staged_candidates.append((doc, m_label))
                except Exception:
                    continue

    # 2. Mine replays for weak matchups when recordings + LLM router are provided
    if router is not None and recordings_paths and weak_matchups:
        matched_recs = _filter_recordings_for_matchups(recordings_paths, weak_matchups)
        if matched_recs:
            mined_dir = staging_dir / "mined"
            mine_res = mine_replays(
                matched_recs,
                router=router,
                engine_mcp=engine_mcp,
                training_mcp=training_mcp,
                checkpoint_path=Path(checkpoint_zip) if checkpoint_zip is not None else None,
                model_id=model_id,
                puzzles_dir=mined_dir,
                max_puzzles=max(1, len(weak_matchups) * int(max_puzzles_per_matchup)),
                adversary_sims=adversary_sims,
                verify_seeds=verify_seeds,
                require_plasticity=False,  # Central Dual Gate below handles quarantine accounting
                card_catalog=catalog,
            )
            for out_m in mine_res.outcomes:
                if out_m.accepted and out_m.puzzle_path and out_m.puzzle_path.is_file():
                    raw = json.loads(out_m.puzzle_path.read_text(encoding="utf-8"))
                    staged_candidates.append((raw, weak_matchups[0].label))

    # 3. Synthesize drills (`compose_boss_matrix` / `synthesize_archetype`) for bottom-K matchups
    if router is not None and enable_synthesis and weak_matchups:
        for wm in weak_matchups:
            tmpl_id = boss_template or _infer_boss_template_for_opponent(wm.opponent_archetype)
            if tmpl_id:
                try:
                    bm_dir = staging_dir / "boss_matrix"
                    bm_rep = compose_boss_matrix(
                        tmpl_id,
                        router=router,
                        engine_mcp=engine_mcp,
                        archetypes=[wm.archetype],
                        security_counts=(1,),
                        memories=(2,),
                        puzzles_dir=bm_dir,
                        deck_library_path=deck_library_path,
                        adversary_sims=adversary_sims,
                        verify_seeds=verify_seeds,
                        require_admissible=require_admissible,
                        card_catalog=catalog,
                    )
                    for cell in bm_rep.cells:
                        if cell.accepted and cell.puzzle_path and cell.puzzle_path.is_file():
                            raw = json.loads(cell.puzzle_path.read_text(encoding="utf-8"))
                            staged_candidates.append((raw, wm.label))
                    continue
                except Exception:
                    pass

            try:
                syn_dir = staging_dir / "synthesized"
                opp_arg = (
                    wm.opponent_archetype
                    if wm.opponent_archetype.lower() not in ("opponent", "field", "greedy", "base")
                    else None
                )
                syn_rep = synthesize_archetype(
                    wm.archetype,
                    router=router,
                    engine_mcp=engine_mcp,
                    training_mcp=training_mcp,
                    opponent_archetype=opp_arg,
                    checkpoint_zip=checkpoint_zip,
                    model_id=model_id,
                    puzzles_dir=syn_dir,
                    deck_library_path=deck_library_path,
                    count=int(max_puzzles_per_matchup),
                    require_admissible=require_admissible,
                    require_plasticity=False,  # Central Dual Gate below records quarantine cleanly
                    adversary_sims=adversary_sims,
                    verify_seeds=verify_seeds,
                    card_catalog=catalog,
                )
                for so in syn_rep.outcomes:
                    if so.accepted and so.puzzle_path and so.puzzle_path.is_file():
                        raw = json.loads(so.puzzle_path.read_text(encoding="utf-8"))
                        staged_candidates.append((raw, wm.label))
            except Exception:
                continue

    # ── 4. Mandatory Dual Gate (Verify Puzzle + Plasticity Probe) ─────────────
    use_plasticity_gate = checkpoint_zip is not None or (
        training_mcp is not None and training_mcp.has_tool("plasticity_probe")
    )
    inproc_plasticity_fn = None
    if checkpoint_zip is not None and (
        training_mcp is None or not training_mcp.has_tool("plasticity_probe")
    ):
        from digimon_training_mcp.whitebox import plasticity_probe as _inproc_probe

        inproc_plasticity_fn = _inproc_probe

    outcomes: list[CandidateGateOutcome] = []
    promoted_paths: list[Path] = []
    quarantine_entries: list[dict[str, Any]] = []
    rejected_verify_count = 0
    seen_ids: set[str] = set()

    for raw_doc, matchup_label in staged_candidates:
        try:
            doc = normalize_studio_puzzle_doc(raw_doc, card_catalog=catalog)
            # Strip nested subfolder so `<out_dir>/verified_puzzles/<id>.json` stays flat & clean
            doc["folder"] = ""
            validate_studio_puzzle_doc(doc, require_target_line=True)
        except PuzzleValidationError as exc:
            rejected_verify_count += 1
            outcomes.append(
                CandidateGateOutcome(
                    puzzle_id=str(raw_doc.get("id") or "invalid"),
                    title=str(raw_doc.get("title") or "invalid"),
                    matchup=matchup_label,
                    verify_passed=False,
                    verify_reason=f"schema validation failed: {exc}",
                )
            )
            continue

        pid = str(doc["id"])
        if pid in seen_ids:
            continue
        seen_ids.add(pid)

        # Gate 1: MCP `verify_puzzle`
        verify_args: dict[str, Any] = {
            "puzzle_json": doc,
            "adversary_sims": int(adversary_sims),
            "seeds": int(verify_seeds),
        }
        if model_id:
            verify_args["model_id"] = model_id
        verify_res = engine_mcp.call_tool("verify_puzzle", verify_args)
        v_passed, v_reason = evaluate_verify_puzzle_result(
            verify_res,
            solver_seat=int(doc.get("seat", 1)),
        )
        if not v_passed:
            rejected_verify_count += 1
            outcomes.append(
                CandidateGateOutcome(
                    puzzle_id=pid,
                    title=doc["title"],
                    matchup=matchup_label,
                    verify_passed=False,
                    verify_reason=v_reason,
                    verify_result=verify_res,
                )
            )
            continue

        # Gate 2: White-box `plasticity_probe`
        target_act, avoid_acts = _extract_root_target_and_avoid(doc)
        if use_plasticity_gate:
            if target_act is None:
                plast_res: dict[str, Any] = {
                    "ok": False,
                    "learnable": False,
                    "diagnosis": "missing target root action for plasticity_probe",
                }
            elif training_mcp is not None and training_mcp.has_tool("plasticity_probe"):
                p_args: dict[str, Any] = {
                    "scenario_json": doc,
                    "target_action_id": int(target_act),
                    "avoid_action_ids": list(avoid_acts),
                    "steps": int(plasticity_steps),
                }
                if checkpoint_zip is not None:
                    p_args["checkpoint_path"] = str(checkpoint_zip)
                plast_res = training_mcp.call_tool("plasticity_probe", p_args)
            elif inproc_plasticity_fn is not None and checkpoint_zip is not None:
                plast_res = inproc_plasticity_fn(
                    checkpoint_path=Path(checkpoint_zip),
                    scenario_json=doc,
                    target_action_id=int(target_act),
                    avoid_action_ids=list(avoid_acts),
                    steps=int(plasticity_steps),
                    lr=float(plasticity_lr),
                )
            else:
                plast_res = {"ok": False, "learnable": False, "diagnosis": "no plasticity backend configured"}

            learnable = bool(plast_res.get("ok") is not False and plast_res.get("learnable") is True)
            diag = str(plast_res.get("diagnosis") or plast_res.get("error") or "")

            if not learnable:
                quarantine_entries.append(
                    {
                        "puzzle_id": pid,
                        "title": doc["title"],
                        "matchup": matchup_label,
                        "target_action_id": target_act,
                        "avoid_action_ids": avoid_acts,
                        "diagnosis": diag,
                        "loss_start": plast_res.get("loss_start"),
                        "loss_end": plast_res.get("loss_end"),
                        "pre": plast_res.get("pre"),
                        "post": plast_res.get("post"),
                        "puzzle": doc,
                    }
                )
                outcomes.append(
                    CandidateGateOutcome(
                        puzzle_id=pid,
                        title=doc["title"],
                        matchup=matchup_label,
                        verify_passed=True,
                        verify_reason=v_reason,
                        plasticity_checked=True,
                        learnable=False,
                        plasticity_diagnosis=diag,
                        promoted=False,
                        quarantined=True,
                        verify_result=verify_res,
                        plasticity_result=plast_res,
                    )
                )
                continue

            saved_path, _ = write_studio_puzzle(doc, verified_dir)
            promoted_paths.append(saved_path)
            outcomes.append(
                CandidateGateOutcome(
                    puzzle_id=pid,
                    title=doc["title"],
                    matchup=matchup_label,
                    verify_passed=True,
                    verify_reason=v_reason,
                    plasticity_checked=True,
                    learnable=True,
                    plasticity_diagnosis=diag,
                    promoted=True,
                    quarantined=False,
                    verified_puzzle_path=saved_path,
                    verify_result=verify_res,
                    plasticity_result=plast_res,
                )
            )
        else:
            saved_path, _ = write_studio_puzzle(doc, verified_dir)
            promoted_paths.append(saved_path)
            outcomes.append(
                CandidateGateOutcome(
                    puzzle_id=pid,
                    title=doc["title"],
                    matchup=matchup_label,
                    verify_passed=True,
                    verify_reason=v_reason,
                    plasticity_checked=False,
                    learnable=None,
                    promoted=True,
                    quarantined=False,
                    verified_puzzle_path=saved_path,
                    verify_result=verify_res,
                )
            )

    # Always write `tensor_blindness_quarantine.json` deterministically
    quarantine_path = out_root / "tensor_blindness_quarantine.json"
    quarantine_doc = {
        "schema": QUARANTINE_SCHEMA,
        "checkpoint_path": str(checkpoint_zip) if checkpoint_zip is not None else None,
        "quarantined_count": len(quarantine_entries),
        "quarantined": quarantine_entries,
    }
    quarantine_path.write_text(json.dumps(quarantine_doc, indent=2) + "\n", encoding="utf-8")

    # ── 5. Compile Promoted Puzzles into `selfplay-shards-v1` + SHA256 Manifest ─
    extra_shards_summary: Optional[dict[str, Any]] = None
    bundle_sha256: Optional[str] = None
    emitted_shards_dir: Optional[Path] = None

    if promoted_paths:
        shard_manifest = emit_puzzle_shards(
            puzzles_or_exams=promoted_paths,
            out_dir=extra_shards_dir,
            permutations_per_puzzle=int(permutations_per_puzzle),
            observation_profile=observation_profile,
            seed=int(seed),
        )
        emitted_shards_dir = extra_shards_dir

        shard_files_meta: list[dict[str, Any]] = []
        bundle_hasher = hashlib.sha256()
        for sf in sorted(extra_shards_dir.rglob("*")):
            if not sf.is_file():
                continue
            rel = str(sf.relative_to(extra_shards_dir)).replace("\\", "/")
            f_sha = _sha256_file(sf)
            bundle_hasher.update(f"{rel}:{f_sha}\n".encode("utf-8"))
            shard_files_meta.append(
                {
                    "path": rel,
                    "sha256": f_sha,
                    "bytes": sf.stat().st_size,
                }
            )
        bundle_sha256 = bundle_hasher.hexdigest()
        shard_manifest_path = extra_shards_dir / "manifest.json"
        extra_shards_summary = {
            "schema": shard_manifest.get("schema", "selfplay-shards-v1"),
            "dir": str(extra_shards_dir),
            "manifest_path": str(shard_manifest_path),
            "manifest_sha256": _sha256_file(shard_manifest_path),
            "bundle_sha256": bundle_sha256,
            "observation_profile": observation_profile,
            "puzzles": int(shard_manifest.get("puzzles", len(promoted_paths))),
            "games": int(shard_manifest.get("games", 0)),
            "rows": int(shard_manifest.get("rows", 0)),
            "avoid_edges": int(shard_manifest.get("avoid_edges", 0)),
            "files": shard_files_meta,
        }

    verified_entries = [
        {
            "id": p.stem,
            "path": str(p),
            "sha256": _sha256_file(p),
        }
        for p in sorted(promoted_paths)
    ]

    curriculum_manifest = {
        "schema": CURRICULUM_MANIFEST_SCHEMA,
        "win_rate_threshold": float(win_rate_threshold),
        "bottom_k": int(bottom_k),
        "observation_profile": observation_profile,
        "checkpoint_path": str(checkpoint_zip) if checkpoint_zip is not None else None,
        "selected_matchups": [m.to_dict() for m in weak_matchups],
        "candidates_evaluated": len(outcomes),
        "promoted_count": len(promoted_paths),
        "quarantined_count": len(quarantine_entries),
        "rejected_verify_count": rejected_verify_count,
        "verified_puzzles": verified_entries,
        "quarantine_path": str(quarantine_path),
        "quarantine_sha256": _sha256_file(quarantine_path),
        "extra_shards": extra_shards_summary,
        "outcomes": [o.to_dict() for o in outcomes],
    }
    manifest_path = out_root / "curriculum_manifest.json"
    manifest_path.write_text(json.dumps(curriculum_manifest, indent=2) + "\n", encoding="utf-8")

    return AutoCurriculumReport(
        out_dir=out_root,
        all_matchups=all_matchups,
        selected_matchups=weak_matchups,
        candidates_evaluated=len(outcomes),
        promoted_count=len(promoted_paths),
        quarantined_count=len(quarantine_entries),
        rejected_verify_count=rejected_verify_count,
        outcomes=outcomes,
        manifest_path=manifest_path,
        quarantine_path=quarantine_path,
        extra_shards_dir=emitted_shards_dir,
        bundle_sha256=bundle_sha256,
        manifest_dict=curriculum_manifest,
    )


__all__ = [
    "CURRICULUM_MANIFEST_SCHEMA",
    "QUARANTINE_SCHEMA",
    "AutoCurriculumReport",
    "CandidateGateOutcome",
    "MatchupWeakness",
    "ingest_eval_matchups",
    "run_auto_curriculum",
    "select_bottom_k_matchups",
]
