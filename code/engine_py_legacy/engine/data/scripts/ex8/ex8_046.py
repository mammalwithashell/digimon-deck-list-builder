from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_046(CardScript):
    """EX8-046 Gotsumon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Deletion] By trashing 1 card with the [Mineral] or [Rock] trait in your hand,
        # <Draw 2> (Draw 2 cards from your deck.)
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDestroyedAnyone)
        effect0.set_effect_name("EX8-046 By trashing 1 [Mineral] or [Rock] hand card, Draw 2")
        effect0.set_effect_description(
            "[On Deletion] By trashing 1 card with the [Mineral] or [Rock] trait in your hand, "
            "<Draw 2> (Draw 2 cards from your deck.)"
        )
        effect0.is_optional = True
        effect0.is_on_deletion = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Engine only calls this from deleted_permanent.card_sources — this Digimon was deleted.
            # Check that there is at least one [Mineral] or [Rock] trait card in hand to pay the cost.
            player = context.get('player')
            if player is None:
                return False
            has_valid = any(
                'Mineral' in getattr(c, 'card_traits', []) or 'Rock' in getattr(c, 'card_traits', [])
                for c in player.hand_cards
            )
            return has_valid

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def hand_filter(c) -> bool:
                traits = getattr(c, 'card_traits', [])
                return 'Mineral' in traits or 'Rock' in traits

            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
                    # Draw 2 only as a result of paying the cost
                    player.draw_cards(2)

            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Inherited: <Blocker> (This Digimon can block in the blocker timing.)
        effect1 = ICardEffect()
        effect1.set_effect_name("EX8-046 Blocker")
        effect1.set_effect_description("<Blocker> (This Digimon can block in the blocker timing.)")
        effect1.is_inherited_effect = True
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
