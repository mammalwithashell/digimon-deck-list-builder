from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_078(CardScript):
    """Auto-transpiled from DCGO BT24_078.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # Digivolve
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-078 Digivolve into this to trash top opponent security")
        effect0.set_effect_description("Digivolve")
        effect0.is_optional = True
        effect0.is_on_attack = True

        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            pass  # TODO: digivolve effect needs card selection

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Delete, Play Card
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-078 Delete opponent's lowest level, play a Digimon")
        effect1.set_effect_description("Delete, Play Card")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete, Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Delete: target selection needed for full impl
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                target = min(enemy.battle_area, key=lambda p: p.dp)
                enemy.delete_permanent(target)
            # Play a card (from hand/trash/reveal)
            pass  # TODO: target selection for play_card

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
