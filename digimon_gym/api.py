from contextlib import asynccontextmanager

from fastapi import Depends, FastAPI, HTTPException
from pydantic import BaseModel
from typing import List, Dict, Any, Optional, Union
from uuid import uuid4
import numpy as np
import json

from digimon_gym.digimon_gym import GameState, greedy_policy
from digimon_gym.engine.runners.headless_game import HeadlessGame
from digimon_gym.engine.runners.interactive_game import InteractiveGame
from digimon_gym.engine.runners.replay_runner import ReplayRunner
from digimon_gym.engine.data.enums import PlayerType
from digimon_gym.engine.data.deck_loader import (
    parse_deck, validate_deck, summarize_deck, DeckValidationResult,
)
from digimon_gym.db.database import get_db, init_db
from digimon_gym.db.models import GameRecording, GameRecordingReport
from digimon_gym.db.schemas import (
    BugReportRequest, BugReportResponse, RecordingResponse, RecordingSaveResponse,
    ReplayRequest, SeekRequest, ReplayCreateResponse, ReplayStepResponse,
)
from digimon_gym.db.routers import auth as auth_router
from digimon_gym.db.routers import users as users_router
from digimon_gym.db.routers import decks as decks_router
from digimon_gym.db.routers import friends as friends_router
from digimon_gym.db.routers import assets as assets_router
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy import select


@asynccontextmanager
async def lifespan(app: FastAPI):
    await init_db()
    yield


app = FastAPI(lifespan=lifespan)

# ─── Database-backed API routers ─────────────────────────────────────────
app.include_router(auth_router.router)
app.include_router(users_router.router)
app.include_router(decks_router.router)
app.include_router(friends_router.router)
app.include_router(assets_router.router)

# ─── Game Session Storage ─────────────────────────────────────────────
active_games: Dict[str, Union[HeadlessGame, InteractiveGame]] = {}
active_replays: Dict[str, ReplayRunner] = {}

# ─── Request/Response Models ──────────────────────────────────────────

class SimulationRequest(BaseModel):
    deck1: str
    deck2: str
    num_simulations: int = 100

class CreateGameRequest(BaseModel):
    deck1: List[str] = []
    deck2: List[str] = []
    deck1_raw: Optional[str] = None  # TTS or text format string
    deck2_raw: Optional[str] = None  # TTS or text format string
    player1_type: str = "agent"  # "agent" or "human"
    player2_type: str = "agent"
    # Recording options (headless only)
    record_actions: bool = False
    record_tensors: bool = False

class DeckRequest(BaseModel):
    deck: str  # Raw deck string (TTS or text format)

class GameActionRequest(BaseModel):
    action: int

# ─── Health Check ─────────────────────────────────────────────────────

@app.get("/")
def health_check():
    return {"status": "ok"}

# ─── Simulation Endpoint ─────────────────────────────────────────────

@app.post("/simulate")
def simulate_game(request: SimulationRequest):
    """Simulates games between two decks using greedy policy.

    Deck strings can be in any supported format:
      - Newline-separated card IDs (legacy)
      - TTS JSON array
      - digimoncard.io text format
    """
    try:
        deck1_list = parse_deck(request.deck1)
    except ValueError:
        # Fallback to legacy newline-separated format
        deck1_list = request.deck1.split('\n')
    try:
        deck2_list = parse_deck(request.deck2)
    except ValueError:
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
    """Create a new game session. Returns game_id and initial state.

    Decks can be provided as:
      - deck1/deck2: List[str] of card IDs (original format)
      - deck1_raw/deck2_raw: Raw string in TTS or text format (auto-detected)
    If both are provided, raw format takes precedence.

    For headless (agent-vs-agent) games, set record_actions=True to enable
    server-side action recording, and record_tensors=True for ML tensor capture.

    For interactive games, recording metadata (deck lists, zone orders, first player)
    is automatically included in the response for client-side recording.
    """
    # Resolve deck lists — raw format takes precedence
    try:
        deck1 = parse_deck(request.deck1_raw) if request.deck1_raw else request.deck1
        deck2 = parse_deck(request.deck2_raw) if request.deck2_raw else request.deck2
    except ValueError as e:
        raise HTTPException(status_code=400, detail=f"Deck parsing error: {e}")

    if not deck1 or not deck2:
        raise HTTPException(status_code=400, detail="Both decks must be provided")

    game_id = str(uuid4())
    p1_type = PlayerType.Human if request.player1_type.lower() == "human" else PlayerType.Agent
    p2_type = PlayerType.Human if request.player2_type.lower() == "human" else PlayerType.Agent

    if p1_type == PlayerType.Agent and p2_type == PlayerType.Agent:
        runner = HeadlessGame(
            deck1, deck2, verbose=True,
            record_actions=request.record_actions,
            record_tensors=request.record_tensors,
        )
    else:
        runner = InteractiveGame(deck1, deck2, p1_type, p2_type)

    active_games[game_id] = runner
    state = runner.game.to_json()
    mask = runner.get_action_mask().tolist()

    result = {
        "game_id": game_id,
        "state": state,
        "action_mask": mask,
    }

    # Include recording metadata for client-side recording (interactive games)
    if isinstance(runner, InteractiveGame):
        result["recording_metadata"] = runner.get_initial_state_dict()

    return result

