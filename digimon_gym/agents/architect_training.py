"""Architect Agent training CLI — deck optimization via meta-aware DQN.

Trains an architect agent to optimize a base decklist for a given archetype
against a target meta. The agent learns card-swap decisions that maximise
win rate using the MetaOptimizer pipeline.

Usage:
    python -m digimon_gym.agents.architect_training --archetype "Medusamon"
    python -m digimon_gym.agents.architect_training --archetype "Medusamon" \
        --meta local_meta.json --pilot models/mlp_agent.onnx --episodes 500
    python -m digimon_gym.agents.architect_training --archetype "Medusamon" \
        --extra-cards "BT24-001,EX10-036" --workers 4 --seed 42

No DB imports. No auth. Pure engine + training.
"""

from __future__ import annotations

import argparse
import json
import logging
import sys
from pathlib import Path
from typing import Dict, List, Optional

from digimon_gym.agents.architect_optimizer import MetaOptimizer
from digimon_gym.agents.architect_pool import CardConstraint

logger = logging.getLogger(__name__)

from digimon_gym.data_paths import DECK_LIBRARY as _DECK_LIBRARY_PATH

# Prefer decklists from sources with better data quality.
_SOURCE_PREFERENCE = {
    "digilab": 3,
    "digimonmeta": 2,
    "egman": 1,
}


def resolve_base_deck(archetype_name: str) -> List[str]:
    """Find the best decklist for the given archetype from deck_library.

    Prefers decklists from higher-quality sources (digilab > digimonmeta > egman).
    Returns the card ID list for the first matching decklist.

    Raises:
        ValueError: If the archetype has no decklists in the library.
        FileNotFoundError: If deck_library.json is missing.
    """
    if not _DECK_LIBRARY_PATH.exists():
        raise FileNotFoundError(
            f"deck_library.json not found at {_DECK_LIBRARY_PATH}"
        )

    with open(_DECK_LIBRARY_PATH, "r", encoding="utf-8") as f:
        library = json.load(f)

    arch = library.get("archetypes", {}).get(archetype_name)
    if not arch or not arch.get("decklists"):
        raise ValueError(f"No decklists found for archetype: {archetype_name}")

    decklists = sorted(
        arch["decklists"],
        key=lambda d: _SOURCE_PREFERENCE.get(d.get("source", ""), 0),
        reverse=True,
    )

    for entry in decklists:
        raw = entry.get("decklist")
        if not raw:
            continue
        deck = json.loads(raw) if isinstance(raw, str) else raw
        if deck:
            return deck

    raise ValueError(f"No valid decklists for archetype: {archetype_name}")


def build_default_meta_config() -> dict:
    """Build a meta config from deck_library.json global stats.

    Filters archetypes with less than 0.5% meta share to keep the
    opponent pool focused on relevant threats.
    """
    if not _DECK_LIBRARY_PATH.exists():
        raise FileNotFoundError(
            f"deck_library.json not found at {_DECK_LIBRARY_PATH}"
        )

    with open(_DECK_LIBRARY_PATH, "r", encoding="utf-8") as f:
        library = json.load(f)

    archetypes = {}
    for arch_name, arch_data in library.get("archetypes", {}).items():
        stats = arch_data.get("stats", {})
        meta_share = stats.get("meta_share", 0)
        if meta_share > 0.005:
            archetypes[arch_name] = {
                "global_meta_share": meta_share,
                "global_conversion_rate": stats.get("conversion_rate", 0),
            }

    return {"archetypes": archetypes}


def _parse_card_pairs(raw: str) -> Dict[str, int]:
    """Parse 'CARD:N,CARD:N,...' into {card_id: count}."""
    result: Dict[str, int] = {}
    for pair in raw.split(","):
        pair = pair.strip()
        if not pair:
            continue
        if ":" not in pair:
            raise ValueError(f"Invalid card:count pair: {pair!r}")
        card_id, count_str = pair.rsplit(":", 1)
        result[card_id.strip()] = int(count_str.strip())
    return result


def _build_constraints(args: argparse.Namespace) -> Optional[Dict[str, CardConstraint]]:
    """Build CardConstraint dict from CLI flags."""
    locks: Dict[str, int] = {}
    mins: Dict[str, int] = {}
    maxs: Dict[str, int] = {}

    if args.lock_cards:
        locks = _parse_card_pairs(args.lock_cards)
    if args.min_counts:
        mins = _parse_card_pairs(args.min_counts)
    if args.max_counts:
        maxs = _parse_card_pairs(args.max_counts)

    if not locks and not mins and not maxs:
        return None

    all_cards = set(locks) | set(mins) | set(maxs)
    constraints: Dict[str, CardConstraint] = {}
    for cid in all_cards:
        if cid in locks:
            constraints[cid] = CardConstraint(
                locked=True,
                locked_count=locks[cid],
            )
        else:
            constraints[cid] = CardConstraint(
                min_count=mins.get(cid, 0),
                max_count=maxs.get(cid) if cid in maxs else None,
            )
    return constraints


