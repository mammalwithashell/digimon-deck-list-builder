"""Seat-balanced policy-only head-to-head between two SB3 checkpoints (or vs greedy).

Mirrors the cloud ``final_policy_eval`` adjacent panel locally: deterministic
masked ``predict`` on both sides, no search, four-game blocks per ordered
archetype pair (both deck assignments x both opening orders), scored from the
candidate's perspective (win 1, draw 0.5). The 95% interval resamples complete
blocks, never correlated single games. Any incomplete game fails the run; it is
never scored as a draw.

Deck pool: the selfplay ``decks.json`` shape (``[{"name": "<Archetype>:...", "cards": [...]}, ...]``);
the archetype is the name's first ``:`` field and list variants rotate per block.

CLI:
    python code/tools/policy_head_to_head.py --candidate child.zip --opponent parent.zip \
        --decks decks.json --out result.json [--blocks-per-pair 1] [--seed 27102026]
    python code/tools/policy_head_to_head.py --candidate child.zip --opponent greedy ...
"""

from __future__ import annotations

import argparse
import itertools
import json
import sys
import time
from collections import defaultdict
from pathlib import Path

import numpy as np

REPO_ROOT = Path(__file__).resolve().parents[2]
for entry in (REPO_ROOT / "code", REPO_ROOT / "code" / "digimon_gym" / "agents"):
    if str(entry) not in sys.path:
        sys.path.insert(0, str(entry))


def load_pool(path: Path) -> dict[str, list[list[str]]]:
    pool: dict[str, list[list[str]]] = defaultdict(list)
    for entry in json.loads(path.read_text(encoding="utf-8")):
        name = entry["name"] if isinstance(entry, dict) else None
        cards = entry["cards"] if isinstance(entry, dict) else entry
        archetype = name.split(":")[0] if name else f"deck-{len(pool)}"
        pool[archetype].append(list(cards))
    if len(pool) < 2:
        raise ValueError("need at least two archetypes")
    return dict(sorted(pool.items()))


