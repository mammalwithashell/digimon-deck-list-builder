from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_098(CardScript):
    """BT16-098 DORU-Din"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] If you have a Digimon with [Dorugoramon] in its name, delete 1 of your opponent's Digimon or Tamers with a play cost of 4 or less. Then, delete all of your opponent's Digimon with the lowest play cost.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-098 Delete all opponent's digimon with lowest play cost")
        effect0.set_effect_description("[Main] If you have a Digimon with [Dorugoramon] in its name, delete 1 of your opponent's Digimon or Tamers with a play cost of 4 or less. Then, delete all of your opponent's Digimon with the lowest play cost.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not (p.contains_card_name('Dorugoramon')):
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
