from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_004(CardScript):
    """BT23-004 DemiMeramon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] 1 of your Digimon with the [Ghost] trait gains <Blocker> and <Retaliation> until your opponent's turn ends.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-004 1 [Ghost] trait digimon gains <Blocker> and <Retaliation>")
        effect0.set_effect_description("[On Deletion] 1 of your Digimon with the [Ghost] trait gains <Blocker> and <Retaliation> until your opponent's turn ends.")
        effect0.is_inherited_effect = True
        effect0.is_on_deletion = True
        effect0._is_blocker = True
        effect0._is_retaliation = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain Keyword Blocker, Gain Keyword Retaliation"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Keyword grant: blocker — flag set on effect object
            pass
            # Keyword grant: retaliation — flag set on effect object
            pass

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
