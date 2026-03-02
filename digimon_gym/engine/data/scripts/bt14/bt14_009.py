from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_009(CardScript):
    """BT14-009 Gotsumon | Lv.3 Red Rock Digimon

    [All Turns] Players can't play Digimon by effects.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: None (continuous restriction)
        # [All Turns] Players can't play Digimon by effects.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-009 Players can't play Digimon by effects")
        effect0.set_effect_description("[All Turns] Players can't play Digimon by effects.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Restriction"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Play restriction (CanNotPutFieldClass) — both players cannot
            # play Digimon by effects while this Digimon is on the field
            pass  # descriptive-tagged

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
