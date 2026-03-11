"""Action description functions for human-readable action labels.

Free functions that take a Game instance as their first argument.
No module in this file imports from game.py — dependency flows one way.
"""
from __future__ import annotations
from typing import TYPE_CHECKING, Optional, Dict

from .constants import (
    FIELD_SLOTS, TARGETS_PER_ATTACKER, FIELDS_PER_HAND, EFFECTS_PER_PERM,
    SOURCES_PER_FIELD, BREEDING_SLOT, SECURITY_TARGET, ACTION_SPACE_SIZE,
)
from ..data.enums import GamePhase

if TYPE_CHECKING:
    from . import Game
    from ..core.player import Player


def describe_actions(game: "Game", player_id: int) -> Dict[int, str]:
    """Return human-readable descriptions for all currently valid actions."""
    mask = game.get_action_mask(player_id)
    me = game.player1 if player_id == 1 else game.player2
    opp = game.player2 if player_id == 1 else game.player1
    descriptions: Dict[int, str] = {}

    for action_id in range(len(mask)):
        if mask[action_id] != 1.0:
            continue
        desc = _describe_single_action(game, action_id, me, opp)
        if desc:
            descriptions[action_id] = desc

    return descriptions


def _describe_single_action(game: "Game", action_id: int, me: "Player", opp: "Player") -> Optional[str]:
    """Return a human-readable description for a single action ID."""
    if game.current_phase == GamePhase.Mulligan:
        if action_id == 0:
            return "Keep opening hand"
        if action_id == 1:
            return "Mulligan (redraw 5)"

    # Play card from hand (0-29)
    if 0 <= action_id <= 29:
        idx = action_id
        if idx < len(me.hand_cards):
            card = me.hand_cards[idx]
            name = card.card_names[0] if card.card_names else card.card_id
            return f"Play {name} from hand"
        return f"Play hand[{idx}]"

    # Trash from hand (30-59)
    if 30 <= action_id <= 59:
        idx = action_id - 30
        if idx < len(me.hand_cards):
            card = me.hand_cards[idx]
            name = card.card_names[0] if card.card_names else card.card_id
            return f"Trash {name} from hand"
        return f"Trash hand[{idx}]"

    # Hatch (60)
    if action_id == 60:
        return "Hatch from egg deck"

    # Move from breeding (61)
    if action_id == 61:
        if me.breeding_area and me.breeding_area.top_card:
            name = me.breeding_area.top_card.card_names[0] if me.breeding_area.top_card.card_names else "Digimon"
            return f"Move {name} from breeding area"
        return "Move from breeding area"

    # Pass / decline (62)
    if action_id == 62:
        phase = game.current_phase
        if phase == GamePhase.Main:
            return "Pass turn"
        elif phase == GamePhase.Breeding:
            return "Pass breeding"
        elif phase == GamePhase.BlockTiming:
            return "Decline to block"
        elif phase == GamePhase.AllianceTiming:
            return "Done selecting allies"
        elif phase == GamePhase.EndOfTurnAction:
            return "End turn"
        return "Decline / Pass"

    # DNA Digivolve (63-92)
    if 63 <= action_id <= 92:
        idx = action_id - 63
        if idx < len(me.hand_cards):
            card = me.hand_cards[idx]
            name = card.card_names[0] if card.card_names else card.card_id
            return f"DNA Digivolve with {name}"
        return f"DNA Digivolve hand[{idx}]"

    # Actions 100-113: phase-dependent (Block / Alliance / Attack)
    if 100 <= action_id <= 100 + FIELD_SLOTS - 1:
        slot = action_id - 100
        if game.current_phase == GamePhase.BlockTiming:
            if slot < len(me.battle_area) and me.battle_area[slot].top_card:
                name = me.battle_area[slot].top_card.card_names[0]
                return f"Block with {name} (slot {slot})"
            return f"Block with slot {slot}"
        elif game.current_phase == GamePhase.AllianceTiming:
            if slot < len(me.battle_area) and me.battle_area[slot].top_card:
                name = me.battle_area[slot].top_card.card_names[0]
                return f"Alliance with {name} (slot {slot})"
            return f"Alliance with slot {slot}"
        # Fall through to attack formula for Main/EndOfTurnAction

    # Attack (100-399)
    if 100 <= action_id <= 399:
        normalized = action_id - 100
        attacker_idx = normalized // TARGETS_PER_ATTACKER
        target_idx = normalized % TARGETS_PER_ATTACKER
        attacker_name = "?"
        if attacker_idx < len(me.battle_area):
            a = me.battle_area[attacker_idx]
            attacker_name = a.top_card.card_names[0] if a.top_card and a.top_card.card_names else "Digimon"
        if target_idx == SECURITY_TARGET:
            return f"Attack player with {attacker_name}"
        elif target_idx < len(opp.battle_area):
            t = opp.battle_area[target_idx]
            target_name = t.top_card.card_names[0] if t.top_card and t.top_card.card_names else "Digimon"
            return f"Attack {target_name} with {attacker_name}"
        return f"Attack target[{target_idx}] with {attacker_name}"

    # Digivolve (400-999)
    if 400 <= action_id <= 999:
        normalized = action_id - 400
        hand_idx = normalized // FIELDS_PER_HAND
        field_idx = normalized % FIELDS_PER_HAND
        hand_name = "?"
        field_name = "?"
        if hand_idx < len(me.hand_cards):
            c = me.hand_cards[hand_idx]
            hand_name = c.card_names[0] if c.card_names else c.card_id
        if field_idx < len(me.battle_area):
            p = me.battle_area[field_idx]
            field_name = p.top_card.card_names[0] if p.top_card and p.top_card.card_names else "Digimon"
        elif field_idx == BREEDING_SLOT and me.breeding_area is not None:
            p = me.breeding_area
            field_name = p.top_card.card_names[0] if p.top_card and p.top_card.card_names else "Breeding Digimon"
        return f"Digivolve {hand_name} onto {field_name}"

    # Effect activation (1000-1999): Training=0, Delay=1
    if 1000 <= action_id <= 1999:
        normalized = action_id - 1000
        perm_idx = normalized // EFFECTS_PER_PERM
        effect_idx = normalized % EFFECTS_PER_PERM
        perm_name = "?"
        if perm_idx < len(me.battle_area):
            p = me.battle_area[perm_idx]
            perm_name = p.top_card.card_names[0] if p.top_card and p.top_card.card_names else "Permanent"
        elif perm_idx == BREEDING_SLOT and me.breeding_area:
            p = me.breeding_area
            perm_name = p.top_card.card_names[0] if p.top_card and p.top_card.card_names else "Breeding"
        if effect_idx == 0:
            return f"Training: {perm_name}"
        elif effect_idx == 1:
            return f"Delay: {perm_name}"
        return f"Activate effect {effect_idx} on {perm_name}"

    # Source selection (2000+)
    if 2000 <= action_id < ACTION_SPACE_SIZE:
        normalized = action_id - 2000
        field_idx = normalized // SOURCES_PER_FIELD
        source_idx = normalized % SOURCES_PER_FIELD
        return f"Select source[{source_idx}] from slot[{field_idx}]"

    # Selection phases use shared action space
    phase = game.current_phase
    if phase in (GamePhase.SelectTarget, GamePhase.SelectMaterial,
                 GamePhase.SelectHand, GamePhase.SelectReveal,
                 GamePhase.SelectTrash, GamePhase.SelectSecurity):
        return _describe_selection_action(game, action_id, me, opp)

    return f"Action {action_id}"