def train(args: argparse.Namespace) -> None:
    """Run architect training with the given CLI arguments."""
    # --- Meta config ---
    if args.meta:
        meta_path = Path(args.meta)
        if not meta_path.exists():
            print(f"ERROR: Meta config not found: {meta_path}", file=sys.stderr)
            sys.exit(1)
        with open(meta_path, "r", encoding="utf-8") as f:
            meta_config = json.load(f)
        print(f"Meta config: {meta_path}")
    else:
        meta_config = build_default_meta_config()
        print(
            f"Meta config: built from deck_library.json "
            f"({len(meta_config.get('archetypes', {}))} archetypes)"
        )

    # --- Base deck ---
    try:
        base_deck = resolve_base_deck(args.archetype)
    except (ValueError, FileNotFoundError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        sys.exit(1)
    print(f"Base deck:   {args.archetype} ({len(base_deck)} cards)")

    # --- Extra cards ---
    extra_cards: Optional[List[str]] = None
    if args.extra_cards:
        extra_cards = [c.strip() for c in args.extra_cards.split(",") if c.strip()]
        print(f"Extra cards: {len(extra_cards)} added to pool")

    # --- Custom pool ---
    custom_pool: Optional[List[str]] = None
    if args.pool_file:
        pool_path = Path(args.pool_file)
        if not pool_path.exists():
            print(f"ERROR: Pool file not found: {pool_path}", file=sys.stderr)
            sys.exit(1)
        with open(pool_path, "r", encoding="utf-8") as f:
            custom_pool = json.load(f)
        print(f"Custom pool: {len(custom_pool)} cards from {pool_path}")

    # --- Card constraints ---
    card_constraints = _build_constraints(args)
    if card_constraints:
        print(f"Constraints: {len(card_constraints)} cards constrained")

    # --- Create optimizer ---
    optimizer = MetaOptimizer(
        archetype_name=args.archetype,
        base_deck=base_deck,
        meta_config=meta_config,
        pilot_policy_path=args.pilot,
        output_dir=args.output_dir,
        extra_cards=extra_cards,
        custom_pool=custom_pool,
        card_constraints=card_constraints,
        use_restricted_list=args.use_restricted_list,
    )

    # --- Train ---
    print(f"\nStarting architect training...")
    print(f"  Episodes:       {args.episodes}")
    print(f"  Games/eval:     {args.games_per_eval}")
    print(f"  Max swaps:      {args.max_swaps}")
    print(f"  Workers:        {args.workers}")
    print(f"  Pilot:          {args.pilot}")
    print(f"  Reward amp:     {args.reward_amp}")
    print(f"  Checkpoint:     every {args.checkpoint_freq} episodes")
    if args.seed is not None:
        print(f"  Seed:           {args.seed}")
    print()

    results = optimizer.train(
        episodes=args.episodes,
        games_per_eval=args.games_per_eval,
        max_swaps=args.max_swaps,
        n_workers=args.workers,
        reward_amplification=args.reward_amp,
        checkpoint_freq=args.checkpoint_freq,
        seed=args.seed,
    )

    # --- Summary ---
    print()
    print("=== Architect Training Complete ===")
    print(f"Archetype:    {args.archetype}")
    print(f"Episodes:     {results.get('episodes', args.episodes)}")
    print(f"Base WR:      {results.get('base_win_rate', 0):.3f}")
    print(f"Final WR:     {results.get('final_win_rate', 0):.3f}")
    print(f"Best WR:      {results.get('best_win_rate', 0):.3f}")
    print(f"Pilot:        {args.pilot}")
    print(f"Output:       {results.get('output_dir', args.output_dir)}")


def main() -> None:
    """Parse CLI arguments and launch training."""
    parser = argparse.ArgumentParser(
        description="Train an architect agent to optimize a deck for a target meta.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )

    parser.add_argument(
        "--archetype",
        required=True,
        help="Archetype name as it appears in deck_library.json",
    )
    parser.add_argument(
        "--meta",
        default=None,
        help="Path to meta config JSON (default: built from deck_library.json global stats)",
    )
    parser.add_argument(
        "--pilot",
        default="greedy",
        help='Pilot policy path or "greedy" (default: greedy)',
    )
    parser.add_argument(
        "--episodes",
        type=int,
        default=200,
        help="Number of training episodes (default: 200)",
    )
    parser.add_argument(
        "--games-per-eval",
        type=int,
        default=50,
        help="Games per win-rate evaluation (default: 50)",
    )
    parser.add_argument(
        "--max-swaps",
        type=int,
        default=10,
        help="Max card swaps per episode (default: 10)",
    )
    parser.add_argument(
        "--workers",
        type=int,
        default=1,
        help="Parallel simulation workers (default: 1)",
    )
    parser.add_argument(
        "--output-dir",
        default="architect_runs",
        help="Output directory for results (default: architect_runs)",
    )
    parser.add_argument(
        "--extra-cards",
        default=None,
        help="Comma-separated extra card IDs for the pool",
    )
    parser.add_argument(
        "--pool-file",
        default=None,
        help="Path to custom pool JSON file (list of card IDs)",
    )
    parser.add_argument(
        "--reward-amp",
        type=float,
        default=10.0,
        help="Reward amplification constant b (default: 10.0)",
    )
    parser.add_argument(
        "--checkpoint-freq",
        type=int,
        default=50,
        help="Checkpoint every N episodes (default: 50)",
    )
    parser.add_argument(
        "--seed",
        type=int,
        default=None,
        help="Random seed for reproducibility",
    )
    parser.add_argument(
        "--lock-cards",
        default=None,
        help="Lock cards at exact count (CARD:N,CARD:N,...)",
    )
    parser.add_argument(
        "--min-counts",
        default=None,
        help="Minimum copies per card (CARD:N,CARD:N,...)",
    )
    parser.add_argument(
        "--max-counts",
        default=None,
        help="Maximum copies per card, capped at 4 (CARD:N,CARD:N,...)",
    )
    parser.add_argument(
        "--use-restricted-list",
        action="store_true",
        help="Enforce official ban/restricted list",
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="Enable debug logging",
    )

    args = parser.parse_args()

    logging.basicConfig(
        level=logging.DEBUG if args.verbose else logging.INFO,
        format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    )

    train(args)


if __name__ == "__main__":
    main()
