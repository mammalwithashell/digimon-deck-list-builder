from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_038(CardScript):
    """Auto-transpiled from DCGO BT14_038.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.SecuritySkill
        # [Security] If you have 3 or more cards with [Sukamon] in their names in your trash, you may play 1 level 6 Digimon card with [Etemon] in its name from your hand without paying the cost.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-038 Play 1 Digimon from hand")
        effect0.set_effect_description("[Security] If you have 3 or more cards with [Sukamon] in their names in your trash, you may play 1 level 6 Digimon card with [Etemon] in its name from your hand without paying the cost.")
        effect0.is_optional = True
        effect0.is_security_effect = True
        effect0.is_security_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Play a card (from hand/trash/reveal)
            pass  # TODO: target selection for play_card
            # Trash from hand (cost/effect)
            if player and player.hand_cards:
                player.trash_from_hand([player.hand_cards[-1]])

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Place this card at the bottom of your security stack.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-038 Place this card at the bottom of security")
        effect1.set_effect_description("[On Deletion] Place this card at the bottom of your security stack.")
        effect1.is_on_deletion = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Add To Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Add top card of deck to security
            if player:
                player.recovery(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Place 1 [Etemon] from your trash at the bottom of your security stack.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-038 Place 1 [Etemon] from trash at the bottom of security")
        effect2.set_effect_description("[On Deletion] Place 1 [Etemon] from your trash at the bottom of your security stack.")
        effect2.is_inherited_effect = True
        effect2.is_on_deletion = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Add To Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Add top card of deck to security
            if player:
                player.recovery(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
