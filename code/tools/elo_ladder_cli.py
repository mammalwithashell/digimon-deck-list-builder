"""Elo ladder CLI — rank a run's checkpoints + frozen champions + greedy on one
comparable scale via a seat-balanced round-robin.

The greedy anchor pins the scale (its rating is held fixed). The matchup matrix
and upset list surface forgetting/cycling (a later checkpoint losing to an
earlier one) that the scalar ratings alone would hide.

Example:
    python code/tools/elo_ladder_cli.py \
        --run cloud_downloads/v022-hf4zm2hl82qk48/models/pilot_ppo_20260531_073321 \
        --champions models/champions/registry.json \
        --deck-pool-snapshot cloud_downloads/.../deck_pool_snapshot.json \
        --n 20 --device cpu
"""
from __future__ import annotations

import argparse
import glob
import json
import re
from pathlib import Path

from digimon_gym.agents.champion_registry import ChampionRegistry
from digimon_gym.agents.elo_ladder import (
    PlayerSpec, compute_elo, elo_standard_errors, find_upsets, run_round_robin,
)
from digimon_gym.agents.gauntlet import GeneralistDeckPool, load_generalist_deck_pool
from digimon_gym.digimon_gym import greedy_policy
from digimon_gym.agents.pilot_training import MaskableRecurrentPPO, make_agent_opponent_fn

DEFAULT_STARTERS = [
    "ST-1 Gaia Red", "ST-2 Cocytus Blue", "ST-3 Heaven's Yellow",
    "ST-4 Giga Green", "ST-5 Machine Black", "ST-6 Venomous Violet",
]
DEFAULT_HASH = "sha256:a20462fbfede51ad3b1585291ea1ec1259b1a6b9c6156cff033db5f9f1ead39e"


def run_layout_hash(run_dir: Path, override: str | None) -> str:
    if override:
        return override
    for m in glob.glob(str(run_dir / "*.meta.json")) + glob.glob(str(run_dir.parent / "*.meta.json")):
        try:
            return str(json.loads(Path(m).read_text()).get("tensor_layout_hash") or DEFAULT_HASH)
        except Exception:
            pass
    return DEFAULT_HASH


def checkpoint_players(run_dir: Path, layout_hash: str, max_ckpts: int | None) -> list[PlayerSpec]:
    cks = glob.glob(str(run_dir / "checkpoints" / "step_*.zip"))
    cks.sort(key=lambda c: int(re.search(r"step_0*(\d+)\.zip", c).group(1)))
    if max_ckpts:
        cks = cks[-max_ckpts:]
    out = []
    for c in cks:
        step = int(re.search(r"step_0*(\d+)\.zip", c).group(1))
        out.append(PlayerSpec(name=f"step_{step//1000}k", kind="model", weights_path=c,
                              algorithm="lstm", layout_hash=layout_hash, order=step))
    return out


