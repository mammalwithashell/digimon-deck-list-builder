from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_094(CardScript):
    """BT8-094 Digimon Emperor"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Effect 0: Deletion observer (NoTiming + _is_deletion_observer)
        # [All Turns] When one of your opponent's level 5 or lower Digimon is deleted,
        # you may suspend this Tamer to <Draw 1>.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT8-094 Draw 1")
        effect0.set_effect_description(
            "[All Turns] When one of your opponent's level 5 or lower Digimon is deleted, "
            "you may suspend this Tamer to <Draw 1>."
        )
        effect0.is_optional = True
        effect0._is_deletion_observer = True

        def condition0(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            # "you may suspend this Tamer" — can't activate if already suspended
            if perm.is_suspended:
                return False
            player = card.owner if card else None
            if not player:
                return False
            # The deleted permanent comes via 'deleted_permanent' in observer context
            deleted = context.get('deleted_permanent')
            if deleted is None:
                return False
            # Must be opponent's Digimon (deleted permanent's owner != our player)
            # In observer pattern: context['player'] is the observer's owner
            # The deleted permanent belongs to the opponent if its owner is not us
            # Check by seeing if deleted is NOT in our battle area/trash
            # Actually: _fire_deletion_observers passes 'player' as the observer's owner.
            # The 'owner' param to _fire_deletion_observers is the deleted perm's owner.
            # For cross-side: deleted perm owner != observer owner (context['player'])
            # We need to check that the deleted permanent belongs to player's enemy.
            enemy = player.enemy if hasattr(player, 'enemy') else None
            if enemy is None:
                return False
            # The deleted perm should belong to our opponent
            # Since _fire_deletion_observers is called with owner=deleted perm's owner,
            # and context['player'] is the observer's owner, we check that the deleted
            # perm is NOT ours. We can check if it was in enemy's trash after deletion.
            # Simplest: check if deleted perm is not in player's battle_area
            # (it's already been removed, so check card sources in trash)
            # Actually the reliable way: check the top_card's owner
            if deleted.top_card and hasattr(deleted.top_card, 'owner') and deleted.top_card.owner is player:
                return False
            # Also guard: if the deleted perm was in our own battle_area, skip
            # For robustness, just verify it's not ours by checking card source owner
            for cs in deleted.card_sources:
                if hasattr(cs, 'owner') and cs.owner is player:
                    return False
                break  # Just check the first card source

            if not deleted.is_digimon:
                return False
            # Level 5 or lower
            deleted_level = deleted.level
            if deleted_level is None or deleted_level > 5:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Suspend this Tamer (cost), then Draw 1 (reward)."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            # Cost: suspend this Tamer
            perm.suspend()
            # Reward: draw 1
            player.draw_cards(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Effect 1: OnMove timing
        # [Opponent's Turn] When one of your opponent's level 3 Digimon is moved
        # from their breeding area to their battle area, gain 2 memory.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnMove)
        effect1.set_effect_name("BT8-094 Memory +2")
        effect1.set_effect_description(
            "[Opponent's Turn] When one of your opponent's level 3 Digimon is moved "
            "from their breeding area to their battle area, gain 2 memory."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            # [Opponent's Turn] — only fires on opponent's turn
            if player.is_my_turn:
                return False
            # The moved permanent is in context via extra_context['moved_permanent']
            moved = context.get('moved_permanent')
            if moved is None:
                return False
            # Must be a Digimon
            if not moved.is_digimon:
                return False
            # Level 3 check
            moved_level = moved.level
            if moved_level is None or moved_level != 3:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 2 memory"""
            player = ctx.get('player')
            if player:
                player.add_memory(2)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT8-094 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
