"""Per-recipient state filtering for network play.

The game engine's ``to_ui_json()`` returns *everything* – both players'
hands, security stacks, etc.  For WebSocket PvP this leaks hidden
information.  This module provides filtering functions that produce
perspective-appropriate copies of the full state dict.

Ported verbatim from ``engine_py_legacy.engine.state_filter`` (which is
engine-independent dict surgery): the Rust engine's
``digimon_engine.RustHeadlessGame.to_ui_json()`` deliberately mirrors the
legacy JSON shape (``player1``/``player2`` with ``handIds``/``handCards``/
``securityIds``), so the contract — never leak opponent ``handIds`` or
``handCards`` (CLAUDE.md rule 14) — carries over unchanged.

Public zones (visible to everyone):
    battle area, breeding area, trash, memory gauge, phase, turn count,
    revealed cards, pending selection/attack context.

Hidden zones:
    hand card IDs      – only visible to the hand's owner
    security card IDs  – never revealed (face-down stack)
"""

from __future__ import annotations

import copy
from typing import Any, Dict, List


def _redact_player(player_data: dict) -> dict:
    """Replace hand/security card IDs/metadata with empty lists, preserving counts."""
    out = dict(player_data)
    out["handIds"] = []
    out["handCards"] = []
    out["securityIds"] = []
    return out


def _swap_pid(pid: Any) -> Any:
    """Map an absolute player id (1 ↔ 2) to the opposite seat; pass ``None`` through."""
    if pid is None:
        return None
    return 2 if pid == 1 else 1


def filter_state_for_player(full_state: Dict[str, Any], player_id: int) -> Dict[str, Any]:
    """Return a copy of the game state filtered + perspective-normalized for a player.

    The player sees:
      - Their own hand card IDs and hand count.
      - Their own security *count* but NOT card IDs (face-down).
      - Opponent's hand *count* but NOT card IDs.
      - Opponent's security *count* but NOT card IDs.
      - All public zones for both players (battle area, trash, breeding).
      - Revealed cards, pending selection/attack context.

    **Perspective normalization (add-room-match-pvp seat-2 fix):** the viewer is
    always returned in the ``player1`` slot, because the frontend renders
    ``player1`` at the bottom/self position and the action mask is
    perspective-relative (``OWN``/``ENEMY``). For seat 1 this is the identity. For
    seat 2 the players are swapped and every ABSOLUTE player reference is remapped
    (``currentPlayer`` / ``winner`` / ``revealedCards[].owner`` /
    ``pendingSelection.selectingPlayer`` / ``zoneOwner`` / each player's ``id``)
    and ``memoryGauge`` — which ``to_ui_json`` emits from player1's perspective —
    is negated. Relative encodings (mask ranges, ``pendingAttack`` slots,
    ``pendingSelection.kind``/``validIndices``) are intentionally left untouched.
    Without this, the joiner (seat 2) sees the host's board on the bottom and
    their own board flipped to the top.
    """
    state = copy.copy(full_state)

    my_key = "player1" if player_id == 1 else "player2"
    opp_key = "player2" if player_id == 1 else "player1"

    # My data: keep hand IDs, redact security IDs. Opponent: redact both.
    my_data = dict(full_state[my_key])
    my_data["securityIds"] = []
    opp_data = _redact_player(full_state[opp_key])

    if player_id == 1:
        # Identity perspective — the viewer is already player1.
        state["player1"] = my_data
        state["player2"] = opp_data
        return state

    # Seat 2: normalize so the viewer becomes player1 (self/bottom).
    my_data["id"] = 1
    opp_data["id"] = 2
    state["player1"] = my_data
    state["player2"] = opp_data

    if "currentPlayer" in full_state:
        state["currentPlayer"] = _swap_pid(full_state["currentPlayer"])
    if "winner" in full_state:
        state["winner"] = _swap_pid(full_state["winner"])
    if full_state.get("memoryGauge") is not None:
        state["memoryGauge"] = -full_state["memoryGauge"]
    if "revealedCards" in full_state:
        state["revealedCards"] = [
            {**rc, "owner": _swap_pid(rc.get("owner"))} for rc in full_state["revealedCards"]
        ]
    pending = full_state.get("pendingSelection")
    if pending:
        pending = dict(pending)
        if pending.get("selectingPlayer") is not None:
            pending["selectingPlayer"] = _swap_pid(pending["selectingPlayer"])
        if pending.get("zoneOwner") is not None:
            pending["zoneOwner"] = _swap_pid(pending["zoneOwner"])
        state["pendingSelection"] = pending

    return state


