from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT6_009(CardScript):
    """BT6-009 BaoHuckmon | Lv.4 Red Digimon

    [On Play] Reveal the top 5 cards of your deck. Add up to 2 Digimon
        cards with [Huckmon], [Jesmon], or [Sistermon] in their names
        among them to your hand. Place the remaining cards at the bottom
        of your deck in any order.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [On Play] Reveal top 5, add up to 2 Huckmon/Jesmon/Sistermon ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT6-009 Reveal top 5 for Huckmon/Jesmon/Sistermon")
        effect0.set_effect_description(
            "[On Play] Reveal the top 5 cards of your deck. Add up to 2 "
            "Digimon cards with [Huckmon], [Jesmon], or [Sistermon] in their "
            "names among them to your hand. Place the remaining cards at the "
            "bottom of your deck in any order."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            if len(player.library_cards) < 1:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def reveal_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if c.contains_card_name('Huckmon'):
                    return True
                if c.contains_card_name('Jesmon'):
                    return True
                if c.contains_card_name('Sistermon'):
                    return True
                return False

            # Two passes, each selecting 1 matching Digimon to hand (up to 2 total)
            game.effect_reveal_and_select_multi(
                player, 5,
                [(reveal_filter, 'hand'), (reveal_filter, 'hand')],
                remaining_placement='deck_bottom',
                is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
