from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST19_01(CardScript):
    """ST19-01 Kyaromon | Lv.2 (Digi-Egg, Yellow, Lesser/LIBERATOR)

    Inherited Effect:
    [When Attacking] [Once Per Turn] If you have another Digimon, <Draw 1>.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Inherited: [When Attacking] [Once Per Turn] Draw 1 if another Digimon exists ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnAllyAttack)
        effect0.set_effect_name("ST19-01 When Attacking: Draw 1")
        effect0.set_effect_description(
            "[When Attacking] [Once Per Turn] If you have another Digimon, "
            "<Draw 1>."
        )
        effect0.is_inherited_effect = True
        effect0.is_on_attack = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("Draw_ST19_01")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            my_perm = card.permanent_of_this_card()
            # Must have another Digimon on the field
            other_digimon = [
                p for p in owner.battle_area
                if p.is_digimon and p is not my_perm
            ]
            return len(other_digimon) > 0

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Draw 1 card."""
            player = ctx.get('player')
            if player:
                player.draw_cards(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
