from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_011(CardScript):
    """BT22-011 BlueMeramon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-011 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.4 for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alliance
        # Alliance
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-011 Alliance")
        effect1.set_effect_description("Alliance")
        effect1._is_alliance = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDeclaration
        # [Main] [Once Per Turn] By paying 3 cost, you may play 1 play cost 5 or lower Digimon card with the [Flame] or [CS] trait from your trash without paying the cost. Then, this Digimon may attack.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT22-011 Pay 3, Play 1 level 5 or lower [Flame]/[CS] digimon from trash. then it may attack")
        effect2.set_effect_description("[Main] [Once Per Turn] By paying 3 cost, you may play 1 play cost 5 or lower Digimon card with the [Flame] or [CS] trait from your trash without paying the cost. Then, this Digimon may attack.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT22_011_Main")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card, Force Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not getattr(c, 'has_play_cost', False):
                    return False
                if getattr(c, 'get_cost_itself', 0) > 5:
                    return False
                if not (any('CS' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            # Force attack — target Digimon may attack (requires engine SelectAttack)
            pass  # descriptive-tagged: force_attack

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: alliance
        # Alliance
        effect3 = ICardEffect()
        effect3.set_effect_name("BT22-011 Alliance")
        effect3.set_effect_description("Alliance")
        effect3.is_inherited_effect = True
        effect3._is_alliance = True

        def condition3(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
