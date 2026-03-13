from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT18_009(CardScript):
    """BT18-009 Shamanmon | Lv.3 Red (Demon)

    [All Turns] While this Digimon is in play, your opponent can't gain
    memory from their Digimon's effects.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [All Turns] Opponent can't gain memory from
        #     their Digimon's effects ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT18-009 Opponent can't gain memory from Digimon effects")
        effect0.set_effect_description(
            "[All Turns] While this Digimon is in play, your opponent "
            "can't gain memory from their Digimon's effects."
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

        # The engine checks _cannot_gain_memory_condition on declarative
        # effects to block memory gain. The condition receives the player
        # attempting to gain memory and the source card effect.
        # Block opponent gaining memory from non-Tamer effects (i.e. Digimon effects).
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
            # Only block if the source effect is from a non-Tamer card
            # (C# checks !IsTamerEffect)
            if source_effect is not None:
                if getattr(source_effect, 'effect_source_card', None) is not None:
                    if not getattr(source_effect, 'is_tamer_effect', False):
                        return True
            return False

        effect0._cannot_gain_memory_condition = cannot_gain_memory_check

        effects.append(effect0)

        return effects
