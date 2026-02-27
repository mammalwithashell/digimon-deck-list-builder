from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_095(CardScript):
    """BT16-095 Shine of Bee"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Suspend 2 of your opponent's Digimon. Then, return all of your opponent's suspended Digimon with the lowest DP to the bottom of the deck. All of your Digimon get +3000 DP until the end of your opponent's turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-095 Suspend Digimon or tamers and activate effects")
        effect0.set_effect_description("[Main] Suspend 2 of your opponent's Digimon. Then, return all of your opponent's suspended Digimon with the lowest DP to the bottom of the deck. All of your Digimon get +3000 DP until the end of your opponent's turn.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Suspend, Effect Immunity"""
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
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
