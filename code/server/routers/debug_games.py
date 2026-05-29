"""Rust-backed debug game endpoints — scenario staging for the
browser-dev / Playwright e2e workflow (add-ui-scenario-test-substrate).

Replaces the legacy `engine_py_legacy`-based debug router (CLAUDE.md rule
#22: production code MUST NOT import the sunset Python engine). All staging
runs through `digimon_engine.RustDebugGame`, which wraps the real card pool
and presents the same play surface as `RustHeadlessGame` — so a game staged
here can be driven through the live `/games/{id}/...` routes.

Endpoints
---------
- POST   /debug/games                      create a staged game
- POST   /debug/games/{id}/set-memory      set the memory gauge
- POST   /debug/games/{id}/inject-card     add one card to a zone
- POST   /debug/games/{id}/place-on-field  place a digivolution stack
- POST   /debug/games/{id}/bulk-setup      replace multiple zones at once
- POST   /debug/games/{id}/step            step + accumulate events (debug-driven)
- GET    /debug/games/{id}/internal-state  full-information board read
- POST   /debug/games/{id}/evaluate        evaluate engine assertions

The assertion evaluator (groups 3 + 4 of the change) lives here as a thin
Python pass-through over the binding's introspection surface:
`internal_state()` (zones/stacks), `to_ui_json()` (effective DP),
`get_action_mask()` (action-legal), `get_pending_selection()` (selection
options), and a per-game accumulated event log (effect-triggered).
"""

from __future__ import annotations

from typing import Any
from uuid import uuid4

from fastapi import APIRouter, HTTPException

from digimon_engine import RustDebugGame

from server.routers.games import GameMeta, game_meta
from server.routers.schemas import (
    CreateDebugGameRequest,
    DebugBulkSetupRequest,
    DebugEvaluateRequest,
    DebugInjectCardRequest,
    DebugPlaceOnFieldRequest,
    DebugZoneSpec,
    SetMemoryRequest,
)
from server.routers.state import active_games

router = APIRouter(prefix="/debug", tags=["debug"])

# Per-game accumulated event log. `effect_triggered` assertions scan this.
# Populated whenever the debug router itself drains events (create-time
# action script + the /debug step endpoint). Games driven through /games
# surface events in those responses instead; the Playwright layer captures
# them there.
_debug_events: dict[str, list[dict[str, Any]]] = {}


def _require_debug_game(game_id: str) -> RustDebugGame:
    runner = active_games.get(game_id)
    if runner is None:
        raise HTTPException(status_code=404, detail="Game not found")
    if not isinstance(runner, RustDebugGame):
        raise HTTPException(
            status_code=400,
            detail="Game is not a debug-staged game",
        )
    return runner


def _apply_zone(game: RustDebugGame, pid: int, zone: DebugZoneSpec) -> None:
    """Apply one player's zone staging. Any present zone REPLACES the dealt
    contents (clear-then-populate)."""
    if zone.hand is not None:
        game.clear_zone(pid, "hand")
        for cid in zone.hand:
            game.inject_card(pid, cid, "hand")
    if zone.deck_top is not None:
        game.clear_zone(pid, "deck")
        # index 0 = next draw = deck top = end of the engine vec, so inject
        # in reverse: last list element goes in first (bottom).
        for cid in reversed(zone.deck_top):
            game.inject_card(pid, cid, "deck_top")
    if zone.security is not None:
        game.clear_zone(pid, "security")
        # security[0] = top = end of vec; inject bottom-first.
        for cid in reversed(zone.security):
            game.inject_card(pid, cid, "security_top")
    if zone.trash is not None:
        game.clear_zone(pid, "trash")
        for cid in zone.trash:
            game.inject_card(pid, cid, "trash")
    if zone.field is not None:
        game.clear_zone(pid, "battle_area")
        for fs in zone.field:
            game.place_on_field(pid, fs.stack, fs.is_suspended, fs.turn_played)
    if zone.breeding is not None:
        game.clear_zone(pid, "breeding")
        if zone.breeding:
            game.place_in_breeding(pid, zone.breeding)


def _apply_scalar_state(
    game: RustDebugGame,
    *,
    memory: int | None,
    phase: str | None,
    turn: int | None,
    first_player: int | None,
) -> None:
    if first_player is not None:
        game.set_first_player(first_player)
    if turn is not None:
        game.set_turn(turn)
    if phase is not None:
        game.set_phase(phase)
    if memory is not None:
        game.set_memory(memory)


def _state_payload(game_id: str, game: RustDebugGame) -> dict[str, Any]:
    return {
        "game_id": game_id,
        "state": game.to_ui_json(),
        "action_mask": game.get_action_mask().tolist(),
        "is_game_over": game.is_game_over,
        "current_player": game.current_player_id,
    }


