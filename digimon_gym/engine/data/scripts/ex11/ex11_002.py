from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_002(CardScript):
    """EX11-002 Hiyarimon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Attack Unsuspended
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-002 Attack Unsuspended")
        effect0.set_effect_description("Attack Unsuspended")
        effect0.is_inherited_effect = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and len(permanent.digivolution_cards) >= 1):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Attack Unsuspended"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Can attack unsuspended Digimon via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CAN_ATTACK_UNSUSPENDED, perm,
                    value_fn=lambda: True, expiry='persistent')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
