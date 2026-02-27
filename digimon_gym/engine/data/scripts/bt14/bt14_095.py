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
        effect0.set_effect_description("[Main] 1 of your opponent's Digimon gains a suspend-triggered -2 memory penalty until end of opponent's turn.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            game = ctx.get('game')
            player = ctx.get('player')
            if game is None or player is None:
                return

            target = None
            if hasattr(game, 'select_opponent_digimon'):
                target = game.select_opponent_digimon(player, 1)
                if isinstance(target, list):
                    target = target[0] if target else None
            elif hasattr(game, 'choose_opponent_digimon'):
                target = game.choose_opponent_digimon(player)

            if target is None:
                return

            temp_effect = ICardEffect()
            temp_effect.set_effect_name("BT14-095 Granted Effect")
            temp_effect.set_effect_description("[All Turns] When this Digimon becomes suspended, lose 2 memory.")

            def granted_condition(event_ctx: Dict[str, Any]) -> bool:
                event_type = event_ctx.get('event') or event_ctx.get('event_type')
                subject = event_ctx.get('permanent') or event_ctx.get('target')
                return event_type in ('BECOME_SUSPENDED', 'becomes_suspended', 'on_suspend') and subject is target

            temp_effect.set_can_use_condition(granted_condition)

            def granted_process(event_ctx: Dict[str, Any]):
                g = event_ctx.get('game') or game
                owner = None
                if hasattr(target, 'owner'):
                    owner = target.owner
                elif hasattr(target, 'get_owner'):
                    owner = target.get_owner()
                if g is not None and owner is not None and hasattr(g, 'change_memory'):
                    g.change_memory(owner, -2)

            temp_effect.set_on_process_callback(granted_process)

            if hasattr(game, 'add_temporary_effect_until_end_of_opponents_turn'):
                game.add_temporary_effect_until_end_of_opponents_turn(player, target, temp_effect)
            elif hasattr(game, 'add_temp_effect'):
                game.add_temp_effect(target, temp_effect, expire='end_of_opponents_turn', source_player=player)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
