from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_005(CardScript):
    """BT20-005 Kapurimon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnSecurityCheck
        # [Your Turn] When this Digimon checks face-up security cards, it gains <Jamming> for the turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-005 Gain <Jamming> for the turn")
        effect0.set_effect_description("[Your Turn] When this Digimon checks face-up security cards, it gains <Jamming> for the turn.")
        effect0.is_inherited_effect = True
        effect0._is_jamming = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Triggered on attack — validated by engine timing
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
                grants = getattr(target_perm, '_keyword_grants', [])
                grants.append('_is_jamming')
                target_perm._keyword_grants = grants
            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
