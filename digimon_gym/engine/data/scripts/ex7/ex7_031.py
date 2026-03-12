from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX7_031(CardScript):
    """EX7-031 Pteromon | Lv.3, Green, Bird Dragon/LIBERATOR, DP 1000, Cost 3

    [Your Turn] When this Digimon would digivolve into a Digimon card with
        [Bird] or [Avian] in any of its traits, reduce the digivolution cost by 1.
    [Inherited][All Turns][Once Per Turn] When this Digimon deletes an
        opponent's Digimon in battle, gain 1 memory.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Your Turn] Digivolve cost -1 into Bird/Avian trait card ---
        # WhenWouldDigivolve cost reduction — engine checks cost_reduction on
        # effects whose condition passes during digivolution cost calculation.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX7-031 Digivolve cost -1 into Bird/Avian trait")
        effect0.set_effect_description(
            "[Your Turn] When this Digimon would digivolve into a Digimon card "
            "with [Bird] or [Avian] in any of its traits, reduce the digivolution "
            "cost by 1."
        )
        effect0.cost_reduction = 1

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check that the digivolving target has Bird or Avian in its traits
            target_card = context.get('digivolving_card')
            if target_card:
                traits = getattr(target_card, 'type_eng', []) or []
                if any('Bird' in t or 'Avian' in t for t in traits):
                    return True
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [Inherited][All Turns][Once Per Turn] Delete in battle → +1 memory ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEndBattle)
        effect1.set_effect_name("EX7-031 Inherited: Delete in battle gains 1 memory")
        effect1.set_effect_description(
            "[Inherited][All Turns][Once Per Turn] When this Digimon deletes an "
            "opponent's Digimon in battle, gain 1 memory."
        )
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("EX7_031_Inherited_BattleDelete")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Gain 1 memory when this Digimon deletes an opponent's Digimon in battle."""
            player = ctx.get('player')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
