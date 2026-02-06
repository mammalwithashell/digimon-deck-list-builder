from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_058(CardScript):
    """Auto-transpiled from DCGO BT14_058.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By placing 1 [Satsuki Tamahime] from your hand as this Digimon's bottom digivolution card, 1 of your Digimon gains <Rush> for the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-058 Place 1 card to digivolution cards and your 1 Digimon gains Rush")
        effect0.set_effect_description("[On Play] By placing 1 [Satsuki Tamahime] from your hand as this Digimon's bottom digivolution card, 1 of your Digimon gains <Rush> for the turn.")
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Trash from hand (cost/effect)
            if player and player.hand_cards:
                player.trash_from_hand([player.hand_cards[-1]])

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By placing 1 [Satsuki Tamahime] from your hand as this Digimon's bottom digivolution card, 1 of your Digimon gains <Rush> for the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-058 Place 1 card to digivolution cards and your 1 Digimon gains Rush")
        effect1.set_effect_description("[When Digivolving] By placing 1 [Satsuki Tamahime] from your hand as this Digimon's bottom digivolution card, 1 of your Digimon gains <Rush> for the turn.")
        effect1.is_optional = True
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Trash from hand (cost/effect)
            if player and player.hand_cards:
                player.trash_from_hand([player.hand_cards[-1]])

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: blocker
        # Blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-058 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True
        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
