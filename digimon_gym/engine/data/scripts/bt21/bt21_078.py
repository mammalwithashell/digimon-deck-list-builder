from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_078(CardScript):
    """BT21-078 WereGarurumon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT21-078 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.4 for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Delete 1 of your opponent's level 4 or lower Digimon. If your Tamers have 2 or more total colors, add 1 to this effect's level maximum.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT21-078 Delete a level 4 digimon, or 5 if tamers have 2+ colour")
        effect1.set_effect_description("[On Play] Delete 1 of your opponent's level 4 or lower Digimon. If your Tamers have 2 or more total colors, add 1 to this effect's level maximum.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
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

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Delete 1 of your opponent's level 4 or lower Digimon. If your Tamers have 2 or more total colors, add 1 to this effect's level maximum.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT21-078 Delete a level 4 digimon, or 5 if tamers have 2+ colour")
        effect2.set_effect_description("[When Digivolving] Delete 1 of your opponent's level 4 or lower Digimon. If your Tamers have 2 or more total colors, add 1 to this effect's level maximum.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
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

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn][Once Per Turn] When your other Digimon are played or digivolve, if any of them have the [ADVENTURE] trait, 1 of your Digimon gains <Alliance> for the turn. Then, 1 of your Digimon may attack.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT21-078 Gain alliance then attack")
        effect3.set_effect_description("[Your Turn][Once Per Turn] When your other Digimon are played or digivolve, if any of them have the [ADVENTURE] trait, 1 of your Digimon gains <Alliance> for the turn. Then, 1 of your Digimon may attack.")
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("alliance-attack-BT21_078")
        effect3.is_on_play = True
        effect3._is_alliance = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Gain Keyword Alliance, Force Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_alliance')
            # Force attack — target Digimon may attack (requires engine SelectAttack)
            pass  # descriptive-tagged: force_attack

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Factory effect: alliance
        # Alliance
        effect4 = ICardEffect()
        effect4.set_effect_name("BT21-078 Alliance")
        effect4.set_effect_description("Alliance")
        effect4.is_inherited_effect = True
        effect4.is_on_attack = True
        effect4._is_alliance = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
