"""Serialization functions for Game state (to_json, to_ui_json).

Free functions that take a Game instance as their first argument.
No module in this file imports from game.py — dependency flows one way.
"""
from __future__ import annotations
from typing import TYPE_CHECKING, Optional, List, Dict, Any

from .constants import (
    FIELD_SLOTS, EFFECTS_PER_PERM, SECURITY_TARGET,
)

if TYPE_CHECKING:
    from . import Game
    from ..core.permanent import Permanent


# ─── Keyword display mapping ────────────────────────────────────────
# Maps internal _is_{flag} attribute names to human-readable UI labels.
_KEYWORD_DISPLAY_MAP = [
    ("_is_rush", "Rush"),
    ("_is_blocker", "Blocker"),
    ("_is_piercing", "Piercing"),
    ("_is_jamming", "Jamming"),
    ("_is_retaliation", "Retaliation"),
    ("_is_collision", "Collision"),
    ("_is_blitz", "Blitz"),
    ("_is_raid", "Raid"),
    ("_is_reboot", "Reboot"),
    ("_is_blast_digivolve", "Blast Digivolve"),
    ("_is_alliance", "Alliance"),
    ("_is_training", "Training"),
    ("_is_progress", "Progress"),
    ("_is_fortitude", "Fortitude"),
    ("_is_save", "Save"),
    ("_is_decoy", "Decoy"),
    ("_is_material_save", "Material Save"),
    ("_is_vortex", "Vortex"),
    ("_is_overclock", "Overclock"),
    ("_is_armor_purge", "Armor Purge"),
    ("_is_evade", "Evade"),
    ("_is_barrier", "Barrier"),
    ("_is_delay", "Delay"),
    ("_is_blast_dna_digivolve", "Blast DNA Digivolve"),
    ("_is_scapegoat", "Scapegoat"),
    ("_is_fragment", "Fragment"),
    ("_is_decode", "Decode"),
    ("_is_execute", "Execute"),
    ("_is_digisorption", "Digisorption"),
    ("_is_iceclad", "Iceclad"),
    ("_is_cannot_attack", "Cannot Attack"),
    ("_is_cannot_attack_player", "Cannot Attack Player"),
    ("_is_cannot_block", "Cannot Block"),
    ("_is_cannot_be_blocked", "Cannot Be Blocked"),
    ("_is_cannot_unsuspend", "Cannot Unsuspend"),
]

# ─── Keywords scanned by to_ui_json ──────────────────────────────
_UI_KEYWORDS = [
    '_is_blocker', '_is_piercing', '_is_jamming', '_is_retaliation',
    '_is_rush', '_is_reboot', '_is_raid', '_is_blitz', '_is_alliance',
    '_is_collision', '_is_training', '_is_progress', '_is_fortitude',
    '_is_save', '_is_decoy', '_is_material_save', '_is_vortex',
    '_is_overclock', '_is_armor_purge', '_is_evade', '_is_barrier',
    '_is_blast_digivolve', '_is_blast_dna_digivolve',
    '_is_delay', '_is_digisorption',
    '_is_scapegoat', '_is_fragment', '_is_iceclad',
    '_is_decode', '_is_execute',
    '_is_cannot_attack', '_is_cannot_attack_player', '_is_cannot_block',
    '_is_cannot_be_blocked', '_is_cannot_unsuspend',
]


def _get_perm_keywords(game: "Game", perm: "Permanent") -> List[str]:
    """Return list of active keyword display names for a permanent."""
    keywords = []
    for attr, label in _KEYWORD_DISPLAY_MAP:
        if perm.has_keyword(attr):
            keywords.append(label)
    # Security Attack modifier (special: includes value)
    sa_mod = perm.security_attack_modifier()
    if sa_mod > 0:
        keywords.append(f"Security Attack +{sa_mod}")
    elif sa_mod < 0:
        keywords.append(f"Security Attack {sa_mod}")
    return keywords


def _get_activatable_effects(game: "Game", perm: "Permanent", slot_idx: int) -> List[Dict[str, Any]]:
    """Return list of activatable effect descriptions for a permanent in a given slot."""
    effects = []
    owner = game._find_owner(perm)
    if (perm.is_digimon and not perm.is_suspended
            and perm.has_keyword('_is_training') and owner.library_cards):
        effects.append({
            "effectIdx": 0,
            "actionId": 1000 + slot_idx * EFFECTS_PER_PERM,
            "name": "Training",
            "description": "Suspend to place top deck card at bottom of digi stack",
        })
    if (game._has_delay_effect(perm)
            and perm.turn_played < game.turn_count):
        perm_name = perm.top_card.card_names[0] if perm.top_card and perm.top_card.card_names else "this card"
        effects.append({
            "effectIdx": 1,
            "actionId": 1000 + slot_idx * EFFECTS_PER_PERM + 1,
            "name": "Delay",
            "description": f"Trash {perm_name} to activate delayed effect",
        })
    return effects


