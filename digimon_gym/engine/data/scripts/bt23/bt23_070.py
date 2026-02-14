from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_070(CardScript):
    """BT23-070"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-070 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 2
        effect0._alt_digi_cost = 2

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Delete all of your opponent's Digimon with the highest level. Then, if a card with [Belphemon] in its name is in this Digimon's digivolution cards, this Digimon attacks without suspending.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-070 Delete all highest level digimon, then if with [Belphemon] in name is in sources, atttack without suspending")
        effect1.set_effect_description("[When Digivolving] Delete all of your opponent's Digimon with the highest level. Then, if a card with [Belphemon] in its name is in this Digimon's digivolution cards, this Digimon attacks without suspending.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEndAttack
        # [End of Attack] This Digimon may digivolve into [Belphemon: Sleep Mode] in the trash, ignoring digivolution requirements and without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-070 Digivolve into [Belphemon: Sleep Mode] in trash")
        effect2.set_effect_description("[End of Attack] This Digimon may digivolve into [Belphemon: Sleep Mode] in the trash, ignoring digivolution requirements and without paying the cost.")
        effect2.is_optional = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
