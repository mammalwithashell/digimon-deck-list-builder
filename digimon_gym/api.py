from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from typing import List, Dict, Any, Union
from uuid import uuid4
import numpy as np
import json

from digimon_gym.digimon_gym import GameState, greedy_policy
from digimon_gym.engine.runners.headless_game import HeadlessGame
from digimon_gym.engine.runners.interactive_game import InteractiveGame
from digimon_gym.engine.data.enums import PlayerType

app = FastAPI()

# ─── Game Session Storage ─────────────────────────────────────────────
active_games: Dict[str, Union[HeadlessGame, InteractiveGame]] = {}

# Global game history buffer for training data collection
game_history = []

# ─── Request/Response Models ──────────────────────────────────────────

class SimulationRequest(BaseModel):
    deck1: str
    deck2: str
    num_simulations: int = 100

class CreateGameRequest(BaseModel):
    deck1: List[str]
    deck2: List[str]
    player1_type: str = "agent"  # "agent" or "human"
    player2_type: str = "agent"

class GameActionRequest(BaseModel):
    action: int

# ─── Health Check ─────────────────────────────────────────────────────

@app.get("/")
def health_check():
    return {"status": "ok"}

# ─── Simulation Endpoint ─────────────────────────────────────────────

@app.post("/simulate")
def simulate_game(request: SimulationRequest):
    """Simulates games between two decks using greedy policy."""
    deck1_list = request.deck1.split('\n')
    deck2_list = request.deck2.split('\n')

    wins_p1 = 0
    wins_p2 = 0
    logs = []

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
            logs.append({
                "sim_id": i,
                "steps": steps,
                "winner": f"Player {winner_id}" if winner_id else "Draw"
            })

    return {
        "p1_win_rate": wins_p1 / request.num_simulations,
        "p2_win_rate": wins_p2 / request.num_simulations,
        "draw_rate": (request.num_simulations - wins_p1 - wins_p2) / request.num_simulations,
        "logs": logs
    }

# ─── Game Session Endpoints ──────────────────────────────────────────

@app.post("/game/create")
def create_game(request: CreateGameRequest):
    """Create a new game session. Returns game_id and initial state."""
    game_id = str(uuid4())
    p1_type = PlayerType.Human if request.player1_type.lower() == "human" else PlayerType.Agent
    p2_type = PlayerType.Human if request.player2_type.lower() == "human" else PlayerType.Agent

    if p1_type == PlayerType.Agent and p2_type == PlayerType.Agent:
        runner = HeadlessGame(request.deck1, request.deck2, verbose=True)
    else:
        runner = InteractiveGame(request.deck1, request.deck2, p1_type, p2_type)

    active_games[game_id] = runner
    state = runner.game.to_json()
    mask = runner.get_action_mask().tolist()

    return {
        "game_id": game_id,
        "state": state,
        "action_mask": mask,
    }

@app.post("/game/{game_id}/action")
def game_action(game_id: str, request: GameActionRequest):
    """Execute an action in a game session."""
    runner = active_games.get(game_id)
    if not runner:
        raise HTTPException(status_code=404, detail="Game not found")

    # Record state before action for training data
    current_player_id = runner.game.current_player_id
    tensor = runner.get_board_tensor(current_player_id) if isinstance(runner, HeadlessGame) else None

    runner.step(request.action)
    state = runner.game.to_json()
    mask = runner.get_action_mask().tolist()

    result = {
        "state": state,
        "action_mask": mask,
        "is_game_over": runner.is_game_over,
    }

    # Include logs if interactive
    if isinstance(runner, InteractiveGame):
        result["logs"] = runner.get_last_log()
        runner.clear_log()

    # Save training data on game over
    if runner.is_game_over and tensor is not None:
        global game_history
        game_history.append({
            "state": tensor.tolist(),
            "action": request.action,
            "player": current_player_id
        })
        winner_id = runner.winner_id
        if winner_id is not None:
            data_entry = {
                "winner": winner_id,
                "history": game_history
            }
            try:
                with open("training_data.json", "a") as f:
                    f.write(json.dumps(data_entry) + "\n")
            except Exception:
                pass
        game_history = []

    return result

@app.post("/game/{game_id}/step")
def game_step(game_id: str):
    """Advance interactive game (runs agent turns, pauses on human).

    For interactive games: auto-plays agent turns, pauses when it's human's turn.
    For headless games: returns 400 (use /action instead).
    """
    runner = active_games.get(game_id)
    if not runner:
        raise HTTPException(status_code=404, detail="Game not found")

    if not isinstance(runner, InteractiveGame):
        raise HTTPException(status_code=400, detail="Step is only for interactive games. Use /action for headless games.")

    state = runner.run_step()
    mask = runner.get_action_mask().tolist()
    logs = runner.get_last_log()
    runner.clear_log()

    return {
        "state": state,
        "action_mask": mask,
        "logs": logs,
        "is_human_turn": runner.is_current_player_human(),
        "is_game_over": runner.is_game_over,
    }

@app.get("/game/{game_id}/state")
def game_state(game_id: str):
    """Get current game state."""
    runner = active_games.get(game_id)
    if not runner:
        raise HTTPException(status_code=404, detail="Game not found")
    return runner.game.to_json()

@app.get("/game/{game_id}/mask")
def game_mask(game_id: str):
    """Get current action mask."""
    runner = active_games.get(game_id)
    if not runner:
        raise HTTPException(status_code=404, detail="Game not found")
    return {"action_mask": runner.get_action_mask().tolist()}

@app.get("/game/{game_id}/log")
def game_log(game_id: str):
    """Get and clear game log."""
    runner = active_games.get(game_id)
    if not runner:
        raise HTTPException(status_code=404, detail="Game not found")
    if isinstance(runner, InteractiveGame):
        logs = runner.get_last_log()
        runner.clear_log()
        return {"logs": logs}
    return {"logs": []}

@app.delete("/game/{game_id}")
def delete_game(game_id: str):
    """Delete a game session."""
    if game_id in active_games:
        del active_games[game_id]
    return {"status": "deleted"}
