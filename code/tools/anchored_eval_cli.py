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
import hashlib
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
from sb3_contrib import MaskablePPO


def _load_candidate(cand_zip: str, device: str):
    """Load a candidate checkpoint regardless of algorithm. The starter
    curriculum produces MLP (MaskablePPO) floors while champions are LSTM
    (MaskableRecurrentPPO); the recurrent loader raises on an MLP zip
    (use_sde kwarg), so fall back to the MLP loader."""
    try:
        m = MaskableRecurrentPPO.load(cand_zip, device=device)
        print("candidate algo: lstm (MaskableRecurrentPPO)")
        return m
    except Exception:
        m = MaskablePPO.load(cand_zip, device=device)
        print("candidate algo: mlp (MaskablePPO)")
        return m

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
    ap.add_argument("--profile", default=None)
    ap.add_argument("--device", default="auto", help="auto|cpu|cuda")
    ap.add_argument("--layout-hash", default=None, help="override candidate layout hash")
    ap.add_argument("--no-greedy", action="store_true")
    ap.add_argument("--card-eval-dir", type=Path, help="atomic per-card evaluation reports")
    ap.add_argument("--checkpoint-step", type=int, default=0)
    ap.add_argument("--eval-window", type=int, default=0)
    ap.add_argument("--eval-panel", default="validation")
    ap.add_argument("--eval-context", type=Path, help="extra scheduling provenance JSON")
    ap.add_argument("--deck-pool-snapshot", default=None,
                    help="frozen deck_pool_snapshot.json (preferred — reproducible, "
                         "and the exact decks a run trained on)")
    ap.add_argument("--archetypes", default=None,
                    help="comma-separated archetypes from deck_library.json "
                         "(fallback if no snapshot); default = 6 starters")
    args = ap.parse_args()

    cand_zip = resolve_candidate(args.candidate)
    candidate = _load_candidate(cand_zip, args.device)
    from model_contract import validate_checkpoint
    contract = validate_checkpoint(candidate, args.profile)
    args.profile = contract["observation_profile"]
    lhash = contract["tensor_layout_hash"]
    if args.layout_hash is not None and args.layout_hash != lhash:
        raise ValueError("--layout-hash differs from candidate's validated training contract")
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

    registry = ChampionRegistry.load(args.champions)

    collector = None
    if args.card_eval_dir:
        import digimon_engine
        from digimon_gym.agents.card_eval import CardEvalCollector, save_report
        if getattr(digimon_engine, "CARD_EVAL_EVENT_VERSION", 0) < 1:
            raise RuntimeError("Card evaluation requires CARD_EVAL_EVENT_VERSION >= 1; rebuild the Rust wheel")
        db = digimon_engine.CardDatabase()
        collector = CardEvalCollector(args.checkpoint_step, args.eval_window, db.get_card)
        collector.provenance = json.loads(args.eval_context.read_text(encoding="utf-8")) if args.eval_context else {}
        collector.provenance.update(contract, evaluation_mode="policy_only", panel=args.eval_panel,
            candidate_sha256=hashlib.sha256(Path(cand_zip).read_bytes()).hexdigest(),
            card_eval_event_version=digimon_engine.CARD_EVAL_EVENT_VERSION,
            base_seed=args.seed, requested_games_per_anchor=args.n,
            snapshot_sha256=hashlib.sha256(Path(args.deck_pool_snapshot).read_bytes()).hexdigest() if args.deck_pool_snapshot else None,
            anchors_sha256=hashlib.sha256(Path(args.champions).read_bytes()).hexdigest(),
            anchor_model_hashes={champ.name: hashlib.sha256(Path(champ.weights_path).read_bytes()).hexdigest()
                for champ in registry.compatible(lhash)},
            deterministic=True, status="running")
    try:
        results = evaluate_against_anchors(
            candidate, lhash, registry, pool,
            n=args.n, base_seed=args.seed, tensor_profile=args.profile,
            include_greedy=not args.no_greedy,
            **({"collector": collector} if collector is not None else {}),
        )
        if collector is not None:
            collector.provenance["status"] = "complete"
    except BaseException as error:
        if collector is not None:
            collector.provenance.update(status="failed", error=str(error))
        raise
    finally:
        if collector is not None:
            collector.finish_game(None, "evaluation_end")
            path = save_report(collector.report(), args.card_eval_dir)
            print(f"card evaluation report: {path}", flush=True)

    print("================ ANCHORED RESULTS ================")
    for name, rec in results.items():
        print(f"  vs {name:<28} {fmt(rec)}", flush=True)
    print("  >50% vs a champion => surpasses it; vs greedy = absolute skill floor.")


if __name__ == "__main__":
    main()
