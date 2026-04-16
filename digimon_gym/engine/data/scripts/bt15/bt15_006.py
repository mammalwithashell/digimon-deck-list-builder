from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_006(CardScript):
    """BT15-006 DemiMeramon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] By trashing 1 level 5 or higher Digimon card in your hand, <Draw 2>.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDestroyedAnyone)
        effect0.set_effect_name("BT15-006 Trash 1 card from hand to Draw 2")
        effect0.set_effect_description("[On Deletion] By trashing 1 level 5 or higher Digimon card in your hand, <Draw 2>.")
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.is_on_deletion = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Trash 1 Lv.5+ Digimon from hand, then Draw 2."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            def hand_filter(c):
                # Must be a Digimon card with level >= 5
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level < 5:
                    return False
                return True

            # Check if any valid targets exist before entering selection
            valid_targets = [c for c in player.hand_cards if hand_filter(c)]
            if not valid_targets:
                return

            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
                    # Draw 2 only after successfully trashing (pay-before-reward)
                    player.draw_cards(2)

            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
