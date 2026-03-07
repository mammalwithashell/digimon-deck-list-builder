"""Pydantic schemas for gameplay-oriented API routers.

Engine-only schemas — no database or SQLAlchemy dependencies.
Safe to import from the desktop sidecar.
"""

from __future__ import annotations

from typing import Any, Dict, List, Optional

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
    player1_policy: str = "greedy"  # "greedy", "random", or "trained"
    player2_policy: str = "greedy"  # "greedy", "random", or "trained"
    player1_model: Optional[str] = None  # ONNX model filename (for "trained" policy)
    player2_model: Optional[str] = None  # ONNX model filename (for "trained" policy)
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


class CreateDebugGameRequest(BaseModel):
    deck1: list[str]
    deck2: list[str]
    player1_type: str = "human"
    player2_type: str = "agent"
    player1_policy: str = "greedy"
    player2_policy: str = "greedy"
    agent_action_delay_ms: int = Field(0, ge=0, le=3000)
    # Deterministic controls
    first_player: int = Field(1, ge=1, le=2)
    skip_shuffle: bool = True
    starting_hand1: list[str] = Field(default_factory=list)
    starting_hand2: list[str] = Field(default_factory=list)
    auto_mulligan: str = "keep"  # "keep" or "manual"
    initial_memory: int = Field(0, ge=-10, le=10)


class SetMemoryRequest(BaseModel):
    memory: int = Field(ge=-10, le=10)


class InjectCardRequest(BaseModel):
    player_id: int = Field(ge=1, le=2)
    card_id: str
    zone: str = "hand"  # "hand", "library_top", "security_top"


# ── Replay ───────────────────────────────────────────────────────────────

class ReplayRequest(BaseModel):
    """Create a replay session from a recording dict."""
    recording: Dict[str, Any]
    verify: bool = False


class SeekRequest(BaseModel):
    """Jump to a specific step in a replay session."""
    step: int


class ReplayCreateResponse(BaseModel):
    """Response from creating a replay session."""
    replay_id: str
    total_steps: int
    initial_state: Dict[str, Any]


class ReplayStepResponse(BaseModel):
    """Response from a replay step or seek."""
    step_number: int
    action_id: int
    player_id: int
    phase_before: str
    phase_after: str
    memory_before: int
    memory_after: int
    turn_number: int
    is_game_over: bool
    winner_id: Optional[int] = None
    state: Dict[str, Any]
    verification_ok: Optional[bool] = None
    verification_errors: List[str] = Field(default_factory=list)
