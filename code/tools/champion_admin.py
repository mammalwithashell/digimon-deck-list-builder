"""Champion registry admin — list champions, or gate-promote a candidate.

Promotion plays the candidate against the current champion panel (seat-balanced)
and registers it only if its aggregate win rate clears the gate (default 55%,
the AlphaGo-Zero rule). ``--force`` registers without gating (manual blessing).

``emit-pool`` derives an OpponentPool manifest from the registry (uniform
weights, filtered to one tensor-layout cohort) so a new run can train against
all promoted champions — the pool-based fictitious-self-play loop
(`harden-training-pipeline` D3).

Examples:
    python code/tools/champion_admin.py list
    python code/tools/champion_admin.py promote --candidate run/final.zip --name v23 \
        --deck-pool-snapshot run/deck_pool_snapshot.json --n 40
    python code/tools/champion_admin.py promote --candidate m.zip --name v0 --force
    python code/tools/champion_admin.py emit-pool --out pool.json
"""
from __future__ import annotations

import argparse
import glob
import json
from pathlib import Path

from digimon_gym.agents.champion_registry import (
    Champion, ChampionRegistry, should_promote, PROMOTION_THRESHOLD,
)

DEFAULT_HASH = "sha256:a20462fbfede51ad3b1585291ea1ec1259b1a6b9c6156cff033db5f9f1ead39e"
DEFAULT_STARTERS = [
    "ST-1 Gaia Red", "ST-2 Cocytus Blue", "ST-3 Heaven's Yellow",
    "ST-4 Giga Green", "ST-5 Machine Black", "ST-6 Venomous Violet",
]


def _candidate_meta(candidate: str) -> dict:
    for cand in (Path(candidate).with_suffix(".meta.json"),
                 Path(candidate).parent.parent / "*.meta.json"):
        for m in glob.glob(str(cand)):
            try:
                return json.loads(Path(m).read_text())
            except Exception:
                pass
    return {}


def cmd_list(args) -> None:
    reg = ChampionRegistry.load(args.champions)
    if not reg.champions:
        print("(no champions registered)")
        return
    for c in reg.champions:
        print(f"  {c.name:<24} {c.algorithm}  {c.observation_profile}  "
              f"{c.tensor_layout_hash[:22]}  {c.created_at}  <- {c.source}")


def cmd_promote(args) -> None:
    reg = ChampionRegistry.load(args.champions)
    meta = _candidate_meta(args.candidate)
    champ = Champion.from_meta(args.name, args.candidate, meta,
                              source=args.source or "manual",
                              created_at=args.created_at or "")
    if not champ.tensor_layout_hash:
        champ.tensor_layout_hash = args.layout_hash or DEFAULT_HASH
    if not champ.observation_profile:
        champ.observation_profile = args.profile

    if args.force:
        reg.register(champ)
        reg.save()
        print(f"force-registered champion {champ.name!r}")
        return

    panel = reg.compatible(champ.tensor_layout_hash)
    if not panel:
        print(f"no compatible champions to gate against; use --force to seed the panel.")
        return

    # Gate: candidate vs the panel, seat-balanced.
    from digimon_gym.agents.anchored_eval import play_matchup
    from digimon_gym.agents.pilot_training import MaskableRecurrentPPO, make_agent_opponent_fn
    from digimon_gym.agents.gauntlet import GeneralistDeckPool, load_generalist_deck_pool

    if args.deck_pool_snapshot:
        from digimon_engine import load_implemented_card_ids
        pool = GeneralistDeckPool.from_snapshot(args.deck_pool_snapshot,
                                                implemented_card_ids=load_implemented_card_ids())
    else:
        arch = [a.strip() for a in args.archetypes.split(",")] if args.archetypes else DEFAULT_STARTERS
        pool = load_generalist_deck_pool(allowed_archetypes=set(arch))

    candidate = MaskableRecurrentPPO.load(args.candidate, device=args.device)
    total_wins = total_games = 0
    print(f"gating {champ.name!r} vs {len(panel)} champion(s), n={args.n} each:")
    for opp in panel:
        opp_fn = make_agent_opponent_fn(opp.weights_path, algorithm=opp.algorithm)
        rec = play_matchup(candidate, opp_fn, pool, args.n, args.seed, args.profile)
        total_wins += rec.wins
        total_games += rec.games
        print(f"  vs {opp.name:<24} {rec.wins}-{rec.losses}-{rec.draws}  WR={100*rec.win_rate:.1f}%")

    wr = total_wins / total_games if total_games else 0.0
    promoted = should_promote(total_wins, total_games, args.threshold)
    print(f"panel win rate: {100*wr:.1f}%  gate={100*args.threshold:.0f}%  -> "
          f"{'PROMOTE' if promoted else 'reject'}")
    if promoted:
        reg.register(champ)
        reg.save()
        print(f"registered champion {champ.name!r}")


def cmd_emit_pool(args) -> None:
    from digimon_gym.agents.opponent_pool import OpponentPool

    layout_hash = args.layout_hash or DEFAULT_HASH
    pool = OpponentPool.from_champion_registry(
        args.champions, layout_hash, manifest_path=args.out
    )
    pool.save()
    print(f"wrote {pool.size}-champion opponent pool to {args.out} "
          f"(layout {layout_hash[:22]}…)")
    for entry in pool.entries:
        print(f"  {entry.name:<24} {entry.algorithm}  {entry.weights_path}")
    print("train with: --opponent pool --opponent-pool-manifest "
          f"{args.out}")


def main() -> None:
    ap = argparse.ArgumentParser(description="Champion registry admin.")
    ap.add_argument("--champions", default="models/champions/registry.json")
    sub = ap.add_subparsers(dest="cmd", required=True)

    sub.add_parser("list")

    ep = sub.add_parser(
        "emit-pool",
        help="derive an OpponentPool manifest from the champion registry",
    )
    ep.add_argument("--out", required=True, help="manifest JSON output path")
    ep.add_argument("--layout-hash", default=None,
                    help="tensor-layout cohort (default: the lite-deck-v2 hash)")

    p = sub.add_parser("promote")
    p.add_argument("--candidate", required=True)
    p.add_argument("--name", required=True)
    p.add_argument("--source", default="")
    p.add_argument("--created-at", default="")
    p.add_argument("--force", action="store_true", help="register without gating")
    p.add_argument("--threshold", type=float, default=PROMOTION_THRESHOLD)
    p.add_argument("--n", type=int, default=40)
    p.add_argument("--seed", type=int, default=777)
    p.add_argument("--profile", default="standard_lite_deck_v2")
    p.add_argument("--device", default="auto")
    p.add_argument("--layout-hash", default=None)
    p.add_argument("--deck-pool-snapshot", default=None)
    p.add_argument("--archetypes", default=None)

    args = ap.parse_args()
    handlers = {"list": cmd_list, "promote": cmd_promote, "emit-pool": cmd_emit_pool}
    handlers[args.cmd](args)


if __name__ == "__main__":
    main()
