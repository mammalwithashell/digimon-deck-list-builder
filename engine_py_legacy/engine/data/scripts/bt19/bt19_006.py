from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_006(CardScript):
    """BT19-006 Pagumon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] If deleted other than by battle, return 1 level 3 purple Digimon card from your trash to the hand.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDestroyedAnyone)
        effect0.set_effect_name("BT19-006 Return 1 level 3 purple Digimon card from your trash")
        effect0.set_effect_description("[On Deletion] If deleted other than by battle, return 1 level 3 purple Digimon card from your trash to the hand.")
        effect0.is_inherited_effect = True
        effect0.is_on_deletion = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Only if deleted other than by battle
            deleted_by_battle = context.get('deleted_by_battle', False)
            if deleted_by_battle:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Return 1 level 3 purple Digimon from trash to hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not player:
                return
            from ....data.enums import CardColor
            for c in list(player.trash_cards):
                if (getattr(c, 'is_digimon', False) and
                    getattr(c, 'level', None) == 3 and
                    CardColor.Purple in (getattr(c, 'card_colors', []) or [])):
                    player.trash_cards.remove(c)
                    player.hand_cards.append(c)
                    break

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
