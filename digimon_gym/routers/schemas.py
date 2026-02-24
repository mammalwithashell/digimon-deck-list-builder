"""Pydantic schemas for gameplay-oriented API routers."""

from __future__ import annotations

from typing import Optional

from pydantic import BaseModel, Field


class SimulationRequest(BaseModel):
    deck1: str
    deck2: str
    num_simulations: int = Field(100, ge=1, le=10000)


class CreateGameRequest(BaseModel):
    deck1: list[str] = Field(default_factory=list)
    deck2: list[str] = Field(default_factory=list)
    deck1_raw: Optional[str] = None
    deck2_raw: Optional[str] = None
    player1_type: str = "agent"  # "agent" or "human"
    player2_type: str = "agent"  # "agent" or "human"
    player1_policy: str = "greedy"  # "greedy" or "random"
    player2_policy: str = "greedy"  # "greedy" or "random"
    agent_action_delay_ms: int = Field(350, ge=0, le=3000)
    record_actions: bool = False
    record_tensors: bool = False


class GameActionRequest(BaseModel):
    action: int


class DeckParseRequest(BaseModel):
    deck: str


class DeckValidateRequest(BaseModel):
    deck: Optional[str] = None
    main_deck: list[str] = Field(default_factory=list)
    egg_deck: list[str] = Field(default_factory=list)
