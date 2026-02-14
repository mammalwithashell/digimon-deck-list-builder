from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_056(CardScript):
    """BT23-056"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-056 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blocker
        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-056 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] If you have a Tamer with the [CS] trait, give 1 of your opponent's Digimon '[Start of Your Main Phase] This Digimon attacks.' until their turn ends.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-056 Give 1 digimon '[Start of your main phase] this digimon attacks' ")
        effect2.set_effect_description("[On Play] If you have a Tamer with the [CS] trait, give 1 of your opponent's Digimon '[Start of Your Main Phase] This Digimon attacks.' until their turn ends.")
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
        # [When Digivolving] If you have a Tamer with the [CS] trait, give 1 of your opponent's Digimon '[Start of Your Main Phase] This Digimon attacks.' until their turn ends.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-056 Give 1 digimon '[Start of your main phase] this digimon attacks' ")
        effect3.set_effect_description("[When Digivolving] If you have a Tamer with the [CS] trait, give 1 of your opponent's Digimon '[Start of Your Main Phase] This Digimon attacks.' until their turn ends.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnAttackTargetChanged
        # [All Turns] [Once Per Turn] When attack targets change, <De-Digivolve 1> 1 of your opponent's Digimon.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT23-056 <De-Digivolve 1>")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When attack targets change, <De-Digivolve 1> 1 of your opponent's Digimon.")
        effect4.is_inherited_effect = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("Bt23_056_AT")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