@app.post("/game/{game_id}/action")
def game_action(game_id: str, request: GameActionRequest):
    """Execute an action in a game session.

    Response includes action_context for client-side recording.
    """
    runner = active_games.get(game_id)
    if not runner:
        raise HTTPException(status_code=404, detail="Game not found")

    # Capture pre-action context for client recording
    current_player_id = runner.game.current_player_id
    memory_before = runner.game.memory
    phase_before = runner.game.current_phase.name
    turn_before = runner.game.turn_count

    runner.step(request.action)
    state = runner.game.to_json()
    mask = runner.get_action_mask().tolist()

    result = {
        "state": state,
        "action_mask": mask,
        "is_game_over": runner.is_game_over,
        # Action context for client-side recording
        "action_context": {
            "player_id": current_player_id,
            "action_id": request.action,
            "phase": phase_before,
            "memory_before": memory_before,
            "memory_after": runner.game.memory,
            "turn": turn_before,
        },
    }

    # Include logs if interactive
    if isinstance(runner, InteractiveGame):
        result["logs"] = runner.get_last_log()
        runner.clear_log()

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

# ─── Game Recording Endpoints ────────────────────────────────────────

@app.get("/game/{game_id}/recording")
def get_game_recording(game_id: str):
    """Get the recording for an in-memory headless game.

    Returns the full action sequence (and optionally tensor snapshots)
    captured during the game. Only available for headless games created
    with record_actions=True.
    """
    runner = active_games.get(game_id)
    if not runner:
        raise HTTPException(status_code=404, detail="Game not found")
    if not isinstance(runner, HeadlessGame):
        raise HTTPException(
            status_code=400,
            detail="Recording endpoint is for headless games. Interactive games use client-side recording.",
        )
    recording = runner.get_recording()
    if not recording:
        raise HTTPException(
            status_code=404,
            detail="This game was not created with recording enabled (record_actions=True).",
        )
    return recording


@app.post("/game/{game_id}/recording/save")
async def save_game_recording(game_id: str, db: AsyncSession = Depends(get_db)):
    """Persist a headless game recording to the database.

    Call this after the game is over to save the recording for later analysis.
    """
    runner = active_games.get(game_id)
    if not runner:
        raise HTTPException(status_code=404, detail="Game not found")
    if not isinstance(runner, HeadlessGame):
        raise HTTPException(status_code=400, detail="Only headless game recordings can be saved server-side.")
    recording_data = runner.get_recording()
    if not recording_data:
        raise HTTPException(status_code=404, detail="No recording data available.")

    record = GameRecording(
        game_mode="headless",
        recording_json=json.dumps(recording_data),
        total_steps=recording_data.get("total_actions", 0),
        has_tensors=1 if recording_data.get("tensor_snapshots_count", 0) > 0 else 0,
    )
    db.add(record)
    await db.commit()
    await db.refresh(record)

    return RecordingSaveResponse(recording_id=record.id)


@app.get("/games/recordings")
async def list_recordings(db: AsyncSession = Depends(get_db)):
    """List saved game recordings from the database."""
    result = await db.execute(
        select(GameRecording).order_by(GameRecording.created_at.desc())
    )
    recordings = result.scalars().all()
    return [
        RecordingResponse(
            id=r.id,
            game_mode=r.game_mode,
            total_steps=r.total_steps,
            has_tensors=bool(r.has_tensors),
            created_at=r.created_at,
        )
        for r in recordings
    ]


@app.get("/games/recordings/{recording_id}")
async def get_saved_recording(recording_id: str, db: AsyncSession = Depends(get_db)):
    """Get a saved recording by ID from the database."""
    result = await db.execute(
        select(GameRecording).where(GameRecording.id == recording_id)
    )
    record = result.scalar_one_or_none()
    if not record:
        raise HTTPException(status_code=404, detail="Recording not found")
    return json.loads(record.recording_json)


@app.post("/games/report", response_model=BugReportResponse)
async def submit_bug_report(request: BugReportRequest, db: AsyncSession = Depends(get_db)):
    """Accept a client-side game recording for bug reporting.

    The client accumulates recording data (initial state + actions) during
    interactive play and submits the bundle here for developer review.
    """
    report = GameRecordingReport(
        description=request.description,
        recording_json=json.dumps(request.recording),
    )
    db.add(report)
    await db.commit()
    await db.refresh(report)

    return BugReportResponse(report_id=report.id)


# ─── Replay Endpoints ────────────────────────────────────────────────

