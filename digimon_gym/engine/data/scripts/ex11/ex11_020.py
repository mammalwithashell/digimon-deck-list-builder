from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_020(CardScript):
    """EX11-020 Hanimon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-020 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Kyaromon] for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_name = "Kyaromon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Kyaromon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] If delete other than in battle, you may play 1 [Shoemon] from your hand without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX11-020 Play 1 [Shoemon] from hand for free.")
        effect1.set_effect_description("[On Deletion] If delete other than in battle, you may play 1 [Shoemon] from your hand without paying the cost.")
        effect1.is_on_deletion = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
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
                if not (any('Shoemon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [Opponent's Turn][Once Per Turn] When one of your opponent's Digimon attacks, by deleting 1 of your other Digimon, end that attack.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX11-020 End the attack by deleting 1 of your Digimon")
        effect2.set_effect_description("[Opponent's Turn][Once Per Turn] When one of your opponent's Digimon attacks, by deleting 1 of your other Digimon, end that attack.")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("StopAttack_EX11-020")
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
