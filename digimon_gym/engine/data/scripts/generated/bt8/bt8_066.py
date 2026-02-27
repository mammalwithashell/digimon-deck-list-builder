from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_066(CardScript):
    """BT8-066 Hisyaryumon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAddDigivolutionCards
        # [Your Turn] When one of your effects places a digivolution card under this Digimon, you may digivolve this Digimon into a Digimon card in your hand with [X-Antibody] in its traits by paying its digivolution cost. When this Digimon would digivolve with this effect, reduce the digivolution cost by 1.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT8-066 This Digimon digivolves")
        effect0.set_effect_description("[Your Turn] When one of your effects places a digivolution card under this Digimon, you may digivolve this Digimon into a Digimon card in your hand with [X-Antibody] in its traits by paying its digivolution cost. When this Digimon would digivolve with this effect, reduce the digivolution cost by 1.")
        effect0.is_optional = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: reboot
        # Reboot
        effect1 = ICardEffect()
        effect1.set_effect_name("BT8-066 Reboot")
        effect1.set_effect_description("Reboot")
        effect1.is_inherited_effect = True
        effect1._is_reboot = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
