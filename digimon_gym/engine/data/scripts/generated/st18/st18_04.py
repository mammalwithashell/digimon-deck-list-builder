from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from .....core.card_script import CardScript
from .....interfaces.card_effect import ICardEffect
from .....data.enums import EffectTiming

if TYPE_CHECKING:
    from .....core.card_source import CardSource


class ST18_04(CardScript):
    """ST18-04 Pteromon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] Reveal the top 3 cards of your deck. Add 1 card with [Bird]
        # or [Avian] in any of its traits and 1 card with the [Vortex Warriors]
        # or [LIBERATOR] trait from among them to your hand. Place the rest at
        # the bottom of your deck in any order.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("ST18-04 Reveal top 3, add Bird/Avian and Vortex Warriors/LIBERATOR")
        effect0.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Add 1 card with "
            "[Bird] or [Avian] in any of its traits and 1 card with the "
            "[Vortex Warriors] or [LIBERATOR] trait from among them to your hand. "
            "Place the rest at the bottom of your deck in any order."
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
            revealed = player.reveal_top_cards(3)
            if not revealed:
                return

            added = 0
            remaining = list(revealed)

            # Add 1 Bird/Avian card
            for c in remaining:
                traits = getattr(c, 'card_traits', []) or []
                if any('Bird' in t or 'Avian' in t for t in traits):
                    player.add_to_hand(c)
                    remaining.remove(c)
                    added += 1
                    break

            # Add 1 Vortex Warriors/LIBERATOR card
            for c in remaining:
                traits = getattr(c, 'card_traits', []) or []
                if any('Vortex Warriors' in t or 'LIBERATOR' in t for t in traits):
                    player.add_to_hand(c)
                    remaining.remove(c)
                    added += 1
                    break

            # Place rest at bottom
            for c in remaining:
                player.place_at_deck_bottom(c)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Inherited: [Your Turn] This Digimon gets +2000 DP.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.StatModifier)
        effect1.set_effect_name("ST18-04 +2000 DP")
        effect1.set_effect_description("[Your Turn] This Digimon gets +2000 DP.")
        effect1.is_inherited_effect = True
        effect1.dp_modifier = 2000

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
