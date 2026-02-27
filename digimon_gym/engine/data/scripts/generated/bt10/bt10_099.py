from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_099(CardScript):
    """BT10-099 Healing Therapy"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Until the end of your opponent's turn, 1 of your opponent's Digimon gains <Security Attack -1>. (This Digimon checks 1 fewer security cards.) If you have a [Venusmon] in play, 3 of your opponent's Digimon gain it instead.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT10-099 Change Security Attack")
        effect0.set_effect_description("[Main] Until the end of your opponent's turn, 1 of your opponent's Digimon gains <Security Attack -1>. (This Digimon checks 1 fewer security cards.) If you have a [Venusmon] in play, 3 of your opponent's Digimon gain it instead.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Change Security Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
