from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_069(CardScript):
    """BT8-069 Ouryumon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may place 1 card with [X-Antibody] in its traits from your hand under this Digimon as its bottom digivolution card to delete 1 of your opponent's Digimon with a play cost of 7 or less.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT8-069 Place a Card to digivolution cards to delete 1 Digimon with 7 or less Cost")
        effect0.set_effect_description("[When Digivolving] You may place 1 card with [X-Antibody] in its traits from your hand under this Digimon as its bottom digivolution card to delete 1 of your opponent's Digimon with a play cost of 7 or less.")
        effect0.is_optional = True
        effect0.is_when_digivolving = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAddDigivolutionCards
        # [Your Turn][Once Per Turn] When one of your effects places a digivolution card under one of your Digimon, this Digimon gets +2000 DP and can't be deleted by your opponent's effects until the end of your opponent's next turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT8-069 This Digimon gets DP +2000 and can't be deleted by effects")
        effect1.set_effect_description("[Your Turn][Once Per Turn] When one of your effects places a digivolution card under one of your Digimon, this Digimon gets +2000 DP and can't be deleted by your opponent's effects until the end of your opponent's next turn.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("DP+2000_BT8_069")
        effect1.dp_modifier = 2000

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP +2000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(2000)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEndAttack
        # [End of Attack][Once Per Turn] If this Digimon has [Alphamon] in its name, unsuspend it.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT8-069 Unsuspend this Digimon")
        effect2.set_effect_description("[End of Attack][Once Per Turn] If this Digimon has [Alphamon] in its name, unsuspend it.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Unsuspend_BT8_069")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
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

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
