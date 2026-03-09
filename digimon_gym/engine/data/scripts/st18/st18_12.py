from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST18_12(CardScript):
    """ST18-12 Zephagamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # <Vortex>
        effect0 = ICardEffect()
        effect0.set_effect_name("ST18-12 Vortex")
        effect0.set_effect_description("Vortex")
        effect0._is_vortex = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [When Digivolving] Suspend 1 Digimon. Then, unsuspend 1 Digimon.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("ST18-12 Suspend 1 Digimon, then unsuspend 1 Digimon")
        effect1.set_effect_description(
            "[When Digivolving] Suspend 1 Digimon. Then, unsuspend 1 Digimon."
        )
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            # Suspend 1 Digimon (any)
            def suspend_filter(p):
                return p.is_digimon and not p.is_suspended

            def on_suspend(target_perm):
                target_perm.suspend()

                # Then, unsuspend 1 Digimon (any)
                def unsuspend_filter(p):
                    return p.is_digimon and p.is_suspended

                def on_unsuspend(unsuspend_target):
                    unsuspend_target.unsuspend()

                game.effect_select_any_permanent(
                    player, on_unsuspend, filter_fn=unsuspend_filter,
                    is_optional=False)

            game.effect_select_any_permanent(
                player, on_suspend, filter_fn=suspend_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [All Turns] [Once Per Turn] When any Digimon unsuspend, for the turn,
        # this Digimon isn't affected by the effects of your opponent's Digimon
        # and gets +3000 DP.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUnTappedAnyone)
        effect2.set_effect_name("ST18-12 On unsuspend: immunity + +3000 DP")
        effect2.set_effect_description(
            "[All Turns] [Once Per Turn] When any Digimon unsuspend, for the "
            "turn, this Digimon isn't affected by the effects of your opponent's "
            "Digimon and gets +3000 DP."
        )
        effect2.is_optional = False
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("ST18_12_AT_Unsuspend")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            # Grant effect immunity for the turn
            from digimon_gym.engine.interfaces.modifiers import ModifierType
            game.register_modifier(
                ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                value_fn=lambda: True, expiry='end_of_turn')

            # Grant +3000 DP for the turn
            perm.change_dp(3000)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
