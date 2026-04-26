from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_085(CardScript):
    """BT10-085 Sistermon Ciel | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] If it's your turn, 1 of your Digimon may digivolve into a Digimon card with [Royal Knight] in its traits in your hand for its digivolution cost.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT10-085 Your 1 Digimon can digivolve to 1 [Royal Knight] card from hand")
        effect0.set_effect_description("[On Play] If it's your turn, 1 of your Digimon may digivolve into a Digimon card with [Royal Knight] in its traits in your hand for its digivolution cost.")
        effect0.is_optional = True
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (any('Royal Knight' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn][Once Per Turn] When one of your Digimon digivolves into a Digimon with [Huckmon] in its name or [Royal Knight] in its traits, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT10-085 Memory +1")
        effect1.set_effect_description("[Your Turn][Once Per Turn] When one of your Digimon digivolves into a Digimon with [Huckmon] in its name or [Royal Knight] in its traits, gain 1 memory.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Memory+1_BT10_085")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