@app.post("/games/replay", response_model=ReplayCreateResponse)
def create_replay(request: ReplayRequest):
    """Create a replay session from a recording dict.

    Returns replay_id, total_steps, and the initial game state.
    Use step/seek endpoints to navigate through the replay.
    """
    try:
        runner = ReplayRunner(request.recording, verify=request.verify)
    except (ValueError, KeyError) as e:
        raise HTTPException(status_code=400, detail=f"Invalid recording: {e}")

    replay_id = str(uuid4())
    active_replays[replay_id] = runner

    return ReplayCreateResponse(
        replay_id=replay_id,
        total_steps=runner.total_steps,
        initial_state=runner.get_state(),
    )


@app.post("/games/replay/{replay_id}/step", response_model=ReplayStepResponse)
def replay_step(replay_id: str):
    """Advance the replay by one recorded action.

    Returns the step result with game state snapshot.
    """
    runner = active_replays.get(replay_id)
    if not runner:
        raise HTTPException(status_code=404, detail="Replay session not found")

    if runner.is_complete:
        raise HTTPException(status_code=400, detail="Replay is complete — no more actions")

    result = runner.step()
    return ReplayStepResponse(
        step_number=result.step_number,
        action_id=result.action_id,
        player_id=result.player_id,
        phase_before=result.phase_before,
        phase_after=result.phase_after,
        memory_before=result.memory_before,
        memory_after=result.memory_after,
        turn_number=result.turn_number,
        is_game_over=result.is_game_over,
        winner_id=result.winner_id,
        state=result.state,
        verification_ok=result.verification_ok,
        verification_errors=result.verification_errors,
    )


@app.post("/games/replay/{replay_id}/seek", response_model=ReplayStepResponse)
def replay_seek(replay_id: str, request: SeekRequest):
    """Jump to a specific step in the replay.

    Forward seek replays remaining actions. Backward seek rebuilds from scratch.
    """
    runner = active_replays.get(replay_id)
    if not runner:
        raise HTTPException(status_code=404, detail="Replay session not found")

    try:
        result = runner.seek(request.step)
    except ValueError as e:
        raise HTTPException(status_code=400, detail=str(e))

    return ReplayStepResponse(
        step_number=result.step_number,
        action_id=result.action_id,
        player_id=result.player_id,
        phase_before=result.phase_before,
        phase_after=result.phase_after,
        memory_before=result.memory_before,
        memory_after=result.memory_after,
        turn_number=result.turn_number,
        is_game_over=result.is_game_over,
        winner_id=result.winner_id,
        state=result.state,
        verification_ok=result.verification_ok,
        verification_errors=result.verification_errors,
    )


@app.delete("/games/replay/{replay_id}")
def delete_replay(replay_id: str):
    """Destroy a replay session."""
    if replay_id in active_replays:
        del active_replays[replay_id]
    return {"status": "deleted"}


@app.get("/games/recordings/{recording_id}/state")
async def get_recording_state_at_step(
    recording_id: str,
    step: int = 0,
    db: AsyncSession = Depends(get_db),
):
    """Stateless replay: load recording from DB, replay to step N, return state.

    No session needed — replays the recording on-the-fly.
    If step=0, returns the initial state (no actions executed).
    """
    result = await db.execute(
        select(GameRecording).where(GameRecording.id == recording_id)
    )
    record = result.scalar_one_or_none()
    if not record:
        raise HTTPException(status_code=404, detail="Recording not found")

    recording_data = json.loads(record.recording_json)

    try:
        runner = ReplayRunner(recording_data)
    except (ValueError, KeyError) as e:
        raise HTTPException(status_code=500, detail=f"Failed to load recording: {e}")

    if step == 0:
        return runner.get_state()

    if step < 0 or step > runner.total_steps:
        raise HTTPException(
            status_code=400,
            detail=f"step must be 0..{runner.total_steps}, got {step}",
        )

    try:
        result = runner.seek(step)
    except (ValueError, IndexError) as e:
        raise HTTPException(status_code=400, detail=str(e))

    return result.state


# ─── Deck Utility Endpoints ──────────────────────────────────────────

@app.post("/deck/parse")
def deck_parse(request: DeckRequest):
    """Parse a deck string (TTS or text format) into a card ID list.

    Returns the parsed card list and a summary with counts.
    """
    try:
        card_ids = parse_deck(request.deck)
    except ValueError as e:
        raise HTTPException(status_code=400, detail=str(e))

    summary = summarize_deck(card_ids)
    return {
        "card_ids": card_ids,
        "summary": summary,
        "total_cards": len(card_ids),
    }


@app.post("/deck/validate")
def deck_validate(request: DeckRequest):
    """Parse and validate a deck string against game rules and restricted list.

    Returns validation result with errors and warnings.
    """
    try:
        card_ids = parse_deck(request.deck)
    except ValueError as e:
        raise HTTPException(status_code=400, detail=str(e))

    result = validate_deck(card_ids)
    summary = summarize_deck(card_ids)
    return {
        "is_valid": result.is_valid,
        "errors": result.errors,
        "warnings": result.warnings,
        "summary": summary,
        "total_cards": len(card_ids),
    }
