from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_030(CardScript):
    """EX10-030 Cometmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-030 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 4
        effect0._alt_digi_cost = 4

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: collision
        # Collision
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-030 Collision")
        effect1.set_effect_description("Collision")
        effect1._is_collision = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may link 1 level 4 or lower Digimon card from your hand or this Digimon's digivolution cards to this Digimon without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-030 You may Link 1 level 4 or lower digimon")
        effect2.set_effect_description("[On Play] You may link 1 level 4 or lower Digimon card from your hand or this Digimon's digivolution cards to this Digimon without paying the cost.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may link 1 level 4 or lower Digimon card from your hand or this Digimon's digivolution cards to this Digimon without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-030 You may Link 1 level 4 or lower digimon")
        effect3.set_effect_description("[When Digivolving] You may link 1 level 4 or lower Digimon card from your hand or this Digimon's digivolution cards to this Digimon without paying the cost.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnLinkCardDiscarded
        # [All Turns] [Once Per Turn] When effects trash any of this Digimon's link cards, 1 of your opponent's Digimon gets -8000 DP for the turn.
        effect4 = ICardEffect()
        effect4.set_effect_name("EX10-030 -8k opposing digimon")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When effects trash any of this Digimon's link cards, 1 of your opponent's Digimon gets -8000 DP for the turn.")
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("MinusEX10_030")
        effect4.dp_modifier = -8000

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: DP -8000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-8000)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] [Once Per Turn] When this Digimon would leave the battle area, by trashing 1 of this Digimon's link cards, it doesn't leave.
        effect5 = ICardEffect()
        effect5.set_effect_name("EX10-030 Trash a linked card to prevent this digimon from leaving")
        effect5.set_effect_description("[All Turns] [Once Per Turn] When this Digimon would leave the battle area, by trashing 1 of this Digimon's link cards, it doesn't leave.")
        effect5.is_inherited_effect = True
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("Protection_EX10-030")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            return True

        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
