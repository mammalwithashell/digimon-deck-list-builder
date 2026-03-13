from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_058(CardScript):
    """BT23-058 Craniamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-058 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.5 for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: reboot
        # Reboot
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-058 Reboot")
        effect1.set_effect_description("Reboot")
        effect1._is_reboot = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: blocker
        # Blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-058 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When any of your Digimon or Tamers would leave the battle area by your opponent's effects,
        # by suspending this Digimon, 1 of those Digimon or Tamers doesn't leave.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.WhenRemoveField)
        effect3.set_effect_name("BT23-058 Prevent Digimon from leaving play")
        effect3.set_effect_description("[All Turns] When any of your Digimon or Tamers would leave the battle area by your opponent's effects, by suspending this Digimon, 1 of those Digimon or Tamers doesn't leave.")
        effect3.is_optional = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Can't activate if already suspended (cost is "by suspending this Digimon")
            my_perm = card.permanent_of_this_card()
            if my_perm and my_perm.is_suspended:
                return False
            # The leaving permanent must belong to our player
            leaving_perm = context.get('permanent')
            player_ctx = context.get('player')
            owner = card.owner if card else None
            if not owner or not player_ctx or player_ctx is not owner:
                return False
            # Must be a Digimon or Tamer
            if not (getattr(leaving_perm, 'is_digimon', False) or
                    getattr(leaving_perm, 'is_tamer', False)):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Suspend THIS Craniamon to prevent a Digimon/Tamer from leaving"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not perm:
                return
            # Cost: suspend this Digimon
            my_perm = card.permanent_of_this_card() if card else None
            if my_perm:
                my_perm.suspend()
            # Engine handles removal prevention via WhenRemoveField callback interaction

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns] [Once Per Turn] When this Digimon suspends, delete all of your opponent's Digimon
        # with the lowest play cost.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnTappedAnyone)
        effect4.set_effect_name("BT23-058 Delete opponent's Digimon")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When this Digimon suspends, delete all of your opponent's Digimon with the lowest play cost.")
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("AT_BT23-058")

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only triggers when THIS Craniamon suspends
            suspended_perm = context.get('permanent')
            my_perm = card.permanent_of_this_card()
            if suspended_perm is not my_perm:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Delete ALL opponent's Digimon with the lowest play cost"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not player:
                return
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                digimon = [p for p in list(enemy.battle_area) if p.is_digimon and p.top_card]
                if digimon:
                    min_cost = min(p.top_card.get_cost_itself for p in digimon)
                    to_delete = [p for p in digimon if p.top_card.get_cost_itself == min_cost]
                    for target in to_delete:
                        enemy.delete_permanent(target, is_opponent_effect=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
