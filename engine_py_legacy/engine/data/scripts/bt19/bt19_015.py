from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_015(CardScript):
    """BT19-015 Gallantmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Delete 1 of your opponent's Digimon with 8000 DP or less. If this effect didn't delete, this Digimon gains <Piercing> and gets +3000 DP until the end of your opponent's turn.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT19-015 Delete 1 Digimon with 8000 DP or less, or get +3000 DP and <Piercing> until the end of your opponent's turn")
        effect0.set_effect_description("[When Digivolving] Delete 1 of your opponent's Digimon with 8000 DP or less. If this effect didn't delete, this Digimon gains <Piercing> and gets +3000 DP until the end of your opponent's turn.")
        effect0.is_when_digivolving = True
        effect0._is_piercing = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: DP +3000, Gain Keyword Piercing"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(3000)
            if not (player and game):
                return
            def target_filter(p):
                if p.dp is None or p.dp > 8000:
                    return False
                return p.is_digimon
            def on_grant(target_perm):
                target_perm.grant_keyword('_is_piercing')
            game.effect_select_opponent_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [Your Turn][Once Per Turn] When an opponent's Digimon is deleted, gain 2 memory.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("BT19-015 Gain 2 memory")
        effect1.set_effect_description("[Your Turn][Once Per Turn] When an opponent's Digimon is deleted, gain 2 memory.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("GainMemory_BT19_015")
        effect1.is_on_deletion = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 2 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(2)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
