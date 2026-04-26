from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_042(CardScript):
    """BT10-042 Venusmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] All of your opponent's Digimon gain <Security Attack -1> until the end of your opponent's turn. (This Digimon checks 1 fewer security cards.)
        effect0 = ICardEffect()
        effect0.set_effect_name("BT10-042 Security Attack -1")
        effect0.set_effect_description("[When Digivolving] All of your opponent's Digimon gain <Security Attack -1> until the end of your opponent's turn. (This Digimon checks 1 fewer security cards.)")
        effect0.is_when_digivolving = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Disable Effect, Effect Immunity
        effect1 = ICardEffect()
        effect1.set_effect_name("BT10-042 Ignore [When Attacking] and [When Digivolving] Effect of opponent's Digimon that has <Security Attack>")
        effect1.set_effect_description("Disable Effect, Effect Immunity")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Disable Effect, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Disable/invalidate effects on target — not yet in engine
            pass  # descriptive-tagged: disable_effect
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
