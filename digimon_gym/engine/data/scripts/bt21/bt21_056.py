from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_056(CardScript):
    """BT21-056 Vemmon | Lv.3

    [On Play] By trashing 1 card with [Vemmon] in its text from your hand,
    you may return 1 non-Digi-Egg card with [Vemmon] in its text from your
    trash to the hand.

    --- Inherited ---
    [Your Turn] When this Digimon with [Vemmon] in its text would digivolve,
    reduce the digivolution cost by 1.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _has_vemmon_text(c) -> bool:
            return 'Vemmon' in getattr(c, 'card_text', '')

        # ─── Effect 0: [On Play] Trash 1 Vemmon-text from hand to return 1 non-egg Vemmon-text from trash
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT21-056 Trash Vemmon-text from hand to return Vemmon-text from trash")
        effect0.set_effect_description("[On Play] By trashing 1 card with [Vemmon] in its text from your hand, you may return 1 non-Digi-Egg card with [Vemmon] in its text from your trash to the hand.")
        effect0.is_on_play = True
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must have a Vemmon-text card in hand to trash
            owner = card.owner if card else None
            if not owner:
                return False
            has_hand = any(_has_vemmon_text(c) for c in owner.hand_cards)
            if not has_hand:
                return False
            # Must have a non-egg Vemmon-text card in trash to return
            has_trash = any(
                _has_vemmon_text(c) and not getattr(c, 'is_digi_egg', False)
                for c in owner.trash_cards
            )
            if not has_trash:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Trash 1 Vemmon-text from hand, then return 1 non-egg Vemmon-text from trash to hand."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            def hand_filter(c):
                return _has_vemmon_text(c)

            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
                # Now return 1 non-egg Vemmon-text from trash to hand
                qualifying = [
                    c for c in player.trash_cards
                    if _has_vemmon_text(c) and not getattr(c, 'is_digi_egg', False)
                ]
                if qualifying:
                    chosen = qualifying[0]
                    player.trash_cards.remove(chosen)
                    player.hand_cards.append(chosen)

            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ─── Effect 1 (Inherited): [Your Turn] When this Digimon with [Vemmon] in its text
        #     would digivolve, reduce the cost by 1.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.WhenWouldDigivolve)
        effect1.set_effect_name("BT21-056 Inherited: Digi cost -1 for Vemmon-text Digimon")
        effect1.set_effect_description("[Your Turn] When this Digimon with [Vemmon] in its text would digivolve, reduce the digivolution cost by 1.")
        effect1.is_inherited_effect = True
        effect1.cost_reduction = 1

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # This Digimon must have [Vemmon] in its text (top card)
            perm = card.permanent_of_this_card()
            if perm is None:
                return False
            top = perm.top_card
            if top is None or not _has_vemmon_text(top):
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
