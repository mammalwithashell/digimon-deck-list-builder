from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX2_004(CardScript):
    """EX2-004 Gummymon | Lv.2 (Digi-Egg)"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Inherited Effect: [Your Turn] [Once Per Turn] When an opponent's
        # Digimon becomes suspended, <Draw 1>
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnTappedAnyone)
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("EX2_004_InhDraw")
        effect0.set_effect_name("EX2-004 Inherited: Draw 1 when opponent suspends")
        effect0.set_effect_description(
            "[Your Turn] [Once Per Turn] When an opponent's Digimon becomes "
            "suspended, Draw 1."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check that the suspended permanent belongs to the opponent
            suspended_perm = context.get('permanent')
            if suspended_perm is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            enemy = owner.enemy if owner else None
            if not enemy:
                return False
            # The suspended permanent should be an opponent's Digimon
            return suspended_perm in enemy.battle_area and suspended_perm.is_digimon

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            owner = card.owner if card else None
            if owner and game and owner.library_cards:
                owner.draw()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
