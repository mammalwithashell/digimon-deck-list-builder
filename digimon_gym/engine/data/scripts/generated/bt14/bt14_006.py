from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_006(CardScript):
    """BT14-006 Bowmon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDiscardHand
        # [Your Turn] When a Digimon card with the [Dark Animal] or [SoC] trait is trashed from your hand, this Digimon may digivolve into that card.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-006 This Digimon digivolves into discarded card")
        effect0.set_effect_description("[Your Turn] When a Digimon card with the [Dark Animal] or [SoC] trait is trashed from your hand, this Digimon may digivolve into that card.")
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.set_hash_string("Digivolve_BT14_006")

        def _has_required_trait(c: Any) -> bool:
            traits = getattr(c, 'card_traits', []) or []
            return any('Dark Animal' in t or 'DarkAnimal' in t or 'SoC' in t for t in traits)

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False

            trashed = context.get('trashed_card') or context.get('card') or context.get('target_card')
            if trashed is None:
                return False

            if getattr(trashed, 'card_kind', None) != 0:
                return False

            if not _has_required_trait(trashed):
                return False

            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            trashed = ctx.get('trashed_card') or ctx.get('card') or ctx.get('target_card')
            if trashed is None:
                return

            def digi_filter(c):
                return c is trashed and _has_required_trait(c)

            game.effect_digivolve_from_hand(player, perm, digi_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
