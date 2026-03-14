"""Deck optimizer endpoints — REST API for on-demand deck optimization.

Engine-only router — no database or auth dependencies.
Safe to mount in the desktop sidecar alongside the hosted API.

Register in api.py and/or desktop_main.py:
    from digimon_gym.routers.deck_optimizer import router as deck_optimizer_router
    app.include_router(deck_optimizer_router)
"""

from __future__ import annotations

import json
import logging
import os
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

import numpy as np
from fastapi import APIRouter, HTTPException
from pydantic import BaseModel

router = APIRouter(prefix="/deck-optimizer", tags=["deck-optimizer"])
logger = logging.getLogger(__name__)

ARCHITECT_MODELS_DIR = os.environ.get("ARCHITECT_MODELS_DIR", "architect_runs")


# ---------------------------------------------------------------------------
# Request / response schemas
# ---------------------------------------------------------------------------


class OptimizeRequest(BaseModel):
    archetype: str
    base_deck: List[str]
    architect_model: str  # directory name under ARCHITECT_MODELS_DIR
    max_swaps: int = 10
    extra_candidates: Optional[List[str]] = None


class SwapRecord(BaseModel):
    step: int
    removed: str
    added: str


class OptimizeResponse(BaseModel):
    optimized_deck: List[str]
    estimated_win_rate: float
    base_win_rate: float
    swaps_made: List[SwapRecord]
    n_swaps: int


class EvaluateRequest(BaseModel):
    deck: List[str]
    meta_config_name: str = "local_meta_dfw"
    pilot: str = "greedy"
    games: int = 50


class EvaluateResponse(BaseModel):
    win_rate: float
    games_played: int


class ModelInfo(BaseModel):
    name: str
    archetype: str
    final_win_rate: float
    episodes: int
    timestamp: str


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _resolve_model_dir(model_name: str) -> Path:
    """Resolve and validate a model directory under ARCHITECT_MODELS_DIR.

    Uses ``Path.name`` sanitization to prevent path traversal.
    """
    safe_name = Path(model_name).name
    if not safe_name:
        raise HTTPException(status_code=400, detail="Invalid model name")
    model_dir = Path(ARCHITECT_MODELS_DIR) / safe_name
    if not model_dir.is_dir():
        raise HTTPException(
            status_code=404,
            detail=f"Architect model directory not found: {safe_name}",
        )
    return model_dir


def _load_pool(model_dir: Path):
    """Load CandidatePool from a model directory (lazy import)."""
    from digimon_gym.agents.architect_pool import CandidatePool

    pool_path = model_dir / "candidate_pool.json"
    if not pool_path.exists():
        raise HTTPException(
            status_code=404,
            detail=f"candidate_pool.json not found in {model_dir.name}",
        )
    with open(pool_path, "r", encoding="utf-8") as f:
        pool_data = json.load(f)
    return CandidatePool.from_json(pool_data)


def _load_agent(model_dir: Path):
    """Load ArchitectDQN from a model directory (lazy import)."""
    from digimon_gym.agents.architect_agent import ArchitectDQN

    agent_path = model_dir / "architect_dqn.pt"
    if not agent_path.exists():
        raise HTTPException(
            status_code=404,
            detail=f"architect_dqn.pt not found in {model_dir.name}",
        )
    return ArchitectDQN.load(str(agent_path), device="cpu")


def _load_metadata(model_dir: Path) -> dict:
    """Load metadata.json from a model directory."""
    meta_path = model_dir / "metadata.json"
    if not meta_path.exists():
        raise HTTPException(
            status_code=404,
            detail=f"metadata.json not found in {model_dir.name}",
        )
    with open(meta_path, "r", encoding="utf-8") as f:
        return json.load(f)


def _load_meta_config(model_dir: Path) -> dict:
    """Load meta_config.json from a model directory."""
    config_path = model_dir / "meta_config.json"
    if not config_path.exists():
        raise HTTPException(
            status_code=404,
            detail=f"meta_config.json not found in {model_dir.name}",
        )
    with open(config_path, "r", encoding="utf-8") as f:
        return json.load(f)


def _separate_egg_deck(deck_ids: List[str]) -> Tuple[List[str], List[str]]:
    """Split a flat deck list into (egg_deck, main_deck).

    Lazy-imports CardDatabase to avoid heavy module load at router import.
    """
    from digimon_gym.engine.data.card_database import CardDatabase
    from digimon_gym.engine.data.enums import CardKind

    db = CardDatabase()
    egg_deck: List[str] = []
    main_deck: List[str] = []
    for cid in deck_ids:
        entity = db.get_card(cid)
        if entity is not None and entity.card_kind == CardKind.DigiEgg:
            egg_deck.append(cid)
        else:
            main_deck.append(cid)
    return egg_deck, main_deck


def _build_state(
    pool,
    deck_counts: Dict[str, int],
    meta_weights: np.ndarray,
    step: int,
    max_swaps: int,
) -> np.ndarray:
    """Construct the observation vector matching DeckBuildingEnv._build_obs."""
    deck_vec = pool.deck_to_vector(deck_counts)
    step_frac = np.array([step / max_swaps], dtype=np.float32)
    return np.concatenate([deck_vec, meta_weights, step_frac])


def _build_meta_weights(meta_config: dict, archetype: str) -> np.ndarray:
    """Extract normalised meta weights from meta_config, excluding the
    agent's own archetype."""
    weights: List[float] = []
    for arch_name, arch_stats in meta_config.get("archetypes", {}).items():
        if arch_name == archetype:
            continue
        w = arch_stats.get("local_meta_share") or arch_stats.get(
            "global_meta_share", 0
        )
        if w > 0:
            weights.append(float(w))
    arr = np.array(weights, dtype=np.float32)
    total = arr.sum()
    if total > 0:
        arr /= total
    elif len(arr) > 0:
        arr[:] = 1.0 / len(arr)
    return arr


