from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX7_004(CardScript):
    """EX7-004 Fluffymon | Digi-Egg Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEndBattle
        # [Your Turn][Once Per Turn] When this Digimon deletes your opponent's Digimon
        # in battle, gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEndBattle)
        effect0.set_effect_name("EX7-004 Gain 1 memory when this Digimon deletes opponent in battle")
        effect0.set_effect_description(
            "[Your Turn][Once Per Turn] When this Digimon deletes your opponent's Digimon "
            "in battle, gain 1 memory."
        )
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("Memory+1_EX7_004")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            owner_perm = card.permanent_of_this_card()
            attacker = context.get('attacker')
            defender = context.get('defender')
            # This Digimon must be the attacker
            if owner_perm is None or attacker is None or owner_perm is not attacker:
                return False
            # The defender (opponent's Digimon) must have been deleted (no longer on field)
            if defender is None:
                return False
            opponent = card.owner.enemy if card.owner else None
            if opponent is None:
                return False
            if defender in opponent.battle_area:
                return False  # Defender survived — no deletion
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            if player:
                player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
