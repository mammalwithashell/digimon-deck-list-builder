from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_026(CardScript):
    """P-026 BlackWarGreymon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDeclaration
        # [Main] <Digi-Burst 2> (Trash 2 of this Digimon's digivolution cards to activate the effect below.) - Unsuspend this Digimon.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDeclaration)
        effect0._is_field_main = True
        effect0.set_effect_name("P-026 Unsuspend this Digimon")
        effect0.set_effect_description("[Main] <Digi-Burst 2> (Trash 2 of this Digimon's digivolution cards to activate the effect below.) - Unsuspend this Digimon.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
