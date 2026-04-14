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

        # Deletion observer: [All Turns] When one of your opponent's level 5
        # or lower Digimon is deleted, you may suspend this Tamer to <Draw 1>.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.NoTiming)
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
            # The deleted permanent is passed via _fire_deletion_observers
            deleted = context.get('deleted_permanent')
            if deleted is None:
                return False
            # Must be opponent's Digimon (check via card source owner)
            deleted_owner = deleted.owner
            if deleted_owner is None or deleted_owner is player:
                return False
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

        # Timing: EffectTiming.OnMove
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
            # Must be opponent's Digimon (moved from breeding = opponent's action)
            # The move is done by the turn player, who is the opponent
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
