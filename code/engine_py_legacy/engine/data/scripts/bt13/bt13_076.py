from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_076(CardScript):
    """BT13-076 KingEtemon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-076 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 4
        effect0._alt_digi_cost = 4
        effect0._alt_digi_level = 5
        effect0._alt_digi_name = "Etemon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [All Turns][Once Per Turn] When another Digimon with [Etemon] or [Sukamon] in its name is deleted, 1 of your opponent's Digimon gets -3000 DP and gains <Security Attack -1> until the end of your opponent's turn.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("BT13-076 DP -3000 and Security Attack -1")
        effect1.set_effect_description("[All Turns][Once Per Turn] When another Digimon with [Etemon] or [Sukamon] in its name is deleted, 1 of your opponent's Digimon gets -3000 DP and gains <Security Attack -1> until the end of your opponent's turn.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("DP-3000_BT13_076")
        effect1.is_on_deletion = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP -3000, Change Security Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-3000)
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: blocker
        # Blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-076 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: cannot_return_to_hand
        # Cannot Return to Hand
        effect3 = ICardEffect()
        effect3.set_effect_name("BT13-076 Cannot Return to Hand")
        effect3.set_effect_description("Cannot Return to Hand")
        effect3._grant_bounce_immunity = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Factory effect: cannot_return_to_deck
        # Cannot Return to Deck
        effect4 = ICardEffect()
        effect4.set_effect_name("BT13-076 Cannot Return to Deck")
        effect4.set_effect_description("Cannot Return to Deck")
        effect4._grant_return_to_deck_immunity = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
