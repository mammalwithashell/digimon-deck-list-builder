from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT4_115(CardScript):
    """BT4-115 Lucemon | Lv.3

    When you would play this card from your hand, if you have 10 or more cards
        in your trash, reduce the play cost by 8.
    [On Play] Recovery +1 (Deck).
    [All Turns] This Digimon can only digivolve into Digimon cards with
        [Lucemon] in their name.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Cost reduction: if 10+ cards in trash, reduce play cost by 8
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("BT4-115 Cost reduction with 10+ trash")
        effect0.set_effect_description(
            "When you would play this card from your hand, if you have 10 or "
            "more cards in your trash, reduce the play cost by 8."
        )
        effect0.cost_reduction = 8

        def condition0(context: Dict[str, Any]) -> bool:
            player = card.owner if card else None
            if not player:
                return False
            card_source = context.get('card_source')
            if card_source is not card:
                return False
            return len(player.trash_cards) >= 10
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [On Play] Recovery +1
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT4-115 On Play: Recovery +1")
        effect1.set_effect_description("[On Play] Recovery +1 (Deck).")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.recovery(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [All Turns] Can only digivolve into Digimon with [Lucemon] in name
        # descriptive-tagged: engine lacks digivolution restriction mechanism
        effect2 = ICardEffect()
        effect2.set_effect_name("BT4-115 Can only digivolve into Lucemon")
        effect2.set_effect_description(
            "[All Turns] This Digimon can only digivolve into Digimon cards "
            "with [Lucemon] in their name."
        )

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
