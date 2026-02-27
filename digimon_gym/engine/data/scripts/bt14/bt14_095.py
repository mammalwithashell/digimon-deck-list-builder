from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_095(CardScript):
    """BT14-095 Poison Ivy"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] 1 of your opponent's Digimon gains
        # "[All Turns] When this Digimon becomes suspended, lose 2 memory."
        # until the end of your opponent's turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-095 Effect")
        effect0.set_effect_description("[Main] 1 of your opponent's Digimon gains \"[All Turns] When this Digimon becomes suspended, lose 2 memory.\" until the end of your opponent's turn.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            target = ctx.get('target')
            permanent = ctx.get('permanent')

            if game is None or player is None:
                return

            # Resolve target: prefer preselected target from context.
            if target is None:
                opponents = []
                if hasattr(game, 'get_opponents'):
                    opponents = game.get_opponents(player) or []
                elif hasattr(game, 'players'):
                    opponents = [p for p in (game.players or []) if p is not player]

                opponent_digimon = []
                for opp in opponents:
                    field = getattr(opp, 'battle_area', None)
                    if field is None:
                        field = getattr(opp, 'digimon', None)
                    if field:
                        opponent_digimon.extend([d for d in field if d is not None])

                if not opponent_digimon:
                    return
                target = opponent_digimon[0]

            # Apply the embedded triggered effect until end of opponent's turn.
            temp_effect = {
                'effect_name': 'BT14-095 Memory -2',
                'effect_description': '[All Turns] When this Digimon becomes suspended, lose 2 memory.',
                'trigger': 'on_become_suspended',
                'memory_delta': -2,
                'duration': 'until_end_of_opponents_turn',
                'source': permanent,
                'source_card_id': 'BT14-095',
            }

            if hasattr(game, 'add_temp_effect'):
                game.add_temp_effect(target, temp_effect, owner=player)
                return

            if hasattr(target, 'add_temp_effect'):
                target.add_temp_effect(temp_effect)
                return

            if not hasattr(target, 'temp_effects') or target.temp_effects is None:
                target.temp_effects = []
            target.temp_effects.append(temp_effect)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
