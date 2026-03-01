from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_065(CardScript):
    """BT11-065 Snatchmon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may place up to 2 [Vemmon] from your trash under this Digimon as its bottom digivolution cards. Then, if there are 4 or more [Vemmon] in this Digimon's digivolution cards, return 1 [Fusionize] from your trash to your hand.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT11-065 Place [Vemmon] from trash to digivolution cards and return 1 [Fusionize] from trash to hand")
        effect0.set_effect_description("[When Digivolving] You may place up to 2 [Vemmon] from your trash under this Digimon as its bottom digivolution cards. Then, if there are 4 or more [Vemmon] in this Digimon's digivolution cards, return 1 [Fusionize] from your trash to your hand.")
        effect0.is_when_digivolving = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Add To Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDigivolutionCardReturnToDeckBottom
        # [All Turns][Once Per Turn] When [Vemmon] is placed from this Digimon's digivolution cards at the bottom of its owner's deck, unsuspend this Digimon, and it gains <Blocker> until the end of your opponent's turn.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDigivolutionCardReturnToDeckBottom)
        effect1.set_effect_name("BT11-065 Unsuspend this Digimon and it gain Blocker")
        effect1.set_effect_description("[All Turns][Once Per Turn] When [Vemmon] is placed from this Digimon's digivolution cards at the bottom of its owner's deck, unsuspend this Digimon, and it gains <Blocker> until the end of your opponent's turn.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Unsuspend_BT11_065")
        effect1._is_blocker = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Unsuspend, Gain Keyword Blocker"""
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
            if perm:
                perm.grant_keyword('_is_blocker')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
