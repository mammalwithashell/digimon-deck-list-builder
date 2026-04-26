"""Per-recipient state filtering for network play.

The game engine's ``to_ui_json()`` returns *everything* – both players'
hands, security stacks, etc.  For WebSocket PvP this leaks hidden
information.  This module provides filtering functions that produce
perspective-appropriate copies of the full state dict.

Public zones (visible to everyone):
    battle area, breeding area, trash, memory gauge, phase, turn count,
    revealed cards, pending selection/attack context.

Hidden zones:
    hand card IDs      – only visible to the hand's owner
    security card IDs  – never revealed (face-down stack)
"""

from __future__ import annotations

import copy
from typing import Any, Dict


def _redact_player(player_data: dict) -> dict:
    """Replace hand/security card IDs/metadata with empty lists, preserving counts."""
    out = dict(player_data)
    out["handIds"] = []
    out["handCards"] = []
    out["securityIds"] = []
    return out


def filter_state_for_player(full_state: Dict[str, Any], player_id: int) -> Dict[str, Any]:
    """Return a copy of the game state filtered for a specific player.

    The player sees:
      - Their own hand card IDs and hand count.
      - Their own security *count* but NOT card IDs (face-down).
      - Opponent's hand *count* but NOT card IDs.
      - Opponent's security *count* but NOT card IDs.
      - All public zones for both players (battle area, trash, breeding).
      - Revealed cards, pending selection/attack context.
    """
    state = copy.copy(full_state)

    my_key = "player1" if player_id == 1 else "player2"
    opp_key = "player2" if player_id == 1 else "player1"

    # My data: keep hand IDs, redact security IDs
    my_data = dict(full_state[my_key])
    my_data["securityIds"] = []
    state[my_key] = my_data

    # Opponent data: redact both hand and security IDs
    state[opp_key] = _redact_player(full_state[opp_key])

    return state


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
