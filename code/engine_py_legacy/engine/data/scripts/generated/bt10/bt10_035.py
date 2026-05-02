from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_035(CardScript):
    """BT10-035 Darcmon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking][Once Per Turn] 1 of your opponent's Digimon gains <Security Attack -1> until the end of your opponent's turn. (This Digimon checks 1 fewer security cards.)
        effect0 = ICardEffect()
        effect0.set_effect_name("BT10-035 Security Attack -1")
        effect0.set_effect_description("[When Attacking][Once Per Turn] 1 of your opponent's Digimon gains <Security Attack -1> until the end of your opponent's turn. (This Digimon checks 1 fewer security cards.)")
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("SecurityAttack-1_BT10_035")
        effect0.is_on_attack = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
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