def main() -> None:
    ap = argparse.ArgumentParser(description="Elo ladder over checkpoints + champions + greedy.")
    ap.add_argument("--run", default=None, help="run dir containing checkpoints/")
    ap.add_argument("--champions", default="models/champions/registry.json")
    ap.add_argument("--no-greedy", action="store_true")
    ap.add_argument("--deck-pool-snapshot", default=None)
    ap.add_argument("--archetypes", default=None)
    ap.add_argument("--n", type=int, default=20, help="seat-balanced games per pair")
    ap.add_argument("--seed", type=int, default=777)
    ap.add_argument("--profile", default="standard_lite_deck_v2")
    ap.add_argument("--device", default="cpu", help="auto|cpu|cuda (cpu avoids VRAM pressure)")
    ap.add_argument("--layout-hash", default=None)
    ap.add_argument("--max-checkpoints", type=int, default=None)
    ap.add_argument("--anchor-rating", type=float, default=1000.0)
    ap.add_argument("--min-games", type=int, default=12, help="below this a rating is flagged provisional")
    ap.add_argument("--out", default=None,
                    help="write the ladder JSON here (defaults to <run>/elo_ladder.json)")
    args = ap.parse_args()

    lhash = run_layout_hash(Path(args.run), args.layout_hash) if args.run else (args.layout_hash or DEFAULT_HASH)

    players: list[PlayerSpec] = []
    if args.run:
        players += checkpoint_players(Path(args.run), lhash, args.max_checkpoints)
    registry = ChampionRegistry.load(args.champions)
    for champ in registry.compatible(lhash):
        players.append(PlayerSpec(name=champ.name, kind="model", weights_path=champ.weights_path,
                                  algorithm=champ.algorithm, layout_hash=champ.tensor_layout_hash))
    if not args.no_greedy:
        players.append(PlayerSpec(name="greedy", kind="greedy"))

    if sum(1 for p in players if p.kind == "model") < 1:
        raise SystemExit("need at least one model player (a --run with checkpoints, or champions)")

    if args.deck_pool_snapshot:
        from digimon_engine import load_implemented_card_ids
        pool = GeneralistDeckPool.from_snapshot(args.deck_pool_snapshot,
                                                implemented_card_ids=load_implemented_card_ids())
    else:
        arch = [a.strip() for a in args.archetypes.split(",")] if args.archetypes else DEFAULT_STARTERS
        pool = load_generalist_deck_pool(allowed_archetypes=set(arch))

    print(f"players ({len(players)}): " + ", ".join(p.name for p in players))
    print(f"layout {lhash[:22]}…  n={args.n}/pair  seed={args.seed}  device={args.device}\n", flush=True)

    model_cache: dict = {}
    oppfn_cache: dict = {}

    def load_model(spec: PlayerSpec):
        if spec.name not in model_cache:
            model_cache[spec.name] = MaskableRecurrentPPO.load(spec.weights_path, device=args.device)
        return model_cache[spec.name]

    def opponent_fn_for(spec: PlayerSpec):
        if spec.kind == "greedy":
            return greedy_policy
        if spec.name not in oppfn_cache:
            oppfn_cache[spec.name] = make_agent_opponent_fn(spec.weights_path, algorithm=spec.algorithm)
        return oppfn_cache[spec.name]

    cells = run_round_robin(players, pool, args.n, args.seed, args.profile,
                            load_model, opponent_fn_for, log=lambda s: print(s, flush=True))

    anchor = None if args.no_greedy else "greedy"
    ratings = compute_elo(cells, anchor=anchor, anchor_rating=args.anchor_rating)
    ses = elo_standard_errors(cells, ratings)
    games_per = {p.name: 0 for p in players}
    for c in cells:
        games_per[c.a_name] += c.games
        games_per[c.b_name] += c.games

    print("\n================ ELO LADDER ================")
    for name in sorted(ratings, key=lambda n: ratings[n], reverse=True):
        se = ses[name]
        flag = "  (provisional)" if games_per.get(name, 0) < args.min_games else ""
        se_s = "±  inf" if se == float("inf") else f"±{se:5.0f}"
        print(f"  {name:<24} {ratings[name]:7.0f} {se_s}  games={games_per.get(name,0):4d}{flag}")

    print("\n---- matchup matrix (row's score vs col, %) ----")
    score = {}
    for c in cells:
        score[(c.a_name, c.b_name)] = c.a_score
        score[(c.b_name, c.a_name)] = 1.0 - c.a_score
    names = [p.name for p in players]
    hdr = " " * 14 + "".join(f"{n[:8]:>9}" for n in names)
    print(hdr)
    for r in names:
        row = f"  {r[:12]:<12}"
        for cc in names:
            row += "    --   " if r == cc else (f"{100*score[(r,cc)]:7.0f}% " if (r, cc) in score else "    .    ")
        print(row)

    order = {p.name: p.order for p in players if p.order is not None}
    upsets = find_upsets(cells, order)
    if upsets:
        print("\n---- forgetting/cycling (later checkpoint loses to earlier) ----")
        for u in upsets:
            print(f"  {u.later} lost to {u.earlier}  ({100*u.later_score:.0f}%)")
    elif order:
        print("\n  no forgetting detected among checkpoints (each beats its earlier selves).")

    # Persist the ladder so the training MCP / later analysis can read it.
    out_path = Path(args.out) if args.out else (Path(args.run) / "elo_ladder.json" if args.run else None)
    if out_path is not None:
        result = {
            "layout_hash": lhash,
            "n_per_pair": args.n,
            "seed": args.seed,
            "anchor": anchor,
            "ratings": {n: round(ratings[n], 1) for n in ratings},
            "standard_errors": {n: (None if ses[n] == float("inf") else round(ses[n], 1)) for n in ses},
            "games": games_per,
            "matchups": [
                {"a": c.a_name, "b": c.b_name, "a_wins": c.a_wins,
                 "b_wins": c.b_wins, "draws": c.draws} for c in cells
            ],
            "upsets": [{"later": u.later, "earlier": u.earlier,
                        "later_score": round(u.later_score, 3)} for u in upsets],
        }
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(json.dumps(result, indent=2), encoding="utf-8")
        print(f"\n  ladder written: {out_path}")


if __name__ == "__main__":
    main()
