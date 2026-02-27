from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_036(CardScript):
    """EX6-036 Keramon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Reveal the top 3 cards of your deck. Add 1 Tamer or Option card with [Diaboromon] in its text and 1 card with the [Unidentified] trait among them to the hand. Trash the rest.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-036 Reveal 3, add 1 tamer/option and 1 [Unidentified] trait")
        effect0.set_effect_description("[On Play] Reveal the top 3 cards of your deck. Add 1 Tamer or Option card with [Diaboromon] in its text and 1 card with the [Unidentified] trait among them to the hand. Trash the rest.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Diaboromon' in text):
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

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] If this card has the [Unidentified] trait, you may play 1 [Diaboromon] (Digimon | 14 Cost | Level 6 | White | Mega | Unknown | Unidentified | DP3000) Token without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX6-036 Play 1 [Diaboromon] Token")
        effect1.set_effect_description("[On Deletion] If this card has the [Unidentified] trait, you may play 1 [Diaboromon] (Digimon | 14 Cost | Level 6 | White | Mega | Unknown | Unidentified | DP3000) Token without paying the cost.")
        effect1.is_inherited_effect = True
        effect1.is_optional = True
        effect1.is_on_deletion = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Token"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Play Diaboromon Token — token play not yet supported in engine
            pass  # descriptive-tagged: play_token

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
