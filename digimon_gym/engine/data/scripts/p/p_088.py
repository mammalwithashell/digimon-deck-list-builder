from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_088(CardScript):
    """P-088 Siriusmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By placing a card with [Gammamon] in its name under this Digimon as its bottom digivolution card, this Digimon gets +2000 DP, for the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("P-088 Place 1 card to digivolution cards to gain DP +2000")
        effect0.set_effect_description("[When Digivolving] By placing a card with [Gammamon] in its name under this Digimon as its bottom digivolution card, this Digimon gets +2000 DP, for the turn.")
        effect0.is_optional = True
        effect0.is_when_digivolving = True
        effect0.dp_modifier = 2000

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: DP +2000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(2000)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] Delete 1 of your opponent's Digimon with 6000 DP or less. If this Digimon has 12000 DP or more, Delete 2 of your opponent's Digimon with 6000 DP or less instead.
        effect1 = ICardEffect()
        effect1.set_effect_name("P-088 Delete Digimon with 6000 DP or less")
        effect1.set_effect_description("[When Attacking] Delete 1 of your opponent's Digimon with 6000 DP or less. If this Digimon has 12000 DP or more, Delete 2 of your opponent's Digimon with 6000 DP or less instead.")
        effect1.is_on_attack = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.dp is None or p.dp > 6000:
                    return False
                if p.dp is None or p.dp < 12000:
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
