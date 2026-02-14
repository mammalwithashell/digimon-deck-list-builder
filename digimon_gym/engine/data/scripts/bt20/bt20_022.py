from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_022(CardScript):
    """BT20-022 Crabmon (X Antibody) | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-022 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Crabmon] for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_name = "Crabmon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Crabmon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] 1 of your Digimon can't be deleted in battle until the end of your opponent's turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-022 Select 1 of your Digimon to gain battle protection")
        effect1.set_effect_description("[On Play] 1 of your Digimon can't be deleted in battle until the end of your opponent's turn.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] 1 of your Digimon can't be deleted in battle until the end of your opponent's turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-022 Select 1 of your Digimon to gain battle protection")
        effect2.set_effect_description("[When Digivolving] 1 of your Digimon can't be deleted in battle until the end of your opponent's turn.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking][Once Per Turn] If you have 7 or fewer cards in your hand, <Draw 1>.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-022 Draw 1 card")
        effect3.set_effect_description("[When Attacking][Once Per Turn] If you have 7 or fewer cards in your hand, <Draw 1>.")
        effect3.is_inherited_effect = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Draw_BT20_022")
        effect3.is_on_attack = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
