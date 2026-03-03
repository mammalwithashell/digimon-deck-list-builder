from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_064(CardScript):
    """BT22-064 Diaboromon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-064 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.5 from [Infermon] for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        effect0._alt_digi_name = "Infermon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Infermon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alliance
        # Alliance
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-064 Alliance")
        effect1.set_effect_description("Alliance")
        effect1.is_on_attack = True
        effect1._is_alliance = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may play 1 [Diaboromon] Token without paying the cost. (Digimon / Cost 14 / Lv.6 / White / Mega / Unknown / Unidentified / 3000 DP)
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT22-064 Play 1 [Diaboromon] token")
        effect2.set_effect_description("[When Digivolving] You may play 1 [Diaboromon] Token without paying the cost. (Digimon / Cost 14 / Lv.6 / White / Mega / Unknown / Unidentified / 3000 DP)")
        effect2.is_optional = True
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Token"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Play Diaboromon Token — token play not yet supported in engine
            if player and game:
                game.effect_play_token(player, 'diaboromon')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] You may play 1 [Diaboromon] Token without paying the cost. (Digimon / Cost 14 / Lv.6 / White / Mega / Unknown / Unidentified / 3000 DP)
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnAllyAttack)
        effect3.set_effect_name("BT22-064 Play 1 [Diaboromon] token")
        effect3.set_effect_description("[When Attacking] You may play 1 [Diaboromon] Token without paying the cost. (Digimon / Cost 14 / Lv.6 / White / Mega / Unknown / Unidentified / 3000 DP)")
        effect3.is_optional = True
        effect3.is_on_attack = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Token"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Play Diaboromon Token — token play not yet supported in engine
            if player and game:
                game.effect_play_token(player, 'diaboromon')

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [All Turns] [Once Per Turn] When any of your other Digimon with the [Unidentified] trait are played, delete 1 of your opponent's Digimon with the lowest play cost.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT22-064 Delete lowest play cost digimon")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When any of your other Digimon with the [Unidentified] trait are played, delete 1 of your opponent's Digimon with the lowest play cost.")
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT22_064_Destroy")
        effect4.is_on_play = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            digimon = [p for p in enemy.battle_area if p.is_digimon]
            if not digimon:
                return
            min_cost = min(getattr(p.top_card, 'get_cost_itself', 0) for p in digimon)
            def target_filter(p):
                return p.is_digimon and getattr(p.top_card, 'get_cost_itself', 0) == min_cost
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
