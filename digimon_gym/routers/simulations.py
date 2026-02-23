"""Simulation endpoints."""

from __future__ import annotations

from fastapi import APIRouter

from digimon_gym.digimon_gym import GameState, greedy_policy
from digimon_gym.engine.data.deck_loader import parse_deck
from digimon_gym.routers.schemas import SimulationRequest

router = APIRouter(tags=["simulations"])


@router.post("/simulations")
@router.post("/simulate", include_in_schema=False)
def simulate_game(request: SimulationRequest):
    """Run N greedy-policy simulations between two decks."""
    try:
        deck1_list = parse_deck(request.deck1)
    except ValueError:
        deck1_list = request.deck1.split("\n")

    try:
        deck2_list = parse_deck(request.deck2)
    except ValueError:
        deck2_list = request.deck2.split("\n")

    wins_p1 = 0
    wins_p2 = 0
    logs: list[dict[str, str | int]] = []

    for i in range(request.num_simulations):
        game = GameState()
        game.reset(deck1=deck1_list, deck2=deck2_list)
        done = False
        steps = 0

        while not done and steps < 200:
            action = greedy_policy(game)
            _, _, done, _ = game.step(action)
            steps += 1

        winner_id = game.runner.winner_id if game.runner else None
        if winner_id == 1:
            wins_p1 += 1
        elif winner_id == 2:
            wins_p2 += 1

        if i < 5:
            logs.append(
                {
                    "sim_id": i,
                    "steps": steps,
                    "winner": f"Player {winner_id}" if winner_id else "Draw",
                }
            )

    return {
        "p1_win_rate": wins_p1 / request.num_simulations,
        "p2_win_rate": wins_p2 / request.num_simulations,
        "draw_rate": (request.num_simulations - wins_p1 - wins_p2) / request.num_simulations,
        "logs": logs,
    }

