from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST10_02(CardScript):
    """ST10-02 Salamon | Lv.3 Yellow Digimon

    Inherited Effect [End of Your Turn] This Digimon and any of your other
    Digimon may DNA digivolve into a Digimon card in the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Inherited [End of Your Turn] DNA digivolve from hand ---
        # This is an inherited effect that enables DNA digivolution at end of turn.
        # The engine handles DNA digivolution via effect_dna_digivolve_from_hand.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEndTurn)
        effect0.set_effect_name("ST10-02 End of turn DNA digivolve")
        effect0.set_effect_description(
            "[End of Your Turn] This Digimon and any of your other Digimon "
            "may DNA digivolve into a Digimon card in the hand."
        )
        effect0.is_inherited_effect = True
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Trigger DNA digivolve from hand at end of turn."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Allow DNA digivolve from hand — any Digimon card with DNA costs
            def dna_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                entity = getattr(c, 'c_entity_base', None)
                if entity and entity.dna_costs:
                    return True
                return False
            game.effect_dna_digivolve_from_hand(player, dna_filter, is_optional=True)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
