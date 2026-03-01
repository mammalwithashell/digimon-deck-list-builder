from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_043(CardScript):
    """BT11-043 KingSukamon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-043 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] If your opponent has 16 or more cards in their trash, or you have 3 or more cards with [Sukamon] in their names in your trash, change 1 of your opponent's Digimon into a white Digimon with 3000 DP and an original name of [Sukamon] until the end of your opponent's turn.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT11-043 Opponent's 1 Digimon becomes [Sukamon]")
        effect1.set_effect_description("[On Play] If your opponent has 16 or more cards in their trash, or you have 3 or more cards with [Sukamon] in their names in your trash, change 1 of your opponent's Digimon into a white Digimon with 3000 DP and an original name of [Sukamon] until the end of your opponent's turn.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If your opponent has 16 or more cards in their trash, or you have 3 or more cards with [Sukamon] in their names in your trash, change 1 of your opponent's Digimon into a white Digimon with 3000 DP and an original name of [Sukamon] until the end of your opponent's turn.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT11-043 Opponent's 1 Digimon becomes [Sukamon]")
        effect2.set_effect_description("[When Digivolving] If your opponent has 16 or more cards in their trash, or you have 3 or more cards with [Sukamon] in their names in your trash, change 1 of your opponent's Digimon into a white Digimon with 3000 DP and an original name of [Sukamon] until the end of your opponent's turn.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] For each other Digimon with [Sukamon] in its name in play, this Digimon gains <Security Attack +1> for the turn.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnAllyAttack)
        effect3.set_effect_name("BT11-043 This Digimon gains Security Attack+")
        effect3.set_effect_description("[When Attacking] For each other Digimon with [Sukamon] in its name in play, this Digimon gains <Security Attack +1> for the turn.")
        effect3.is_on_attack = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.WhenPermanentWouldBeDeleted
        # [All Turns] When this Digimon would be deleted, by deleting 1 other Digimon with [Sukamon] in its name, prevent that deletion.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.WhenPermanentWouldBeDeleted)
        effect4.set_effect_name("BT11-043 Prevent this Digimon from being deleted")
        effect4.set_effect_description("[All Turns] When this Digimon would be deleted, by deleting 1 other Digimon with [Sukamon] in its name, prevent that deletion.")
        effect4.is_inherited_effect = True
        effect4.is_optional = True
        effect4.set_hash_string("Substitute_BT11_043")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
