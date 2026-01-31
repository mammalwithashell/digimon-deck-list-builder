from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from typing import List, Dict, Any
from python_impl.digimon_gym import GameState, greedy_policy
from python_impl.csharp_wrapper import CSharpGameWrapper
import random
import numpy as np
import json

app = FastAPI()

# Global debug game instance
active_debug_game: CSharpGameWrapper = None

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
            # Use Greedy Policy or Random Valid Action
            # action = random.randint(0, 50) # Old way

            action = greedy_policy(game)

            # Step returns 4 values now
            obs, reward, done, info = game.step(action)
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

@app.get("/debug/state")
def get_debug_state():
    """
    Returns the current state of the debug game in JSON format.
    Initializes a new game if one doesn't exist.
    """
    global active_debug_game
    if active_debug_game is None:
        # Create dummy decks for debugging
        dummy_deck = ["DebugCard_001"] * 50
        active_debug_game = CSharpGameWrapper(dummy_deck, dummy_deck)

    state_json = active_debug_game.get_state_json()
    return json.loads(state_json)

@app.post("/debug/reset")
def reset_debug_game():
    """
    Resets the debug game instance to a fresh state.
    """
    global active_debug_game
    # Create dummy decks for debugging
    dummy_deck = ["DebugCard_001"] * 50
    active_debug_game = CSharpGameWrapper(dummy_deck, dummy_deck)

    state_json = active_debug_game.get_state_json()
    return {"status": "reset", "state": json.loads(state_json)}

class ActionRequest(BaseModel):
    action: int

@app.post("/action")
def perform_action(request: ActionRequest):
    """
    Executes an action in the debug game and returns the new state.
    """
    global active_debug_game
    if active_debug_game is None:
        dummy_deck = ["DebugCard_001"] * 50
        active_debug_game = CSharpGameWrapper(dummy_deck, dummy_deck)

    active_debug_game.step(request.action)

    state_json = active_debug_game.get_state_json()
    return {"status": "success", "state": json.loads(state_json)}
