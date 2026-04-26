from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_055(CardScript):
    """BT19-055 Monitamon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Reveal the top 3 cards of your deck. Among them, add 1 card with [Knightmon] in its text or the [Twilight] trait to the hand and place 1 such card under any of your Tamers. Return the rest to the bottom of the deck.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDestroyedAnyone)
        effect0.set_effect_name("BT19-055 Reveal the top 3 cards of your deck, add 1, place 1 under a tamer")
        effect0.set_effect_description("[On Deletion] Reveal the top 3 cards of your deck. Among them, add 1 card with [Knightmon] in its text or the [Twilight] trait to the hand and place 1 such card under any of your Tamers. Return the rest to the bottom of the deck.")
        effect0.is_on_deletion = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Knightmon' in text):
                    return False
            else:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Add To Hand, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            if not (player and game):
                return
            def reveal_filter(c):
                return True
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
            game.effect_reveal_and_select(
                player, 3, reveal_filter, on_revealed, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: reboot
        # Reboot
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-055 Reboot")
        effect1.set_effect_description("Reboot")
        effect1.is_inherited_effect = True
        effect1._is_reboot = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
