"""Game session endpoints — Rust engine backed.

Engine-only router (no DB, no auth). The desktop client dispatches the
same operations through Tauri `invoke()` into the Rust engine; this
router exposes the same Rust engine over HTTP for the browser-dev path.

Previously this router used `engine_py_legacy.engine.runners`. That
path was hollowed during the Rust migration (CLAUDE.md rule #22:
production code MUST NOT import from `engine_py_legacy.*`). The
legacy runners also depend on Python card scripts that bit-rotted
when the EffectTiming enum was refactored — `ST14-12` still
references the removed `DelaySkill` value, blocking game creation.

Replacing the legacy backend with `RustHeadlessGame` (via PyO3
bindings) fixes both: no more legacy import, no more script bit-rot.
The HTTP response shape is kept identical so the frontend's
`gameApi.ts` HTTP fallback drops in without DTO translation —
`RustHeadlessGame.to_ui_json()` already returns the camelCase shape
the React `GameState` type speaks.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any
from uuid import uuid4

from fastapi import APIRouter, HTTPException
from digimon_engine import RustHeadlessGame, parse_deck

from server.routers.schemas import (
    CreateGameRequest,
    GameActionRequest,
    SurrenderRequest,
)
from server.routers.state import active_games

router = APIRouter(tags=["games"])


@dataclass
class GameMeta:
    """Per-session non-engine state — drives the agent-vs-human autoplay
    loop and stores logs/events the engine emits during a step so the
    HTTP response can carry them back. Mirrors the role `InteractiveGame`
    used to play in the legacy stack."""

    p1_human: bool
    p2_human: bool
    last_log: list[str]
    last_events: list[dict[str, Any]]
    player_labels: dict[int, str]


game_meta: dict[str, GameMeta] = {}


def _require_game(game_id: str) -> RustHeadlessGame:
    runner = active_games.get(game_id)
    if not runner:
        raise HTTPException(status_code=404, detail="Game not found")
    if not isinstance(runner, RustHeadlessGame):
        raise HTTPException(
            status_code=500,
            detail="Game session is not a Rust runner — incompatible runner registered",
        )
    return runner


def _require_meta(game_id: str) -> GameMeta:
    meta = game_meta.get(game_id)
    if not meta:
        raise HTTPException(status_code=404, detail="Game metadata missing")
    return meta


def _is_human_turn(game: RustHeadlessGame, meta: GameMeta) -> bool:
    """Python player-id convention: 1 → meta.p1_human, 2 → meta.p2_human."""
    cur = game.current_player_id
    if cur == 1:
        return meta.p1_human
    if cur == 2:
        return meta.p2_human
    return False


def _autoplay_agent_turns(
    game: RustHeadlessGame,
    meta: GameMeta,
    *,
    log_buffer: list[str],
    event_buffer: list[dict[str, Any]],
    max_steps: int = 4096,
) -> None:
    """Step through agent decisions until the engine wants a human or the
    game ends. Mirrors the legacy `InteractiveGame.run_step` agent loop.

    Per-step logs/events drain into the caller-supplied buffers so the
    HTTP response carries the cumulative trace from this call (matching
    how `gameApi.ts` consumes `logs` and `events`)."""
    steps = 0
    while not game.is_game_over and steps < max_steps:
        if _is_human_turn(game, meta):
            return
        action = game.greedy_action()
        game.step(action)
        for log_line in game.get_last_log():
            log_buffer.append(log_line)
        for ev in game.get_events_since_last_step():
            event_buffer.append(ev)
        steps += 1


def _build_state_payload(
    game: RustHeadlessGame,
    meta: GameMeta,
    *,
    logs: list[str] | None = None,
    events: list[dict[str, Any]] | None = None,
) -> dict[str, Any]:
    """Common response builder used by /actions, /steps, /state, etc.
    Keys mirror the legacy router's contract so the frontend's
    `gameApi.ts` HTTP fallback can consume the same shape it consumes
    from Tauri responses."""
    return {
        "state": game.to_ui_json(),
        "action_mask": game.get_action_mask().tolist(),
        "is_game_over": game.is_game_over,
        "logs": logs or [],
        "events": events or [],
        "is_human_turn": _is_human_turn(game, meta),
        "player_labels": meta.player_labels,
    }


@router.post("/games")
@router.post("/game/create", include_in_schema=False)
def create_game(request: CreateGameRequest):
    """Create a new Rust-engine-backed session.

    Accepts the same `CreateGameRequest` schema the legacy router took.
    Trained-policy support is currently dropped — only `greedy` is
    available for agent players in the HTTP path; the desktop Tauri
    path still has trained-policy support via the ONNX inference
    state. Browser-dev is for testing, not for matchmaking against
    trained policies, so this trade-off is acceptable for now."""
    try:
        deck1 = parse_deck(request.deck1_raw) if request.deck1_raw else request.deck1
        deck2 = parse_deck(request.deck2_raw) if request.deck2_raw else request.deck2
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=f"Deck parsing error: {exc}")

    if not deck1 or not deck2:
        raise HTTPException(status_code=400, detail="Both decks must be provided")

    game_id = str(uuid4())
    try:
        game = RustHeadlessGame(deck1, deck2, False, False, False, None)
    except Exception as exc:  # PyValueError from the Rust binding
        raise HTTPException(status_code=400, detail=f"Engine construction failed: {exc}")

    p1_human = request.player1_type.lower() == "human"
    p2_human = request.player2_type.lower() == "human"
    meta = GameMeta(
        p1_human=p1_human,
        p2_human=p2_human,
        last_log=[],
        last_events=[],
        player_labels={
            1: "You" if p1_human else "Agent",
            2: "You" if p2_human else "Agent",
        },
    )

    active_games[game_id] = game
    game_meta[game_id] = meta

    # If the opening turn belongs to an agent, advance until a human
    # decision (or game end). All emitted logs/events flow into the
    # initial response so the client never misses the agent's prelude.
    log_buffer: list[str] = []
    event_buffer: list[dict[str, Any]] = []
    _autoplay_agent_turns(game, meta, log_buffer=log_buffer, event_buffer=event_buffer)

    return {
        "game_id": game_id,
        **_build_state_payload(game, meta, logs=log_buffer, events=event_buffer),
    }


@router.post("/games/{game_id}/actions")
@router.post("/game/{game_id}/action", include_in_schema=False)
def game_action(game_id: str, request: GameActionRequest):
    """Execute a single human action in an active session."""
    game = _require_game(game_id)
    meta = _require_meta(game_id)

    if game.is_game_over:
        raise HTTPException(status_code=400, detail="Game already over")

    current_player_id = game.current_player_id
    # `RustHeadlessGame` doesn't expose `memory` as a top-level field
    # before/after a step (only via `to_ui_json`). We capture the
    # before-state from the UI JSON for action_context parity with the
    # legacy router. This is cheap (engine has the field in hand).
    pre = game.to_ui_json()
    memory_before = pre.get("memoryGauge", 0)
    phase_before = pre.get("currentPhase", "")
    turn_before = pre.get("turnCount", 0)

    try:
        game.step(int(request.action))
    except Exception as exc:
        raise HTTPException(status_code=400, detail=f"Step failed: {exc}")

    log_buffer = list(game.get_last_log())
    event_buffer = list(game.get_events_since_last_step())

    # If after the human step the next decision is an agent's, advance
    # through agent turns automatically.
    _autoplay_agent_turns(game, meta, log_buffer=log_buffer, event_buffer=event_buffer)

    payload = _build_state_payload(game, meta, logs=log_buffer, events=event_buffer)
    payload["action_context"] = {
        "player_id": current_player_id,
        "action_id": request.action,
        "phase": phase_before,
        "memory_before": memory_before,
        "memory_after": game.to_ui_json().get("memoryGauge", 0),
        "turn": turn_before,
    }
    return payload


@router.post("/games/{game_id}/steps")
@router.post("/game/{game_id}/step", include_in_schema=False)
def game_step(game_id: str):
    """Advance the session until the next human decision or game end.
    No-op if the current decision is already a human's."""
    game = _require_game(game_id)
    meta = _require_meta(game_id)

    log_buffer: list[str] = []
    event_buffer: list[dict[str, Any]] = []
    _autoplay_agent_turns(game, meta, log_buffer=log_buffer, event_buffer=event_buffer)
    return _build_state_payload(game, meta, logs=log_buffer, events=event_buffer)