def _swap_event_pid(pid: Any) -> Any:
    """Swap an absolute seat id (1 ↔ 2); pass everything else through unchanged.

    Unlike :func:`_swap_pid`, non-seat values are left as-is: GameOver events
    carry the engine's ``player`` default of ``0`` (no acting player), and a
    null ``winner``/``target_player`` means draw / hit-security. Neither should
    be coerced into a seat.
    """
    if pid == 1:
        return 2
    if pid == 2:
        return 1
    return pid


# The only ``meta`` keys that hold an ABSOLUTE player id, per the authoritative
# event shape in ``code/digimon-engine-py/src/lib.rs::event_to_pydict``:
#   - ``target_player`` (Attack)   - ``winner`` (GameOver)
# Every other meta value (delta/total/turn_count/dp/cost/…) is numeric data that
# must NOT be swapped. If the engine adds a new player-id meta key, add it here.
_EVENT_META_PID_KEYS = ("target_player", "winner")


def normalize_events_for_player(
    events: List[Dict[str, Any]], player_id: int
) -> List[Dict[str, Any]]:
    """Perspective-normalize an event batch for one recipient (companion to
    :func:`filter_state_for_player`).

    ``GameEvent``s carry ABSOLUTE player ids — top-level ``player`` plus the
    ``meta`` keys in :data:`_EVENT_META_PID_KEYS`. The per-recipient STATE is
    normalized so the viewer is always ``player1``; the event stream must match
    or the joiner (seat 2) sees their own moves labelled "Opponent" in the game
    log and event-driven animations (DigivolveBanner / BattleEffect) render on
    the wrong side.

    Seat 1 is the identity (the input list is returned untouched). Seat 2 swaps
    every absolute player id (1 ↔ 2) into fresh dicts — the input events are not
    mutated, because the same batch is fanned out to the other seat and to
    spectators (who keep the absolute perspective). RELATIVE encodings —
    ``source_slot`` / ``target_slot`` (field indices, like ``pendingAttack``
    slots) and all non-player meta — are left untouched.
    """
    if player_id == 1:
        # Identity perspective — the viewer is already player1.
        return events

    normalized: List[Dict[str, Any]] = []
    for ev in events:
        out = dict(ev)
        out["player"] = _swap_event_pid(ev.get("player"))
        meta = ev.get("meta")
        if isinstance(meta, dict) and any(k in meta for k in _EVENT_META_PID_KEYS):
            new_meta = dict(meta)
            for key in _EVENT_META_PID_KEYS:
                if key in new_meta:
                    new_meta[key] = _swap_event_pid(new_meta[key])
            out["meta"] = new_meta
        normalized.append(out)
    return normalized


def filter_state_for_spectator(
    full_state: Dict[str, Any],
    spectator_mode: str = "hidden",
) -> Dict[str, Any]:
    """Return a copy of the game state redacted for spectators.

    ``spectator_mode`` controls visibility:
      - ``"hidden"`` (default): Both players' hands and security are
        redacted (card counts only).  Spectators see board, trash,
        memory, phase, and publicly revealed cards.
      - ``"open"``:  Full visibility (opt-in by the game host, e.g.
        for tournament streams with delay).
    """
    if spectator_mode == "open":
        return copy.copy(full_state)

    state = copy.copy(full_state)
    state["player1"] = _redact_player(full_state["player1"])
    state["player2"] = _redact_player(full_state["player2"])
    return state
