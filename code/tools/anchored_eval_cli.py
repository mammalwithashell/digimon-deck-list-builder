"""Anchored evaluation CLI — score a candidate model against the fixed anchors
(greedy + registered champions), seat-balanced, on one comparable scale.

This is the trustworthy progress signal the in-run win rate cannot give under
self-play. See the `add-model-evaluation-harness` change and
`code/digimon_gym/agents/anchored_eval.py`.

Examples:
    python code/tools/anchored_eval_cli.py --candidate models/champions/registry.json:v022-generalist-v1
    python code/tools/anchored_eval_cli.py \
        --candidate /path/to/run/checkpoints/step_000500000.zip \
        --champions models/champions/registry.json --n 40
"""
from __future__ import annotations

import argparse
import glob
import json
import os
import re
import sys
from pathlib import Path

from digimon_gym.agents.anchored_eval import (
    MatchRecord, evaluate_against_anchors,
)
from digimon_gym.agents.champion_registry import ChampionRegistry
from digimon_gym.agents.gauntlet import GeneralistDeckPool, load_generalist_deck_pool
from digimon_gym.agents.pilot_training import MaskableRecurrentPPO

DEFAULT_STARTERS = [
    "ST-1 Gaia Red", "ST-2 Cocytus Blue", "ST-3 Heaven's Yellow",
    "ST-4 Giga Green", "ST-5 Machine Black", "ST-6 Venomous Violet",
]
DEFAULT_HASH = "sha256:a20462fbfede51ad3b1585291ea1ec1259b1a6b9c6156cff033db5f9f1ead39e"


def resolve_candidate(path: str) -> str:
    """Resolve a candidate to a concrete .zip: a run dir picks its latest
    checkpoint (else final.zip); a .zip is used directly."""
    p = Path(path)
    if p.is_dir():
        ck_dir = p / "checkpoints"
        cks = glob.glob(str(ck_dir / "step_*.zip")) if ck_dir.is_dir() else []
        if cks:
            cks.sort(key=lambda c: int(re.search(r"step_0*(\d+)\.zip", c).group(1)))
            return cks[-1]
        if (p / "final.zip").exists():
            return str(p / "final.zip")
        sys.exit(f"no checkpoints or final.zip under {path}")
    return path


def candidate_layout_hash(candidate_zip: str, override: str | None) -> str:
    if override:
        return override
    # Try a sibling/run meta.json next to the checkpoint or run dir.
    for cand in (Path(candidate_zip).with_suffix(".meta.json"),
                 Path(candidate_zip).parent.parent / "*.meta.json"):
        for m in glob.glob(str(cand)):
            try:
                return str(json.loads(Path(m).read_text()).get("tensor_layout_hash") or DEFAULT_HASH)
            except Exception:
                pass
    return DEFAULT_HASH


def fmt(rec: MatchRecord) -> str:
    return f"{rec.wins}-{rec.losses}-{rec.draws}  WR={100 * rec.win_rate:5.1f}%  (n={rec.games})"


def main() -> None:
    ap = argparse.ArgumentParser(description="Anchored evaluation against greedy + champions.")
    ap.add_argument("--candidate", required=True, help="checkpoint .zip or run dir")
    ap.add_argument("--champions", default="models/champions/registry.json")
    ap.add_argument("--n", type=int, default=40, help="seat-balanced games per anchor")
    ap.add_argument("--seed", type=int, default=777)
    ap.add_argument("--profile", default="standard_lite_deck_v2")
    ap.add_argument("--device", default="auto", help="auto|cpu|cuda")
    ap.add_argument("--layout-hash", default=None, help="override candidate layout hash")
    ap.add_argument("--no-greedy", action="store_true")
    ap.add_argument("--deck-pool-snapshot", default=None,
                    help="frozen deck_pool_snapshot.json (preferred — reproducible, "
                         "and the exact decks a run trained on)")
    ap.add_argument("--archetypes", default=None,
                    help="comma-separated archetypes from deck_library.json "
                         "(fallback if no snapshot); default = 6 starters")
    args = ap.parse_args()

    cand_zip = resolve_candidate(args.candidate)
    lhash = candidate_layout_hash(cand_zip, args.layout_hash)
    if args.deck_pool_snapshot:
        from digimon_engine import load_implemented_card_ids
        pool = GeneralistDeckPool.from_snapshot(
            args.deck_pool_snapshot,
            implemented_card_ids=load_implemented_card_ids(),
        )
    else:
        archetypes = ([a.strip() for a in args.archetypes.split(",")] if args.archetypes
                      else DEFAULT_STARTERS)
        pool = load_generalist_deck_pool(allowed_archetypes=set(archetypes))

    print(f"candidate : {cand_zip}")
    print(f"layout    : {lhash[:22]}…  profile={args.profile}  device={args.device}")
    print(f"anchors   : {'greedy + ' if not args.no_greedy else ''}"
          f"{ChampionRegistry.load(args.champions).names()}")
    print(f"games/anchor: {args.n} (seat-balanced)  seed={args.seed}\n", flush=True)

    candidate = MaskableRecurrentPPO.load(cand_zip, device=args.device)
    registry = ChampionRegistry.load(args.champions)

    results = evaluate_against_anchors(
        candidate, lhash, registry, pool,
        n=args.n, base_seed=args.seed, tensor_profile=args.profile,
        include_greedy=not args.no_greedy,
    )

    print("================ ANCHORED RESULTS ================")
    for name, rec in results.items():
        print(f"  vs {name:<28} {fmt(rec)}", flush=True)
    print("  >50% vs a champion => surpasses it; vs greedy = absolute skill floor.")


if __name__ == "__main__":
    main()
