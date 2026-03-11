"""Tensor building functions for the RL observation space.

Free functions that take a Game instance as their first argument.
No module in this file imports from game.py — dependency flows one way.
"""
from __future__ import annotations
from typing import TYPE_CHECKING, List

import numpy as np

from .constants import (
    TENSOR_SIZE, FIELD_SLOTS, MAX_HAND, MAX_TRASH, MAX_SECURITY, MAX_SOURCES,
    MAX_REVEALED, SLOT_SIZE, SOURCE_ENTRY_SIZE, DP_NORM,
    _GLOBAL, _MY_BATTLE, _OPP_BATTLE, _MY_HAND, _OPP_HAND,
    _MY_TRASH, _OPP_TRASH, _MY_SECURITY, _OPP_SECURITY,
    _MY_BREEDING, _OPP_BREEDING, _REVEALED, _SELECTION,
)
from ..data.card_registry import CardRegistry
from ..data.enums import GamePhase

if TYPE_CHECKING:
    from . import Game
    from ..core.permanent import Permanent
    from ..core.player import Player


def build_board_state_tensor(game: "Game", player_id: int) -> np.ndarray:
    """Build a flat tensor representing the board from player's perspective.

    Returns a numpy array of shape (TENSOR_SIZE,) with dtype float32.
    Card identities are encoded as integer registry indices (float-cast).
    The nn.Embedding lookup happens inside the FeaturesExtractor on the GPU.

    Layout (TENSOR_SIZE=1375):
      [0-9]         Global data
      [10-569]      My battle area  (14 slots × 40)
      [570-1129]    Opp battle area (14 slots × 40)
      [1130-1149]   My hand  (20 card IDs)
      [1150-1169]   Opp hand (20 card IDs)
      [1170-1214]   My trash (45 card IDs)
      [1215-1259]   Opp trash (45 card IDs)
      [1260-1269]   My security (10 card IDs)
      [1270-1279]   Opp security (10 card IDs)
      [1280-1319]   My breeding (1 slot × 40)
      [1320-1359]   Opp breeding (1 slot × 40)
      [1360-1369]   Revealed cards (10 card IDs)
      [1370-1374]   Selection context (5)
    """
    me = game.player1 if player_id == 1 else game.player2
    opp = game.player2 if player_id == 1 else game.player1

    t = np.zeros(TENSOR_SIZE, dtype=np.float32)

    # --- Global [0-9] ---
    t[0] = min(float(game.turn_count) / 30.0, 1.0)
    t[1] = float(game.current_phase.value)
    t[2] = float(_get_memory_for(game, me)) / 10.0
    # [3-9] are reserved (already 0.0)

    # --- My field ---
    off = _GLOBAL
    _write_field(t, off, me.battle_area, FIELD_SLOTS)

    # --- Opp field ---
    off += _MY_BATTLE
    _write_field(t, off, opp.battle_area, FIELD_SLOTS)

    # --- My hand ---
    off += _OPP_BATTLE
    _write_card_ids(t, off, me.hand_cards, MAX_HAND)

    # --- Opp hand ---
    off += _MY_HAND
    _write_card_ids(t, off, opp.hand_cards, MAX_HAND)

    # --- My trash ---
    off += _OPP_HAND
    _write_card_ids(t, off, me.trash_cards, MAX_TRASH)

    # --- Opp trash ---
    off += _MY_TRASH
    _write_card_ids(t, off, opp.trash_cards, MAX_TRASH)

    # --- My security (only face-up cards visible) ---
    off += _OPP_TRASH
    _write_security_ids(t, off, me)

    # --- Opp security (only face-up cards visible) ---
    off += _MY_SECURITY
    _write_security_ids(t, off, opp)

    # --- My breeding ---
    off += _OPP_SECURITY
    breeding_list = [me.breeding_area] if me.breeding_area else []
    _write_field(t, off, breeding_list, 1)

    # --- Opp breeding ---
    off += _MY_BREEDING
    opp_breeding_list = [opp.breeding_area] if opp.breeding_area else []
    _write_field(t, off, opp_breeding_list, 1)

    # --- Revealed cards ---
    off += _OPP_BREEDING
    _write_card_ids(t, off, game.revealed_cards, MAX_REVEALED)

    # --- Selection context ---
    off += _REVEALED
    ps = game.pending_selection
    if game.current_phase in (
        GamePhase.SelectTarget, GamePhase.SelectMaterial,
        GamePhase.SelectTrash, GamePhase.SelectSource,
        GamePhase.SelectHand, GamePhase.SelectReveal,
        GamePhase.SelectEffectChoice, GamePhase.SelectSecurity,
    ):
        t[off] = float(game.current_phase.value)

    if ps:
        t[off + 1] = float(len(ps.valid_indices))
        t[off + 2] = float(ps.selecting_player.player_id)

    # [off+3, off+4] are reserved (already 0.0)

    return t


def _get_memory_for(game: "Game", player: "Player") -> int:
    """Memory relative to player (positive = their favour)."""
    if player is game.turn_player:
        return game.memory
    return -game.memory


def _write_field(tensor: np.ndarray, start_idx: int, permanents: List, slots: int):
    """Write field slot data into tensor starting at start_idx.

    Layout per slot (SLOT_SIZE=40 floats):
      +0:       top card ID (integer registry index)
      +1:       current DP
      +2:       suspended (0/1)
      +3:       OPT total
      +4:       OPT used
      +5:       linked card count
      +6:       source count
      +7..+39:  11 source entries × 3 each:
                [card_id, opt_state, dp_contribution]
    """
    for i, perm in enumerate(permanents[:slots]):
        base = start_idx + i * SLOT_SIZE
        top = perm.top_card

        # +0: top card ID
        if top:
            tensor[base] = float(CardRegistry.get_id(top.card_id))

        # Scalar fields
        tensor[base + 1] = float(perm.dp or 0) / DP_NORM
        tensor[base + 2] = 1.0 if perm.is_suspended else 0.0
        tensor[base + 3] = float(perm.opt_total)
        tensor[base + 4] = float(perm.opt_used)
        tensor[base + 5] = float(len(perm.linked_cards))
        tensor[base + 6] = float(len(perm.card_sources))

        # Source entries: [card_id, opt_state, dp_contribution] × MAX_SOURCES
        src_base = base + 7
        for j, src in enumerate(perm.card_sources[:MAX_SOURCES]):
            off = src_base + j * SOURCE_ENTRY_SIZE
            tensor[off] = float(CardRegistry.get_id(src.card_id))
            tensor[off + 1] = perm.source_opt_state(src)
            tensor[off + 2] = perm.source_dp_contribution(src) / DP_NORM


def _write_card_ids(tensor: np.ndarray, start_idx: int, cards: list, limit: int):
    """Write card integer IDs into tensor starting at start_idx (1 float per card)."""
    for i, card in enumerate(cards[:limit]):
        tensor[start_idx + i] = float(CardRegistry.get_id(card.card_id))


def _write_security_ids(tensor: np.ndarray, start_idx: int, player: "Player"):
    """Write security card IDs — only face-up cards are visible, face-down = 0.0."""
    for i, card in enumerate(player.security_cards[:MAX_SECURITY]):
        if card in player.face_up_security:
            tensor[start_idx + i] = float(CardRegistry.get_id(card.card_id))
        # else stays 0.0 (face-down)
