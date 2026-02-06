from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_089(CardScript):
    """Auto-transpiled from DCGO BT24_089.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] You may play 1 [Elizamon] or [Owen Dreadnought] from your hand or trash without paying the cost. Then, place this card in the battle area.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-089 Play 1 [Elizamon]/[Owen Dreadnought] from hand or trash, then place in battle area")
        effect0.set_effect_description("[Main] You may play 1 [Elizamon] or [Owen Dreadnought] from your hand or trash without paying the cost. Then, place this card in the battle area.")

        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
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

        # Timing: EffectTiming.OnTappedAnyone
        # [Your Turn] When any of your [Owen Dreadnought] suspend, <Delay> (By trashing this card after the placing turn, activate the effect below.)\r\n・1 of your [Dragonkin] or [Reptile] trait Digimon may digivolve into a [Dragonkin] or [Reptile] and [LIBERATOR] trait Digimon card in the hand with the digivolution cost reduced by 3.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-089 1 of your [Dragonkin] or [Reptile] trait Digimon may digivolve")
        effect1.set_effect_description("[Your Turn] When any of your [Owen Dreadnought] suspend, <Delay> (By trashing this card after the placing turn, activate the effect below.)\r\n・1 of your [Dragonkin] or [Reptile] trait Digimon may digivolve into a [Dragonkin] or [Reptile] and [LIBERATOR] trait Digimon card in the hand with the digivolution cost reduced by 3.")
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            pass  # TODO: digivolve effect needs card selection

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
