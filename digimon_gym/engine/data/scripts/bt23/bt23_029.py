from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_029(CardScript):
    """BT23-029"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-029 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Turuiemon] for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_name = "Turuiemon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Turuiemon') or permanent.contains_card_name('Wendigomon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alliance
        # Alliance
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-029 Alliance")
        effect1.set_effect_description("Alliance")
        effect1._is_alliance = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [All Turns] [Once Per Turn] When any of your cards with the [Beast], [Beastkin] or [CS] trait are played, until your opponent's turn ends, 1 of their Digimon can't activate [When Digivolving] effects.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-029 1 digimon gains can't activate [When Digivolving] effects.")
        effect2.set_effect_description("[All Turns] [Once Per Turn] When any of your cards with the [Beast], [Beastkin] or [CS] trait are played, until your opponent's turn ends, 1 of their Digimon can't activate [When Digivolving] effects.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT23_029_AT")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns] [Once Per Turn] When any of your other Digimon suspend, 1 of your opponent's Digimon gets -4000 DP for the turn.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-029 -4k DP")
        effect3.set_effect_description("[All Turns] [Once Per Turn] When any of your other Digimon suspend, 1 of your opponent's Digimon gets -4000 DP for the turn.")
        effect3.is_inherited_effect = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("BT23_029_ESS")
        effect3.dp_modifier = -4000

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: DP -4000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-4000)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