def _serialize_perm(game: "Game", perm: "Permanent", slot_idx: int) -> Dict[str, Any]:
    """Serialize a single permanent for to_json(), including keywords and effects."""
    return {
        "TopCardId": perm.top_card.card_id if perm.top_card else None,
        "TopCardName": perm.top_card.card_names[0] if perm.top_card and perm.top_card.card_names else None,
        "DP": perm.dp,
        "Level": perm.level,
        "IsSuspended": perm.is_suspended,
        "SourceCount": len(perm.card_sources),
        "CardKind": perm.top_card.c_entity_base.card_kind.name if perm.top_card and perm.top_card.c_entity_base and perm.top_card.c_entity_base.card_kind else None,
        "TurnPlayed": perm.turn_played,
        "Keywords": _get_perm_keywords(game, perm),
        "ActivatableEffects": _get_activatable_effects(game, perm, slot_idx),
    }


def to_json(game: "Game") -> Dict[str, Any]:
    """Serialize game state to a dictionary (mirrors C# Game.ToJson)."""
    from ..core.player import Player

    def player_data(p: Player) -> Dict[str, Any]:
        ba = p.breeding_area
        return {
            "Id": p.player_id,
            "Memory": game._get_memory_for(p),
            "HandCount": len(p.hand_cards),
            "HandIds": [c.card_id for c in p.hand_cards],
            "SecurityCount": len(p.security_cards),
            "DeckCount": len(p.library_cards),
            "BattleAreaCount": len(p.battle_area),
            "BattleArea": [
                _serialize_perm(game, perm, idx)
                for idx, perm in enumerate(p.battle_area)
            ],
            "BreedingArea": {
                "TopCardId": ba.top_card.card_id if ba and ba.top_card else None,
                "TopCardName": ba.top_card.card_names[0] if ba and ba.top_card and ba.top_card.card_names else None,
                "Level": ba.level if ba else None,
                "Keywords": _get_perm_keywords(game, ba) if ba else [],
                "TurnPlayed": ba.turn_played if ba else None,
            } if ba else None,
        }

    return {
        "TurnCount": game.turn_count,
        "CurrentPhase": game.current_phase.name,
        "CurrentPlayer": game.turn_player.player_id,
        "MemoryGauge": game.memory,
        "IsGameOver": game.game_over,
        "Winner": game.winner.player_id if game.winner else None,
        "Player1": player_data(game.player1),
        "Player2": player_data(game.player2),
    }