@router.post("/games")
def create_debug_game(request: CreateDebugGameRequest):
    """Create a staged debug game. Supports both the legacy fixture fields
    (`starting_hand1/2`, `initial_memory`, `first_player`) and the richer
    `zones` / `phase` / `turn` scenario staging."""
    if not request.deck1 or not request.deck2:
        raise HTTPException(status_code=400, detail="Both decks must be provided")

    game_id = str(uuid4())
    try:
        game = RustDebugGame(request.deck1, request.deck2, request.seed)
    except Exception as exc:
        raise HTTPException(status_code=400, detail=f"Engine construction failed: {exc}")

    # Mulligan is auto-finalized at construction; make it explicit so the
    # board is staged in a post-mulligan turn phase.
    game.skip_mulligan()

    # Rich per-player zone staging (scenario fixtures).
    if request.zones:
        for pid_str, zone in request.zones.items():
            try:
                pid = int(pid_str)
            except ValueError:
                raise HTTPException(status_code=400, detail=f"Bad player key {pid_str!r}")
            _apply_zone(game, pid, zone)

    # Legacy starting-hand fields (existing e2e fixture).
    if request.starting_hand1:
        game.clear_zone(1, "hand")
        for cid in request.starting_hand1:
            game.inject_card(1, cid, "hand")
    if request.starting_hand2:
        game.clear_zone(2, "hand")
        for cid in request.starting_hand2:
            game.inject_card(2, cid, "hand")

    # Scalar state. Default phase to Main so a staged board is immediately
    # actionable; explicit `phase` overrides.
    _apply_scalar_state(
        game,
        memory=request.initial_memory,
        phase=request.phase or "Main",
        turn=request.turn,
        first_player=request.first_player,
    )

    try:
        game.validate()
    except Exception as exc:
        raise HTTPException(status_code=400, detail=f"Staged board invalid: {exc}")

    active_games[game_id] = game
    _debug_events[game_id] = []

    # Register a GameMeta so the staged game can be driven through the live
    # `/games/{id}/...` routes (which require meta for the human/agent turn
    # loop). Undo isn't supported for staged games — `human_action_history`
    # starts empty and `deck1/deck2/seed` are recorded only for shape.
    p1_human = request.player1_type.lower() == "human"
    p2_human = request.player2_type.lower() == "human"
    player_labels = {
        1: "You" if p1_human else "Agent",
        2: "You" if p2_human else "Agent",
    }
    game_meta[game_id] = GameMeta(
        p1_human=p1_human,
        p2_human=p2_human,
        last_log=[],
        last_events=[],
        player_labels=player_labels,
        deck1=list(request.deck1),
        deck2=list(request.deck2),
        seed=request.seed or 0,
    )

    payload = _state_payload(game_id, game)
    payload["player_labels"] = player_labels
    return payload


@router.post("/games/{game_id}/set-memory")
def debug_set_memory(game_id: str, request: SetMemoryRequest):
    game = _require_debug_game(game_id)
    game.set_memory(request.memory)
    return _state_payload(game_id, game)


@router.post("/games/{game_id}/inject-card")
def debug_inject_card(game_id: str, request: DebugInjectCardRequest):
    game = _require_debug_game(game_id)
    try:
        game.inject_card(request.player_id, request.card_id, request.zone)
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))
    return _state_payload(game_id, game)


@router.post("/games/{game_id}/place-on-field")
def debug_place_on_field(game_id: str, request: DebugPlaceOnFieldRequest):
    game = _require_debug_game(game_id)
    try:
        game.place_on_field(
            request.player_id, request.stack, request.is_suspended, request.turn_played
        )
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))
    return _state_payload(game_id, game)


@router.post("/games/{game_id}/bulk-setup")
def debug_bulk_setup(game_id: str, request: DebugBulkSetupRequest):
    game = _require_debug_game(game_id)
    for pid_str, zone in request.zones.items():
        try:
            pid = int(pid_str)
        except ValueError:
            raise HTTPException(status_code=400, detail=f"Bad player key {pid_str!r}")
        _apply_zone(game, pid, zone)
    _apply_scalar_state(
        game, memory=request.memory, phase=request.phase, turn=request.turn, first_player=None
    )
    try:
        game.validate()
    except Exception as exc:
        raise HTTPException(status_code=400, detail=f"Staged board invalid: {exc}")
    return _state_payload(game_id, game)


