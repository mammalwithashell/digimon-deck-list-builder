from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_072(CardScript):
    """BT10-072 Soundbirdmon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] By placing 1 purple Digimon card from your hand under one of your Tamers, <Draw 1>. (Draw 1 card from your deck.)
        effect0 = ICardEffect()
        effect0.set_effect_name("BT10-072 Place 1 card from hand under 1 Tamer to draw 1")
        effect0.set_effect_description("[When Attacking] By placing 1 purple Digimon card from your hand under one of your Tamers, <Draw 1>. (Draw 1 card from your deck.)")
        effect0.is_optional = True
        effect0.is_on_attack = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: save
        # Save
        effect1 = ICardEffect()
        effect1.set_effect_name("BT10-072 Save")
        effect1.set_effect_description("Save")
        effect1._is_save = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDigivolutionCardDiscarded
        # [Opponent's Turn] When an effect trashes this digivolution card, gain 1 memory.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT10-072 Memory +1")
        effect2.set_effect_description("[Opponent's Turn] When an effect trashes this digivolution card, gain 1 memory.")
        effect2.is_inherited_effect = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
