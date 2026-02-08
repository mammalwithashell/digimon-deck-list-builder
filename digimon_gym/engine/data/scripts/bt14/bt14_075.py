from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_075(CardScript):
    """Auto-transpiled from DCGO BT14_075.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Trash the top 3 cards of your deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-075 Trash 3 cards from deck top")
        effect0.set_effect_description("[On Play] Trash the top 3 cards of your deck.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] Trash the top 3 cards of your deck.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-075 Trash 3 cards from deck top")
        effect1.set_effect_description("[When Attacking] Trash the top 3 cards of your deck.")
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # [Your Turn] This Digimon gets +1000 DP for every 3 cards in your trash.
        # Dynamic DP — computed from trash count, not a flat modifier.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-075 DP +1000 per 3 trash")
        effect2.set_effect_description("[Your Turn] This Digimon gets +1000 DP for every 3 cards in your trash.")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return card.owner is not None and card.owner.is_my_turn

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player and perm:
                bonus = (len(player.trash_cards) // 3) * 1000
                perm.change_dp(bonus)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Trash 1 card in your opponent's hand without looking.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT14-075 Trash 1 card from opponent's hand")
        effect3.set_effect_description("[On Deletion] Trash 1 card in your opponent's hand without looking.")
        effect3.is_on_deletion = True

        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Trash from hand (cost/effect)
            if player and player.hand_cards:
                player.trash_from_hand([player.hand_cards[-1]])

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