@router.post("/games/{game_id}/step")
def debug_step(game_id: str, action: int):
    """Step the staged game by one action and accumulate emitted events
    into the per-game log so `effect_triggered` assertions can scan them.
    For scenarios driven entirely through the debug router."""
    game = _require_debug_game(game_id)
    if game.is_game_over:
        raise HTTPException(status_code=400, detail="Game already over")
    mask = game.get_action_mask().tolist()
    if action < 0 or action >= len(mask) or mask[action] != 1:
        raise HTTPException(status_code=400, detail=f"action {action} is not legal")
    game.step(int(action))
    for ev in game.get_events_since_last_step():
        _debug_events.setdefault(game_id, []).append(ev)
    return _state_payload(game_id, game)


@router.get("/games/{game_id}/internal-state")
def debug_internal_state(game_id: str):
    game = _require_debug_game(game_id)
    return game.internal_state()


def _player(state: dict[str, Any], pid: int) -> dict[str, Any]:
    """internal_state() players are listed in order; player_id is 1-based."""
    for p in state["players"]:
        if p["player_id"] == pid:
            return p
    raise HTTPException(status_code=400, detail=f"no such player {pid}")


def _evaluate_one(
    game: RustDebugGame,
    state: dict[str, Any],
    ui: dict[str, Any],
    mask: list[int],
    events: list[dict[str, Any]],
    a: Any,
) -> dict[str, Any]:
    """Evaluate one engine assertion. Returns {kind, passed, message}."""
    kind = a.kind

    def ok(passed: bool, message: str = "") -> dict[str, Any]:
        return {"kind": kind, "passed": passed, "message": message}

    if kind == "memory_equals":
        actual = state["memory"]
        return ok(actual == a.value, f"expected memory {a.value}, got {actual}")

    if kind == "stack_top":
        p = _player(state, a.player)
        ba = p["battle_area"]
        if a.field_index >= len(ba):
            return ok(False, f"field_index {a.field_index} out of range ({len(ba)})")
        top = ba[a.field_index]["stack"][-1]
        return ok(top == a.card_id, f"expected top {a.card_id}, got {top}")

    if kind == "effective_dp":
        ba = ui[f"player{a.player}"]["battleArea"]
        if a.field_index >= len(ba):
            return ok(False, f"field_index {a.field_index} out of range ({len(ba)})")
        actual = ba[a.field_index]["dp"]
        return ok(actual == a.value, f"expected DP {a.value}, got {actual}")

    if kind == "zone_count":
        p = _player(state, a.player)
        counts = {
            "hand": len(p["hand"]),
            "deck": p["deck_count"],
            "security": p["security_count"],
            "trash": len(p["trash"]),
        }
        actual = counts.get(a.zone)
        if actual is None:
            return ok(False, f"unknown zone {a.zone}")
        return ok(actual == a.value, f"expected {a.zone} count {a.value}, got {actual}")

    if kind == "zone_contains":
        p = _player(state, a.player)
        if a.zone not in ("hand", "trash"):
            return ok(False, f"zone_contains supports hand|trash, got {a.zone}")
        present = a.card_id in p[a.zone]
        return ok(present, f"{a.card_id} {'in' if present else 'not in'} {a.zone}")

    if kind == "action_legal":
        legal = 0 <= a.action_id < len(mask) and mask[a.action_id] == 1
        want = a.expected if a.expected is not None else True
        return ok(legal == want, f"action {a.action_id} legal={legal}, expected {want}")

    if kind == "legal_selection_options":
        sel = game.get_pending_selection()
        n = len(sel["validIndices"]) if sel else 0
        return ok(n == a.count, f"expected {a.count} selection options, got {n}")

    if kind == "effect_triggered":
        fired = any(ev.get("type") == a.event_type for ev in events)
        want = a.expected if a.expected is not None else True
        return ok(
            fired == want,
            f"event {a.event_type} fired={fired}, expected {want}",
        )

    return ok(False, f"unknown assertion kind {kind!r}")


@router.post("/games/{game_id}/evaluate")
def debug_evaluate(game_id: str, request: DebugEvaluateRequest):
    """Evaluate a fixture's engine assertions against the current staged
    state. Single source of truth for engine-correctness verdicts — the
    Rust runner and the Playwright fixture both consume these results."""
    game = _require_debug_game(game_id)
    state = game.internal_state()
    ui = game.to_ui_json()
    mask = game.get_action_mask().tolist()
    events = _debug_events.get(game_id, [])
    results = [
        _evaluate_one(game, state, ui, mask, events, a) for a in request.assertions
    ]
    return {
        "all_passed": all(r["passed"] for r in results),
        "results": results,
    }


@router.delete("/games/{game_id}")
def debug_delete_game(game_id: str):
    active_games.pop(game_id, None)
    game_meta.pop(game_id, None)
    _debug_events.pop(game_id, None)
    return {"status": "deleted"}
