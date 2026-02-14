from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_020(CardScript):
    """BT20-020 Imperialdramon: Fighter Mode | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-020 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Imperialdramon: Dragon Mode] for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_name = "Imperialdramon: Dragon Mode"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Imperialdramon: Dragon Mode'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: raid
        # Raid
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-020 Raid")
        effect1.set_effect_description("Raid")
        effect1._is_raid = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Your opponent can't play Digimon or Tamers by effects until the end of their turn. Then, if [Imperialdramon: Dragon Mode] is in this Digimon's digivolution cards, trash your opponent's top security card.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-020 Gains raid, piercing, trash security stack")
        effect2.set_effect_description("[When Digivolving] Your opponent can't play Digimon or Tamers by effects until the end of their turn. Then, if [Imperialdramon: Dragon Mode] is in this Digimon's digivolution cards, trash your opponent's top security card.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop()
                        enemy.trash_cards.append(trashed)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns] [Once Per Turn] When your opponent's security stack is removed from, delete 1 of their Digimon with as much or less DP as this Digimon.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-020 Delete 1 Digimon with as much or less DP as this Digimon")
        effect3.set_effect_description("[All Turns] [Once Per Turn] When your opponent's security stack is removed from, delete 1 of their Digimon with as much or less DP as this Digimon.")
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("ImperialDramon_BT20_020")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
