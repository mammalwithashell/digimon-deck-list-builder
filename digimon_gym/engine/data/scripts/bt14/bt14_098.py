from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_098(CardScript):
    """Auto-transpiled from DCGO BT14_098.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] <De-Digivolve 1> 1 of your opponent's Digimon. Then, by returning 3 cards with the [D-Brigade] or [DigiPolice] trait from your trash to the top of the deck, delete up to 6 play cost's total worth of your opponent's Digimon.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-098 Delete, De Digivolve")
        effect0.set_effect_description("[Main] <De-Digivolve 1> 1 of your opponent's Digimon. Then, by returning 3 cards with the [D-Brigade] or [DigiPolice] trait from your trash to the top of the deck, delete up to 6 play cost's total worth of your opponent's Digimon.")

        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Delete, De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Delete: target selection needed for full impl
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                target = min(enemy.battle_area, key=lambda p: p.dp)
                enemy.delete_permanent(target)
            # De-digivolve opponent's digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                target = enemy.battle_area[-1]
                removed = target.de_digivolve(1)
                enemy.trash_cards.extend(removed)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
