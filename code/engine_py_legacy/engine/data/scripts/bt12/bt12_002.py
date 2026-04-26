from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT12_002(CardScript):
    """BT12-002 DemiVeemon | Lv.2 Digi-Egg (Blue)"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Inherited Effect: [When Attacking] [Once Per Turn]
        # If you have a green Digimon in play, <Draw 1>
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnUseAttack)
        effect0.set_effect_name("BT12-002 Inherited: Draw 1 if green Digimon in play")
        effect0.set_effect_description(
            "[When Attacking][Once Per Turn] If you have a green Digimon in play, "
            "<Draw 1>. (Draw 1 card from your deck.)"
        )
        effect0.is_inherited_effect = True
        effect0.is_on_attack = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("Draw1_BT12_002")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            # Check that the owner has a green Digimon in play
            from ....data.enums import CardColor
            has_green_digimon = any(
                p.is_digimon and p.top_card is not None
                and CardColor.Green in (p.top_card.card_colors or [])
                for p in owner.battle_area
            )
            if not has_green_digimon:
                return False
            # Must have at least 1 card in deck to draw (C#: LibraryCards.Count >= 1)
            if len(owner.library_cards) < 1:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.draw_cards(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