def schedule(pool: dict[str, list[list[str]]], blocks_per_pair: int, seed: int) -> list[dict]:
    """Frozen game list. Block = ordered archetype pair x repetition; four games
    per block: candidate on A or B, opening seed parity 0 or 1."""
    names = list(pool)
    rows = []
    block = 0
    for a, b in itertools.permutations(names, 2):
        for rep in range(blocks_per_pair):
            variant_a = pool[a][(block + rep) % len(pool[a])]
            variant_b = pool[b][(block // len(pool[a]) + rep) % len(pool[b])]
            for swap in range(2):
                for parity in range(2):
                    cand_arch, opp_arch = (b, a) if swap else (a, b)
                    cand_deck, opp_deck = (variant_b, variant_a) if swap else (variant_a, variant_b)
                    rows.append(dict(index=len(rows), block=block, candidate_archetype=cand_arch,
                                     opponent_archetype=opp_arch, candidate_deck=cand_deck,
                                     opponent_deck=opp_deck,
                                     seed=2 * (seed + len(rows)) + parity))
            block += 1
    return rows


def block_interval(scores: list[float], blocks: list[int], seed: int = 997, samples: int = 10000):
    groups = defaultdict(list)
    for s, b in zip(scores, blocks):
        groups[b].append(s)
    means = np.array([np.mean(v) for v in groups.values()])
    rng = np.random.default_rng(seed)
    draws = rng.choice(means, size=(samples, len(means)), replace=True).mean(axis=1)
    return float(means.mean()), [float(np.percentile(draws, 2.5)), float(np.percentile(draws, 97.5))]


def opponent_fn_for(spec: str, profile: str):
    from digimon_gym.digimon_gym import greedy_policy

    if spec == "greedy":
        return greedy_policy
    from digimon_gym.agents.pilot_training import make_agent_opponent_fn

    return make_agent_opponent_fn(spec, algorithm="mlp")


def run(args) -> dict:
    from sb3_contrib import MaskablePPO

    from digimon_gym.agents.anchored_eval import play_one
    from digimon_gym.digimon_gym import NoOpLoopError
    from model_contract import validate_checkpoint

    candidate = MaskablePPO.load(str(args.candidate), device="cpu")
    contract = validate_checkpoint(candidate)
    profile = contract["observation_profile"]
    opponent = opponent_fn_for(str(args.opponent), profile)
    pool = load_pool(args.decks)
    rows = schedule(pool, args.blocks_per_pair, args.seed)
    if args.limit:
        rows = rows[: args.limit]
    started = time.perf_counter()
    games = []
    incomplete = []
    for row in rows:
        try:
            winner = play_one(candidate, opponent, row["candidate_deck"], row["opponent_deck"],
                              row["seed"], profile)
        except (NoOpLoopError, RuntimeError) as exc:
            # An engine fault (mask/execution mismatch, safety cap) is evidence about
            # the engine, not about either policy: it is never scored, only counted.
            if args.on_engine_error != "skip":
                raise
            incomplete.append(dict(index=row["index"], block=row["block"], seed=row["seed"],
                                   candidate_archetype=row["candidate_archetype"],
                                   opponent_archetype=row["opponent_archetype"], error=str(exc)))
            print(f"game {row['index']} incomplete (engine fault): {str(exc)[:160]}", flush=True)
            continue
        score = 1.0 if winner == 1 else 0.0 if winner == 2 else 0.5
        games.append(dict(index=row["index"], block=row["block"], winner=winner, score=score,
                          candidate_archetype=row["candidate_archetype"],
                          opponent_archetype=row["opponent_archetype"], seed=row["seed"]))
        if args.progress and len(games) % args.progress == 0:
            print(f"{len(games)}/{len(rows)} games, score so far {np.mean([g['score'] for g in games]):.3f}",
                  flush=True)
    scores = [g["score"] for g in games]
    mean, interval = block_interval(scores, [g["block"] for g in games])
    per_arch = {}
    for arch in pool:
        mine = [g["score"] for g in games if g["candidate_archetype"] == arch]
        per_arch[arch] = dict(games=len(mine), score=float(np.mean(mine)) if mine else None)
    return dict(candidate=str(args.candidate), opponent=str(args.opponent), decks=str(args.decks),
                profile=profile, seed=args.seed, blocks_per_pair=args.blocks_per_pair,
                games=len(games), wins=sum(g["winner"] == 1 for g in games),
                losses=sum(g["winner"] == 2 for g in games),
                draws=sum(g["winner"] not in (1, 2) for g in games),
                incomplete=len(incomplete), incomplete_games=incomplete,
                score=mean, score_ci95=interval, per_candidate_archetype=per_arch,
                elapsed_seconds=time.perf_counter() - started, game_log=games,
                scope="deterministic masked policy, no search; four-game blocks per ordered "
                      "archetype pair; interval resamples blocks")


def main(argv=None) -> int:
    p = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    p.add_argument("--candidate", type=Path, required=True, help="SB3 .zip scored from its perspective")
    p.add_argument("--opponent", required=True, help="SB3 .zip or `greedy`")
    p.add_argument("--decks", type=Path, required=True)
    p.add_argument("--out", type=Path, required=True)
    p.add_argument("--blocks-per-pair", type=int, default=1)
    p.add_argument("--seed", type=int, default=27102026)
    p.add_argument("--limit", type=int, default=0, help="debug: play only the first N scheduled games")
    p.add_argument("--progress", type=int, default=50)
    p.add_argument("--on-engine-error", choices=["fail", "skip"], default="fail",
                   help="skip: record an engine-faulted game as incomplete (unscored) and continue; "
                        "set DIGIMON_EVAL_FAILURE_DIR to keep each fault's recording")
    args = p.parse_args(argv)
    result = run(args)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(result, indent=2), encoding="utf-8")
    summary = {k: result[k] for k in ("candidate", "opponent", "games", "wins", "losses", "draws", "incomplete",
                                      "score", "score_ci95")}
    print(json.dumps(summary), flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
