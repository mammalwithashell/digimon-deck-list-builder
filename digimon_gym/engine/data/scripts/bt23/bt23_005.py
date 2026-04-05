from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_005(CardScript):
    """BT23-005 Elizamon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Your Turn] When this Digimon would digivolve into a Digimon card
        # with the [Reptile] or [Dragonkin] trait, reduce the digivolution cost by 1.
        #
        # Uses WhenWouldDigivolve timing so the engine's digivolve() path picks
        # it up.  Context keys: player, permanent, card_source (incoming card).
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.WhenWouldDigivolve)
        effect0.set_effect_name("BT23-005 Reduce the digivolution cost by 1")
        effect0.set_effect_description(
            "[Your Turn] When this Digimon would digivolve into a Digimon card "
            "with the [Reptile] or [Dragonkin] trait, reduce the digivolution "
            "cost by 1."
        )
        effect0.cost_reduction = 1

        def condition0(context: Dict[str, Any]) -> bool:
            # Must be on the field
            if card and card.permanent_of_this_card() is None:
                return False
            # [Your Turn] restriction
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check that the incoming digivolution card has Reptile or Dragonkin
            incoming = context.get('card_source')
            if incoming:
                traits = getattr(incoming, 'card_traits', []) or []
                if not any('Reptile' in t or 'Dragonkin' in t for t in traits):
                    return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Inherited: [Your Turn] This Digimon gets +2000 DP.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-005 DP modifier")
        effect1.set_effect_description("[Your Turn] This Digimon gets +2000 DP.")
        effect1.is_inherited_effect = True
        effect1.dp_modifier = 2000

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
