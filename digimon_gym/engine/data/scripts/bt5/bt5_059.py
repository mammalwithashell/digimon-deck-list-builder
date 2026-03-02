from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT5_059(CardScript):
    """BT5-059 Keramon | Lv.3 Rookie | Black | Unidentified

    [On Play] Reveal 5 cards from the top of your deck. Add 1 Digimon
        card with the [Unidentified] trait and 1 [Arata Sanada] card among
        them to your hand. Place the remaining cards at the bottom of your
        deck in any order.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [On Play] Reveal top 5, add Unidentified + Arata Sanada ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT5-059 On Play: Reveal 5, add Unidentified + Arata Sanada")
        effect0.set_effect_description(
            "[On Play] Reveal 5 cards from the top of your deck. Add 1 Digimon "
            "card with the [Unidentified] trait and 1 [Arata Sanada] card among "
            "them to your hand. Place the remaining cards at the bottom of your "
            "deck in any order."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Reveal top 5, add 1 Unidentified Digimon + 1 Arata Sanada to hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Reveal top 5
            revealed = []
            for _ in range(min(5, len(player.library_cards))):
                revealed.append(player.library_cards.pop(0))

            added = []
            # Add 1 Digimon with [Unidentified] trait
            for c in revealed:
                if c not in added and c.is_digimon:
                    if 'Unidentified' in (c.card_traits or []):
                        added.append(c)
                        break

            # Add 1 [Arata Sanada] card
            for c in revealed:
                if c not in added:
                    name = c.c_entity_base.card_name_eng if c.c_entity_base else ''
                    if 'Arata Sanada' in name:
                        added.append(c)
                        break

            for c in added:
                player.hand_cards.append(c)
            # Remaining go to bottom of deck
            remaining = [c for c in revealed if c not in added]
            for c in remaining:
                player.library_cards.append(c)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
