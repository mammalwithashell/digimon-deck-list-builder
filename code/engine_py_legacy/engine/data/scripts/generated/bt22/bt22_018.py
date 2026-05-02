from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_018(CardScript):
    """BT22-018 Sangomon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Gain Keyword Blocker, Gain Keyword Cannot Be Deleted By Battle
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-018 Place under to give blocker and battle immunity")
        effect0.set_effect_description("Gain Keyword Blocker, Gain Keyword Cannot Be Deleted By Battle")
        effect0.is_optional = True
        effect0.is_on_play = True
        effect0._is_blocker = True
        effect0._is_cannot_be_deleted_by_battle = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain Keyword Blocker, Gain Keyword Cannot Be Deleted By Battle"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_grant(target_perm):
                target_perm.grant_keyword('_is_blocker')
                target_perm.grant_keyword('_is_cannot_be_deleted_by_battle')
            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: jamming
        # Jamming
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-018 Jamming")
        effect1.set_effect_description("Jamming")
        effect1.is_inherited_effect = True
        effect1._is_jamming = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
