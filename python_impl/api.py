from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from typing import List, Dict, Any
from python_impl.digimon_gym import GameState
import random

app = FastAPI()

class SimulationRequest(BaseModel):
    deck1: str
    deck2: str
    num_simulations: int = 100

@app.post("/simulate")
def simulate_game(request: SimulationRequest):
    """
    Simulates games between two decks.
    """
    deck1_list = request.deck1.split('\n')
    deck2_list = request.deck2.split('\n')

    wins_p1 = 0
    wins_p2 = 0
    draws = 0
    logs = []

    # Run simulations
    for i in range(request.num_simulations):
        game = GameState()
        obs = game.reset(deck1=deck1_list, deck2=deck2_list)
        done = False
        steps = 0
        game_log = []

        # Simple simulation loop
        while not done and steps < 200:
            # Random action placeholder
            action = random.randint(0, 10)
            obs, reward, done = game.step(action)
            steps += 1

        # Determine winner based on GameState (scaffolded)
        # In a real game, GameState would report who won.
        # Here we just randomize or use steps to decide for demo.
        winner = random.choice([0, 1]) # 0 for P1, 1 for P2

        if winner == 0:
            wins_p1 += 1
        else:
            wins_p2 += 1

        if i < 5: # Keep logs for first 5 games
            logs.append({
                "sim_id": i,
                "steps": steps,
                "winner": f"Player {winner + 1}"
            })

    return {
        "p1_win_rate": wins_p1 / request.num_simulations,
        "p2_win_rate": wins_p2 / request.num_simulations,
        "draw_rate": draws / request.num_simulations,
        "logs": logs
    }

@app.get("/")
def health_check():
    return {"status": "ok"}
