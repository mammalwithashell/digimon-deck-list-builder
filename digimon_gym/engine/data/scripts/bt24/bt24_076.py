from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_076(CardScript):
    """Auto-transpiled from DCGO BT24_076.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDeclaration
        # [Trash] [Main] If you have 4 or fewer cards in your hand, you may play this card from your trash with the play cost reduced by 2.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-076 Play this card from trash with reduced cost")
        effect0.set_effect_description("[Trash] [Main] If you have 4 or fewer cards in your hand, you may play this card from your trash with the play cost reduced by 2.")

        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Play a card (from hand/trash/reveal)
            pass  # TODO: target selection for play_card

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-076 Effect")
        effect1.set_effect_description("Effect")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-076 Effect")
        effect2.set_effect_description("Effect")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] You may play 1 level 4 or lower Digimon card with the [Dark Dragon] or [Evil Dragon] trait from your trash without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-076 Play 1 Digimon from trash")
        effect3.set_effect_description("[On Deletion] You may play 1 level 4 or lower Digimon card with the [Dark Dragon] or [Evil Dragon] trait from your trash without paying the cost.")
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.is_on_deletion = True

        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Play a card (from hand/trash/reveal)
            pass  # TODO: target selection for play_card

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
