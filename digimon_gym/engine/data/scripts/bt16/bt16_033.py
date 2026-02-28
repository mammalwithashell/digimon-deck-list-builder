from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_033(CardScript):
    """BT16-033 Harpymon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: armor_purge
        # Armor Purge
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-033 Armor Purge")
        effect0.set_effect_description("Armor Purge")
        effect0._is_armor_purge = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect1 = ICardEffect()
        effect1.set_effect_name("BT16-033 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 2
        effect1._alt_digi_cost = 2

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnSecurityCheck
        # [Your Turn] When this Digimon checks an opponent's security, if you have 3 or more security cards, gain 1 memory. If you have 2 or fewer security cards, [Recovery +1].
        effect2 = ICardEffect()
        effect2.set_effect_name("BT16-033 Gain memory or recover 1.")
        effect2.set_effect_description("[Your Turn] When this Digimon checks an opponent's security, if you have 3 or more security cards, gain 1 memory. If you have 2 or fewer security cards, [Recovery +1].")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain 1 memory, Recovery +1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)
            if player:
                player.recovery(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
