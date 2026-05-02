from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_177(CardScript):
    """P-177 Gigimon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDestroyedAnyone
        # Add To Hand
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDestroyedAnyone)
        effect0.set_effect_name("P-177 Return 1 card from trash to hand")
        effect0.set_effect_description("Add To Hand")
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.is_on_deletion = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Return 1 card with [Growlmon] or [Gallantmon] from trash to hand."""
            player = ctx.get('player')
            if not player:
                return
            for c in list(player.trash_cards):
                if (c.contains_card_name('Growlmon') or
                    c.contains_card_name('Gallantmon')):
                    player.trash_cards.remove(c)
                    player.hand_cards.append(c)
                    break

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
