from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_016(CardScript):
    """BT13-016 SaviorHuckmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] When you play a Digimon with [Sistermon] in its name, this Digimon may digivolve into a Digimon card with [Jesmon] in its name in the hand for the digivolution cost. When this Digimon would digivolve by this effect, reduce the digivolution cost by 2.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-016 This Digimon digivolves into 1 Digimon card with [Jesmon] in its name")
        effect0.set_effect_description("[Your Turn] When you play a Digimon with [Sistermon] in its name, this Digimon may digivolve into a Digimon card with [Jesmon] in its name in the hand for the digivolution cost. When this Digimon would digivolve by this effect, reduce the digivolution cost by 2.")
        effect0.is_optional = True
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
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
                if not (any('Jesmon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking][Once Per Turn] If this Digimon has the [Royal Knight] trait, you may play 1 Digimon card with [Sistermon] in its name from your hand or trash without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-016 Play 1 Digimon card with [Sistermon] in its name")
        effect1.set_effect_description("[When Attacking][Once Per Turn] If this Digimon has the [Royal Knight] trait, you may play 1 Digimon card with [Sistermon] in its name from your hand or trash without paying the cost.")
        effect1.is_inherited_effect = True
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Play_BT13_016")
        effect1.is_on_attack = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (any('Sistermon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
