from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from typing import List, Dict, Any
from python_impl.headless_runner import HeadlessRunner
from python_impl.digimon_gym import GameState, greedy_policy
import random
import numpy as np

app = FastAPI()

class SimulationRequest(BaseModel):
    deck1: str
    deck2: str
    num_simulations: int = 100

@app.post("/simulate")
def simulate_game(request: SimulationRequest):
    """
    Simulates games between two decks using HeadlessRunner.
    """
    runner = HeadlessRunner(request.deck1, request.deck2, request.num_simulations)
    results = runner.run()

    return results

@app.get("/")
def health_check():
    return {"status": "ok"}