# ---------------------------------------------------------------------------
# Endpoints
# ---------------------------------------------------------------------------


@router.post("/optimize", response_model=OptimizeResponse)
def optimize_deck(request: OptimizeRequest):
    """Optimize a deck using a trained architect DQN model.

    Runs greedy inference (no simulation) — returns in milliseconds.
    The estimated win rate comes from training metadata, not live games.
    """
    model_dir = _resolve_model_dir(request.architect_model)

    pool = _load_pool(model_dir)
    agent = _load_agent(model_dir)
    metadata = _load_metadata(model_dir)
    meta_config = _load_meta_config(model_dir)

    # Separate egg deck from main deck
    egg_deck, main_ids = _separate_egg_deck(request.base_deck)

    # Convert to counts (main deck only)
    deck_counts = pool.deck_list_to_counts(main_ids)

    # Build meta weights vector (same shape as training)
    meta_weights = _build_meta_weights(meta_config, request.archetype)

    # Greedy inference loop
    max_swaps = request.max_swaps
    swaps_made: List[SwapRecord] = []

    for step in range(max_swaps):
        state = _build_state(pool, deck_counts, meta_weights, step, max_swaps)
        action_mask = pool.get_action_mask(deck_counts)

        action = agent.greedy_action(state, action_mask)

        # No-op means the agent elects to stop
        swap = pool.decode_action(action)
        if swap is None:
            break

        remove_card, add_card = swap
        deck_counts = pool.apply_swap(deck_counts, remove_card, add_card)
        swaps_made.append(
            SwapRecord(step=step, removed=remove_card, added=add_card)
        )

    optimized_deck = pool.counts_to_deck_list(deck_counts, egg_deck)

    # Win rates from training metadata (no live simulation)
    base_win_rate = metadata.get("base_win_rate", 0.0)
    estimated_win_rate = metadata.get("best_win_rate", 0.0)

    return OptimizeResponse(
        optimized_deck=optimized_deck,
        estimated_win_rate=estimated_win_rate,
        base_win_rate=base_win_rate,
        swaps_made=swaps_made,
        n_swaps=len(swaps_made),
    )


@router.get("/models", response_model=List[ModelInfo])
def list_models():
    """List available architect models from ARCHITECT_MODELS_DIR."""
    models_dir = Path(ARCHITECT_MODELS_DIR)
    if not models_dir.is_dir():
        return []

    results: List[ModelInfo] = []
    for entry in sorted(models_dir.iterdir()):
        if not entry.is_dir():
            continue
        meta_path = entry / "metadata.json"
        if not meta_path.exists():
            continue
        try:
            with open(meta_path, "r", encoding="utf-8") as f:
                meta = json.load(f)
            results.append(
                ModelInfo(
                    name=entry.name,
                    archetype=meta.get("archetype", "unknown"),
                    final_win_rate=float(meta.get("best_win_rate", 0.0)),
                    episodes=int(meta.get("episodes", 0)),
                    timestamp=meta.get("timestamp", ""),
                )
            )
        except (json.JSONDecodeError, KeyError, ValueError):
            logger.warning("Skipping malformed metadata in %s", entry.name)
            continue

    return results


@router.post("/evaluate", response_model=EvaluateResponse)
def evaluate_deck(request: EvaluateRequest):
    """Evaluate a deck's win rate against a meta configuration via simulation.

    This endpoint runs actual games and may take several seconds depending
    on the number of games requested.
    """
    from digimon_gym.agents.architect_optimizer import MetaOptimizer
    from digimon_gym.agents.architect_simulator import DeckSimulator

    # Load meta config from file
    safe_name = Path(request.meta_config_name).name
    if not safe_name:
        raise HTTPException(status_code=400, detail="Invalid meta config name")

    # Search for meta config in known locations
    config_path: Optional[Path] = None
    search_paths = [
        Path(ARCHITECT_MODELS_DIR) / f"{safe_name}.json",
        Path("meta_configs") / f"{safe_name}.json",
        Path(safe_name),
    ]
    for candidate in search_paths:
        if candidate.exists():
            config_path = candidate
            break

    if config_path is None:
        raise HTTPException(
            status_code=404,
            detail=f"Meta config not found: {safe_name}",
        )

    with open(config_path, "r", encoding="utf-8") as f:
        meta_config = json.load(f)

    # Build opponents using MetaOptimizer's logic
    # We create a temporary optimizer just to use build_opponents()
    temp_optimizer = MetaOptimizer(
        archetype_name="__eval__",
        base_deck=request.deck,
        meta_config=meta_config,
        pilot_policy_path=request.pilot,
    )
    opponents = temp_optimizer.build_opponents()

    if not opponents:
        raise HTTPException(
            status_code=422,
            detail="No valid opponents could be built from meta config. "
            "Check that deck_library.json has simulatable decklists "
            "for the archetypes referenced in the meta config.",
        )

    # Create simulator and evaluate
    simulator = DeckSimulator(
        pilot_policy_path=request.pilot,
        opponent_policy="greedy",
        games_per_eval=request.games,
        max_turns=200,
        n_workers=1,
        cache_enabled=False,
    )

    win_rate = simulator.evaluate_deck(request.deck, opponents)

    # Total games actually played (games_per_eval distributed across opponents)
    total_games = sum(
        max(2, round(request.games * (w / sum(ww for _, ww in opponents))))
        for _, w in opponents
    )

    return EvaluateResponse(
        win_rate=win_rate,
        games_played=total_games,
    )