@router.get("/games/{game_id}/state")
@router.get("/game/{game_id}/state", include_in_schema=False)
def game_state(game_id: str):
    """Return the current `to_ui_json` snapshot."""
    game = _require_game(game_id)
    return game.to_ui_json()


@router.get("/games/{game_id}/action-mask")
@router.get("/game/{game_id}/mask", include_in_schema=False)
def game_mask(game_id: str):
    """Return the current action mask."""
    game = _require_game(game_id)
    return {"action_mask": game.get_action_mask().tolist()}


@router.post("/games/{game_id}/surrender")
def surrender_game(game_id: str, request: SurrenderRequest):
    """Concede on behalf of `player_id`. Player IDs are 1 or 2 (Python
    convention); the Rust binding translates to its 0/1 internal IDs."""
    game = _require_game(game_id)
    meta = _require_meta(game_id)
    if game.is_game_over:
        raise HTTPException(status_code=400, detail="Game is already over.")
    try:
        game.concede(int(request.player_id))
    except Exception as exc:
        raise HTTPException(status_code=400, detail=f"Concede failed: {exc}")
    logs = list(game.get_last_log())
    events = list(game.get_events_since_last_step())
    payload = _build_state_payload(game, meta, logs=logs, events=events)
    payload["surrendered_by"] = request.player_id
    return payload


@router.delete("/games/{game_id}")
@router.delete("/game/{game_id}", include_in_schema=False)
def delete_game(game_id: str):
    """Drop the active session and its metadata."""
    active_games.pop(game_id, None)
    game_meta.pop(game_id, None)
    return {"status": "deleted"}
