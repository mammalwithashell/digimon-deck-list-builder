from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT3_046(CardScript):
    """BT3-046 Terriermon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [All Turns] Your opponent can't gain memory other than by Tamer effects.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT3-046 Opponent can't gain memory except by Tamer effects")
        effect0.set_effect_description(
            "[All Turns] Your opponent can't gain memory other than by Tamer effects."
        )
        effect0.is_declarative = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if perm is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            if perm not in owner.battle_area:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def cannot_gain_memory_check(player, source_effect):
            """Return True if this memory gain should be blocked."""
            owner = card.owner if card else None
            if not owner:
                return False
            # Only block the opponent
            if player is owner:
                return False
            if player != owner.enemy:
                return False
            # Allow Tamer effects to grant memory
            if source_effect is not None:
                if getattr(source_effect, 'is_tamer_effect', False):
                    return False
                # Check if source card is a Tamer
                src_card = getattr(source_effect, 'effect_source_card', None)
                if src_card and getattr(src_card, 'is_tamer', False):
                    return False
            # Block all non-Tamer memory gain
            return True

        effect0._cannot_gain_memory_condition = cannot_gain_memory_check

        effects.append(effect0)

        return effects
