from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST12_10(CardScript):
    """ST12-10 Jesmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blitz
        # Blitz
        effect0 = ICardEffect()
        effect0.set_effect_name("ST12-10 Blitz")
        effect0.set_effect_description("Blitz")
        effect0.is_on_play = True
        effect0._is_blitz = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] You may play 1 Digimon card with [Sistermon] in its name from your hand without paying its memory cost.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnAllyAttack)
        effect1.set_effect_name("ST12-10 Play 1 Digimon card with [Sistermon] in its name from hand")
        effect1.set_effect_description("[When Attacking] You may play 1 Digimon card with [Sistermon] in its name from your hand without paying its memory cost.")
        effect1.is_optional = True
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

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn][Once Per Turn] When you play another Digimon by an effect, this Digimon gets +3000 DP and gains <Security Attack +1> for the turn. (This Digimon checks 1 additional security card.)
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("ST12-10 This Digimon gains DP +3000 and Security Attack +1")
        effect2.set_effect_description("[Your Turn][Once Per Turn] When you play another Digimon by an effect, this Digimon gets +3000 DP and gains <Security Attack +1> for the turn. (This Digimon checks 1 additional security card.)")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("DP+3000AndSAttack+1_ST12_10")
        effect2.is_on_play = True
        effect2.dp_modifier = 3000

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: DP +3000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(3000)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
