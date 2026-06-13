"""Pydantic schemas for gameplay-oriented API routers.

Engine-only schemas — no database or SQLAlchemy dependencies.
"""

from __future__ import annotations

from typing import Any, Dict, List, Optional

from pydantic import BaseModel, Field, field_validator

from server.routers.seed_utils import parse_optional_seed


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
    # Scenario-staging knobs for the browser-dev E2E workflow. The Rust
    # engine path uses them; the Tauri desktop path ignores them.
    #
    # `seed` — explicit RNG seed. Without one the server generates a
    # cryptographic random seed (still recorded in meta for /debug
    # export). Pin this to replay the exact same deck shuffle every time.
    #
    # `action_script` — list of HUMAN action IDs to apply after engine
    # init + initial agent autoplay. Lets you fast-forward into a
    # mid-game scenario (e.g. "the state where DNA digivolve is legal")
    # without playing through manually. Agent steps between scripted
    # human actions are auto-played via the same greedy loop the live
    # /games/<id>/actions endpoint uses, so the script only needs to
    # capture *your* decisions.
    seed: Optional[int | str] = None
    action_script: Optional[list[int]] = None
    # Paced agent stepping (add-bot-action-pacing): when true, the opening
    # agent prelude advances at most one action; the client then drives
    # subsequent beats via POST /games/{id}/agent-step while the response's
    # `agent_pending` is true, so each bot action renders as a perceivable
    # beat. Ignored when `action_script` is set (staging is not
    # presentation). Distinct from the vestigial `agent_action_delay_ms`
    # (legacy server-side sleep — unused by the Rust-backed router).
    paced: bool = False

    @field_validator("seed", mode="before")
    @classmethod
    def _validate_seed(cls, value):
        return parse_optional_seed(value)


class GameActionRequest(BaseModel):
    action: int
    # See CreateGameRequest.paced — caps the post-action agent autoplay at
    # one action so the client can pace the bot's response.
    paced: bool = False


class SurrenderRequest(BaseModel):
    player_id: int = Field(ge=1, le=2)


class DeckParseRequest(BaseModel):
    deck: str


class DeckValidateRequest(BaseModel):
    deck: Optional[str] = None
    main_deck: list[str] = Field(default_factory=list)
    egg_deck: list[str] = Field(default_factory=list)
    game_mode: str = Field("standard", pattern=r"^(standard|eden|edh_commander|titan|no_restriction)$")


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
    # Rust-backed scenario staging (add-ui-scenario-test-substrate).
    # `seed` pins the shuffle; `phase`/`turn` set scalar state; `zones`
    # carries the rich per-player board staging from a scenario fixture.
    # All optional — the legacy fields above still drive the existing
    # e2e fixture unchanged.
    seed: Optional[int] = Field(None, ge=0)
    phase: Optional[str] = None
    turn: Optional[int] = Field(None, ge=1)
    zones: Optional[dict[str, "DebugZoneSpec"]] = None


class DebugFieldStack(BaseModel):
    """One battle-area permanent: a bottom-to-top digivolution stack with
    suspend state and turn-played value."""

    stack: list[str]
    is_suspended: bool = False
    turn_played: int = 0


class DebugZoneSpec(BaseModel):
    """Per-player zone staging. Any field present REPLACES the dealt
    contents of that zone (the loader clears it first)."""

    hand: Optional[list[str]] = None
    deck_top: Optional[list[str]] = None  # index 0 = next draw
    security: Optional[list[str]] = None  # top -> bottom
    trash: Optional[list[str]] = None
    field: Optional[list[DebugFieldStack]] = None
    breeding: Optional[list[str]] = None  # bottom -> top


class DebugInjectCardRequest(BaseModel):
    player_id: int = Field(ge=1, le=2)
    card_id: str
    zone: str = "hand"  # hand | deck_top | security_top | trash


class DebugPlaceOnFieldRequest(BaseModel):
    player_id: int = Field(ge=1, le=2)
    stack: list[str]  # bottom-to-top
    is_suspended: bool = False
    turn_played: int = 0


class DebugBulkSetupRequest(BaseModel):
    """Replace multiple zones in one call. Mirrors the create-time `zones`
    staging for an already-active game."""

    zones: dict[str, "DebugZoneSpec"]
    memory: Optional[int] = None
    phase: Optional[str] = None
    turn: Optional[int] = Field(None, ge=1)


class DebugAssertion(BaseModel):
    """One engine assertion from a scenario fixture. `kind` selects the
    check; the remaining fields are kind-specific (see qa/scenarios/README)."""

    kind: str
    value: Optional[int] = None
    player: Optional[int] = None
    field_index: Optional[int] = None
    card_id: Optional[str] = None
    zone: Optional[str] = None
    action_id: Optional[int] = None
    event_type: Optional[str] = None
    expected: Optional[bool] = None
    count: Optional[int] = None


class DebugEvaluateRequest(BaseModel):
    assertions: list[DebugAssertion]


class SetMemoryRequest(BaseModel):
    memory: int = Field(ge=-10, le=10)


class InjectCardRequest(BaseModel):
    player_id: int = Field(ge=1, le=2)
    card_id: str
    zone: str = "hand"  # "hand", "library_top", "security_top"


# ── Enhanced debug endpoints ─────────────────────────────────────────────


class FieldEntrySchema(BaseModel):
    card_ids: list[str]
    is_suspended: bool = False
    turn_played: int = -1  # -1 = no summoning sickness


class PlaceOnFieldRequest(BaseModel):
    player_id: int = Field(ge=1, le=2)
    card_ids: list[str]  # bottom-to-top digivolution stack
    is_suspended: bool = False
    turn_played: int = -1


class PlaceInBreedingRequest(BaseModel):
    player_id: int = Field(ge=1, le=2)
    card_ids: list[str]  # bottom-to-top stack


class ZoneSetupSchema(BaseModel):
    hand: list[str] = Field(default_factory=list)
    library_top: list[str] = Field(default_factory=list)
    security: list[str] = Field(default_factory=list)
    trash: list[str] = Field(default_factory=list)
    field: list[FieldEntrySchema] = Field(default_factory=list)
    breeding: list[str] = Field(default_factory=list)


class BulkSetupRequest(BaseModel):
    player1: ZoneSetupSchema = Field(default_factory=ZoneSetupSchema)
    player2: ZoneSetupSchema = Field(default_factory=ZoneSetupSchema)
    memory: Optional[int] = None
    phase: Optional[str] = None


class ClearZoneRequest(BaseModel):
    player_id: int = Field(ge=1, le=2)
    zone: str  # "hand", "field", "security", "trash", "breeding", "library"


class SetPhaseRequest(BaseModel):
    phase: str  # GamePhase name e.g. "Main", "Breeding"


# Resolve forward references for the debug-staging models that reference
# `DebugZoneSpec` before its definition (module uses `from __future__ import
# annotations`, so all hints are strings until rebuilt).
CreateDebugGameRequest.model_rebuild()
DebugBulkSetupRequest.model_rebuild()


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
