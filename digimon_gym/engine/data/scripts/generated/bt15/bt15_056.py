from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_056(CardScript):
    """BT15-056 Ryudamon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT15-056 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 0
        effect0._alt_digi_cost = 0

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] By placing 1 [Shuu Yulin] from your hand or trash as this Digimon's bottom digivolution card, this Digimon isn't affected by the effects of your opponent's Digimon until the end of your opponent's turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT15-056 Place cards under this Digimon's digivolution cards to make this Digimon unaffected by your opponent's effects until the end of your opponent's turn.")
        effect1.set_effect_description("[Start of Your Main Phase] By placing 1 [Shuu Yulin] from your hand or trash as this Digimon's bottom digivolution card, this Digimon isn't affected by the effects of your opponent's Digimon until the end of your opponent's turn.")
        effect1.is_optional = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
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

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns] [Once Per Turn] When this Digimon becomes suspended, suspend 1 of your opponent's Digimon or Tamers with a play cost less than or equal to this Digimon's play cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT15-056 Suspend opponent's Digimon or Tamers with play cost less than this Digimon.")
        effect2.set_effect_description("[All Turns] [Once Per Turn] When this Digimon becomes suspended, suspend 1 of your opponent's Digimon or Tamers with a play cost less than or equal to this Digimon's play cost.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("suspend_BT15_056_inherited")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
