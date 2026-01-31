from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from typing import List, Dict, Any, Optional
from python_impl.digimon_gym import GameState, greedy_policy
from python_impl.engine.data.enums import GamePhase
import random
import numpy as np
import uuid
from fastapi.middleware.cors import CORSMiddleware

app = FastAPI()

# Allow CORS
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

class SimulationRequest(BaseModel):
    deck1: str
    deck2: str
    num_simulations: int = 100

class ActionRequest(BaseModel):
    action: int

# Session Management
class SessionManager:
    def __init__(self):
        self.sessions: Dict[str, GameState] = {}

    def create_session(self) -> str:
        session_id = str(uuid.uuid4())
        game_state = GameState()
        # Ensure game is started
        game_state.reset()
        self.sessions[session_id] = game_state
        return session_id

    def get_session(self, session_id: str) -> Optional[GameState]:
        return self.sessions.get(session_id)

session_manager = SessionManager()

# Serialization Helpers
def serialize_card(card) -> Optional[Dict[str, Any]]:
    if not card:
        return None
    return {
        "id": card.card_id,
        "name": card.card_names[0] if card.card_names else "Unknown",
        "cost": card.get_cost_itself,
        "dp": card.base_dp,
        "level": card.level,
        "is_digimon": card.is_digimon,
        "is_option": card.is_option,
        "is_tamer": card.is_tamer,
        "is_digi_egg": card.is_digi_egg,
        "colors": [c.name for c in card.card_colors] if hasattr(card, 'card_colors') else [],
        "traits": card.card_traits,
        "index": card.card_index if hasattr(card, 'card_index') else -1
    }

def serialize_permanent(perm) -> Dict[str, Any]:
    return {
        "top_card": serialize_card(perm.top_card),
        "sources": [serialize_card(c) for c in perm.digivolution_cards],
        "is_suspended": perm.is_suspended,
        "dp": perm.dp
    }

def serialize_player(player, is_self: bool = False) -> Dict[str, Any]:
    # Hide opponent hand info if needed, but for now we send everything
    # Frontend can choose to hide it. Or we mask it here.
    # To support "cheating" / debugging, we send it all.

    return {
        "name": player.player_name,
        "memory": player.memory,
        "hand": [serialize_card(c) for c in player.hand_cards],
        "battle_area": [serialize_permanent(p) for p in player.battle_area],
        "breeding_area": serialize_permanent(player.breeding_area) if player.breeding_area else None,
        "security_count": len(player.security_cards),
        "trash_count": len(player.trash_cards),
        "trash_top": serialize_card(player.trash_cards[-1]) if player.trash_cards else None,
        "deck_count": len(player.library_cards),
        "digitama_count": len(player.digitama_library_cards)
    }

def serialize_game_state(game_state: GameState) -> Dict[str, Any]:
    game = game_state.game
    return {
        "turn_count": game.turn_count,
        "current_phase": game.current_phase.name,
        "turn_player_id": game.turn_player.player_id,
        "memory": game.memory, # Relative to turn player
        "pending_action": game.pending_action.name,
        "game_over": game.game_over,
        "winner": game.winner.player_name if game.winner else None,
        "player1": serialize_player(game.player1),
        "player2": serialize_player(game.player2),
    }

# New Endpoints
@app.post("/game/start")
def start_game():
    session_id = session_manager.create_session()
    return {"session_id": session_id}

@app.get("/game/{session_id}")
def get_game_state(session_id: str):
    session = session_manager.get_session(session_id)
    if not session:
        raise HTTPException(status_code=404, detail="Session not found")

    return serialize_game_state(session)

@app.post("/game/{session_id}/action")
def perform_action(session_id: str, request: ActionRequest):
    session = session_manager.get_session(session_id)
    if not session:
        raise HTTPException(status_code=404, detail="Session not found")

    obs, reward, done, info = session.step(request.action)

    return {
        "state": serialize_game_state(session),
        "reward": reward,
        "done": done,
        "info": info
    }

@app.get("/game/{session_id}/actions")
def get_valid_actions(session_id: str):
    session = session_manager.get_session(session_id)
    if not session:
        raise HTTPException(status_code=404, detail="Session not found")

    mask = session.get_action_mask()
    # Convert numpy boolean array to list of indices
    valid_indices = [int(i) for i, valid in enumerate(mask) if valid]

    return {"valid_actions": valid_indices}


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
