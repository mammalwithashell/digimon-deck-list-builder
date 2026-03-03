from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_036(CardScript):
    """EX6-036 Keramon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX6-036 Reveal 3, add 1 tamer/option and 1 [Unidentified] trait")
        effect0.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Add 1 Tamer or Option card with [Diaboromon] in its text and 1 card with the [Unidentified] trait among them to the hand. Trash the rest."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def reveal_filter_0(c):
                if not (getattr(c, 'is_tamer', False) or getattr(c, 'is_option', False)):
                    return False
                return 'Diaboromon' in getattr(c, 'card_text', '')

            def reveal_filter_1(c):
                return 'Unidentified' in getattr(c, 'card_traits', [])

            game.effect_reveal_and_select_multi(
                player, 3, [(reveal_filter_0, 'hand'), (reveal_filter_1, 'hand')],
                remaining_placement='trash', is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("EX6-036 Play 1 [Diaboromon] Token")
        effect1.set_effect_description(
            "[On Deletion] If this card has the [Unidentified] trait, you may play 1 [Diaboromon] (Digimon | 14 Cost | Level 6 | White | Mega | Unknown | Unidentified | DP3000) Token without paying the cost."
        )
        effect1.is_inherited_effect = True
        effect1.is_optional = True
        effect1.is_on_deletion = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                game.effect_play_token(player, 'diaboromon')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
