from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_032(CardScript):
    """P-032 Palmon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDigivolutionCardDiscarded
        # [Your Turn] When this card is trashed due to activating this Digimon's <Digi-Burst>, 1 of your Digimon gains  <Jamming> (This Digimon can't be deleted in battles against Security Digimon) for the turn.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDigivolutionCardDiscarded)
        effect0.set_effect_name("P-032 Your 1 Digimon gains Jamming")
        effect0.set_effect_description("[Your Turn] When this card is trashed due to activating this Digimon's <Digi-Burst>, 1 of your Digimon gains  <Jamming> (This Digimon can't be deleted in battles against Security Digimon) for the turn.")
        effect0.is_inherited_effect = True
        effect0._is_jamming = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain Keyword Jamming"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_grant(target_perm):
                target_perm.grant_keyword('_is_jamming')
            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
