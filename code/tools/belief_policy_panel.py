"""One isolated, indexed fair policy-evaluation chunk (no search or training)."""

from __future__ import annotations
import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))


def evaluate(candidate, opponent, decks, seed, games, start):
    import torch

    torch.set_num_threads(1)
    from sb3_contrib import MaskablePPO
    from digimon_gym.agents.anchored_eval import play_one
    from digimon_gym.agents.pilot_training import make_agent_opponent_fn
    from digimon_gym.digimon_gym import greedy_policy

    model = MaskablePPO.load(str(candidate), device="cpu")
    opponent_fn = (
        greedy_policy if opponent == "greedy" else make_agent_opponent_fn(str(opponent))
    )
    result = {
        "wins": 0,
        "losses": 0,
        "draws": 0,
        "failed": 0,
        "truncated": 0,
        "games": games,
        "first_seat_games": 0,
        "second_seat_games": 0,
        "game_start": start,
    }
    for i in range(start, start + games):
        result["first_seat_games" if i % 2 == 0 else "second_seat_games"] += 1
        a = decks[(i // 2) % len(decks)]["cards"]
        b = decks[((i // 2) + 1) % len(decks)]["cards"]
        try:
            winner = play_one(
                model,
                opponent_fn,
                a,
                b,
                2 * (seed + i) + (i % 2),
                model.digimon_contract["observation_profile"],
            )
            result["wins" if winner == 1 else "losses" if winner == 2 else "draws"] += 1
        except BaseException as error:
            if isinstance(error, (KeyboardInterrupt, SystemExit)):
                raise
            result["failed"] += 1
            result.setdefault("errors", []).append({"game": i, "error": str(error)})
    return result


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--candidate", type=Path, required=True)
    p.add_argument("--opponent", required=True)
    p.add_argument("--decks", type=Path, required=True)
    p.add_argument("--seed", type=int, required=True)
    p.add_argument("--games", type=int, required=True)
    p.add_argument("--game-start", type=int, required=True)
    p.add_argument("--out", type=Path, required=True)
    a = p.parse_args()
    if a.games < 1 or a.game_start < 0:
        p.error("invalid game range")
    result = evaluate(
        a.candidate,
        a.opponent,
        json.loads(a.decks.read_text()),
        a.seed,
        a.games,
        a.game_start,
    )
    from tools.selfplay_scheduler import atomic_json

    atomic_json(a.out, result)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
