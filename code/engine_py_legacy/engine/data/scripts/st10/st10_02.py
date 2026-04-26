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
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            perm = card.permanent_of_this_card()
            if not perm:
                return False
            owner = card.owner
            # Need at least 1 OTHER Digimon on field as second DNA material
            # ("This Digimon and any of your other Digimon may DNA digivolve")
            other_digimon = [
                p for p in owner.battle_area
                if p is not perm and p.is_digimon
            ]
            if not other_digimon:
                return False
            # Need at least 1 Digimon card in hand as the DNA target
            hand_digimon = [
                c for c in owner.hand_cards
                if getattr(c, 'is_digimon', False)
            ]
            if not hand_digimon:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Trigger DNA digivolve from hand at end of turn.

            The engine's effect_dna_digivolve_from_hand already filters
            candidates down to cards with dna_costs and verifies that valid
            DNA materials exist, so this script just forwards the call with
            an open filter.
            """
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            game.effect_dna_digivolve_from_hand(
                player,
                filter_fn=lambda c: True,
                is_optional=True,
                prompt="Select a card from hand to DNA digivolve (end of turn).",
            )
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
