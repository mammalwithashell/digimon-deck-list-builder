from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_028(CardScript):
    """BT11-028 MachGaogamon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Until the end of your opponent's turn, this Digimon gains <Blocker> (When an opponent's Digimon attacks, you may suspend this Digimon to force the opponent to attack it instead) and gets +2000 DP for every 4 cards in your opponent's hand.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-028 This Digimon gains Blocker and it gets DP+")
        effect0.set_effect_description("[When Digivolving] Until the end of your opponent's turn, this Digimon gains <Blocker> (When an opponent's Digimon attacks, you may suspend this Digimon to force the opponent to attack it instead) and gets +2000 DP for every 4 cards in your opponent's hand.")
        effect0.is_when_digivolving = True
        effect0._is_blocker = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain Keyword Blocker"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_blocker')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAddHand
        # [All Turns][Once Per Turn] When an effect adds cards to your opponent's hand, unsuspend this Digimon.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT11-028 Unsuspend this Digimon")
        effect1.set_effect_description("[All Turns][Once Per Turn] When an effect adds cards to your opponent's hand, unsuspend this Digimon.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Unsuspend_BT11_028")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