def _describe_selection_action(game: "Game", action_id: int, me: "Player", opp: "Player") -> str:
    """Describe a selection-phase action."""
    # Hand card selection (0-29)
    if 0 <= action_id <= 29:
        if action_id < len(me.hand_cards):
            c = me.hand_cards[action_id]
            name = c.card_names[0] if c.card_names else c.card_id
            return f"Select {name} from hand"
        return f"Select hand[{action_id}]"

    # Revealed cards (30-39)
    if 30 <= action_id <= 39:
        idx = action_id - 30
        if idx < len(game.revealed_cards):
            c = game.revealed_cards[idx]
            name = c.card_names[0] if c.card_names else c.card_id
            return f"Select {name} from revealed"
        return f"Select revealed[{idx}]"

    # Own security (40-49)
    if 40 <= action_id <= 49:
        return f"Select security[{action_id - 40}]"

    # Opponent security (50-59)
    if 50 <= action_id <= 59:
        return f"Select opponent security[{action_id - 50}]"

    # Decline (62)
    if action_id == 62:
        return "Decline selection"

    # Breeding area (99)
    if action_id == 99:
        if me.breeding_area and me.breeding_area.top_card:
            name = me.breeding_area.top_card.card_names[0] if me.breeding_area.top_card.card_names else "Breeding"
            return f"Select {name} from breeding"
        return "Select breeding area"

    # Own battle area (100-111)
    if 100 <= action_id <= 111:
        idx = action_id - 100
        if idx < len(me.battle_area):
            p = me.battle_area[idx]
            name = p.top_card.card_names[0] if p.top_card and p.top_card.card_names else "Permanent"
            return f"Select own {name}"
        return f"Select own slot[{idx}]"

    # Opponent battle area (112-123)
    if 112 <= action_id <= 123:
        idx = action_id - 112
        if idx < len(opp.battle_area):
            p = opp.battle_area[idx]
            name = p.top_card.card_names[0] if p.top_card and p.top_card.card_names else "Permanent"
            return f"Select opponent {name}"
        return f"Select opponent slot[{idx}]"

    # Trash selection (130-179)
    if 130 <= action_id <= 179:
        idx = action_id - 130
        if idx < len(me.trash_cards):
            c = me.trash_cards[idx]
            name = c.card_names[0] if c.card_names else c.card_id
            return f"Select {name} from trash"
        return f"Select trash[{idx}]"

    # Effect branch choice (1000-1009)
    if 1000 <= action_id <= 1009:
        return f"Choose effect option {action_id - 1000 + 1}"

    return f"Select action {action_id}"