def to_ui_json(game: "Game") -> Dict[str, Any]:
    """Extended game state for the UI frontend."""
    from ..core.permanent import Permanent
    from ..core.player import Player

    def clean_text(text: Optional[str]) -> str:
        return (text or "").replace("\r\n", "\n").strip()

    def perm_data(perm: Permanent) -> Dict[str, Any]:
        keywords = [
            kw.replace('_is_', '')
            for kw in _UI_KEYWORDS
            if perm.has_keyword(kw)
        ]

        keyword_innate: List[str] = []
        keyword_gained: List[str] = []
        grants = getattr(perm, "_granted_keywords", {})
        for kw in _UI_KEYWORDS:
            if not perm.has_keyword(kw):
                continue
            key = kw.replace('_is_', '')
            expiry = grants.get(kw)
            is_gained = (
                expiry is not None
                and (expiry == -1 or game.turn_count <= expiry)
            )
            if is_gained:
                keyword_gained.append(key)
            else:
                keyword_innate.append(key)

        sources = []
        for src in perm.card_sources:
            entity = getattr(src, "c_entity_base", None)
            sources.append({
                "cardId": src.card_id,
                "cardName": src.card_names[0] if src.card_names else None,
                "isTop": src is perm.top_card,
                "optState": perm.source_opt_state(src),
                "dpContribution": perm.source_dp_contribution(src),
                "mainEffectText": clean_text(entity.effect_description_eng if entity else ""),
                "inheritedEffectText": clean_text(entity.inherited_effect_description_eng if entity else ""),
                "colors": [c.value for c in getattr(src, 'card_colors', [])],
            })

        inherited_effects = []
        for idx, src in enumerate(perm.card_sources[:-1]):
            entity = getattr(src, "c_entity_base", None)
            inherited_text = clean_text(entity.inherited_effect_description_eng if entity else "")
            if inherited_text:
                inherited_effects.append({
                    "sourceIndex": idx,
                    "cardId": src.card_id,
                    "cardName": src.card_names[0] if src.card_names else None,
                    "text": inherited_text,
                })

        top_entity = perm.top_card.c_entity_base if perm.top_card else None
        main_effect_text = clean_text(top_entity.effect_description_eng if top_entity else "")

        dp_base = perm.top_card.base_dp if perm.top_card and perm.top_card.base_dp is not None else None
        dp_sources = [
            {
                "cardId": src.card_id,
                "cardName": src.card_names[0] if src.card_names else None,
                "value": perm.source_dp_contribution(src),
            }
            for src in perm.card_sources
        ]
        temp_mods = getattr(perm, "_dp_modifiers", [])
        if perm.is_immune_to_opponent_effects:
            dp_temporary = float(sum(m for m in temp_mods if m >= 0))
        else:
            dp_temporary = float(sum(temp_mods))

        colors = []
        if perm.top_card:
            colors = [c.value for c in getattr(perm.top_card, 'card_colors', [])]
        return {
            "topCardId": perm.top_card.card_id if perm.top_card else None,
            "topCardName": perm.top_card.card_names[0] if perm.top_card and perm.top_card.card_names else None,
            "dp": perm.dp,
            "level": perm.level,
            "isSuspended": perm.is_suspended,
            "sourceCount": len(perm.card_sources),
            "keywords": keywords,
            "keywordBreakdown": {
                "innate": keyword_innate,
                "gained": keyword_gained,
            },
            "securityAttackModifier": perm.security_attack_modifier(),
            "linkedCardIds": [lc.card_id for lc in perm.linked_cards],
            "sources": sources,
            "mainEffectText": main_effect_text,
            "inheritedEffects": inherited_effects,
            "dpBreakdown": {
                "base": dp_base,
                "sources": dp_sources,
                "temporary": dp_temporary,
                "aura": perm._get_aura_dp_modifier() if perm.is_digimon else 0,
                "total": perm.dp,
            },
            "turnPlayed": perm.turn_played,
            "colors": colors,
        }

    def player_ui_data(p: Player) -> Dict[str, Any]:
        breeding = None
        if p.breeding_area:
            breeding = perm_data(p.breeding_area)
        return {
            "id": p.player_id,
            "memory": game._get_memory_for(p),
            "handCount": len(p.hand_cards),
            "handIds": [c.card_id for c in p.hand_cards],
            "handCards": [
                {
                    "cardId": c.card_id,
                    "cardName": c.card_names[0] if c.card_names else "",
                    "playCost": c.c_entity_base.play_cost if c.c_entity_base else 0,
                    "level": c.c_entity_base.level if c.c_entity_base else None,
                    "dp": c.c_entity_base.dp if c.c_entity_base else None,
                    "colors": [col.value for col in c.card_colors],
                    "cardKind": c.c_entity_base.card_kind.value if c.c_entity_base else 0,
                    "evoCosts": [
                        {"color": ec.card_color.value, "level": ec.level, "cost": ec.memory_cost}
                        for ec in (c.c_entity_base.evo_costs if c.c_entity_base else [])
                    ],
                }
                for c in p.hand_cards
            ],
            "securityCount": len(p.security_cards),
            "securityIds": [
                c.card_id if c in p.face_up_security else None
                for c in p.security_cards
            ],
            "securityFaceUp": [c in p.face_up_security for c in p.security_cards],
            "deckCount": len(p.library_cards),
            "eggDeckCount": len(p.digitama_library_cards),
            "battleAreaCount": len(p.battle_area),
            "battleArea": [perm_data(perm) for perm in p.battle_area],
            "breedingArea": breeding,
            "trashIds": [c.card_id for c in p.trash_cards],
        }

    revealed = [
        {"cardId": c.card_id, "owner": c.owner.player_id if c.owner else 0}
        for c in game.revealed_cards
    ]

    pending_sel = None
    if game.pending_selection:
        ps = game.pending_selection
        pending_sel = {
            "phase": game.current_phase.value,
            "validIndices": ps.valid_indices,
            "isOptional": ps.is_optional,
            "prompt": ps.prompt,
            "selectingPlayer": ps.selecting_player.player_id,
        }
        if ps.effect_choices:
            pending_sel["effectChoices"] = ps.effect_choices
        if ps.keyword_prompt:
            pending_sel["keywordPrompt"] = ps.keyword_prompt

    pending_atk = None
    if game.pending_attack:
        pa = game.pending_attack
        attacker_slot = -1
        for i, perm in enumerate(game.turn_player.battle_area):
            if perm is pa.attacker:
                attacker_slot = i
                break
        target_slot = -1
        if isinstance(pa.effective_target, Permanent):
            enemy = game.turn_player.enemy
            for i, perm in enumerate(enemy.battle_area):
                if perm is pa.effective_target:
                    target_slot = i
                    break
        else:
            target_slot = SECURITY_TARGET  # security attack
        pending_atk = {
            "attackerSlot": attacker_slot,
            "targetSlot": target_slot,
        }

    return {
        "turnCount": game.turn_count,
        "currentPhase": game.current_phase.value,
        "currentPlayer": game.current_player_id,
        "memoryGauge": game._get_memory_for(game.player1),
        "isGameOver": game.game_over,
        "winner": game.winner.player_id if game.winner else None,
        "player1": player_ui_data(game.player1),
        "player2": player_ui_data(game.player2),
        "revealedCards": revealed,
        "pendingSelection": pending_sel,
        "pendingAttack": pending_atk,
    }
