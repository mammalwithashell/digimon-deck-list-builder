from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_069(CardScript):
    """BT15-069 Candlemon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] If your opponent has 1 or less memory, <Draw 1>. If your opponent has 1 or more memory, gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDestroyedAnyone)
        effect0.set_effect_name("BT15-069 Draw 1 and/or gain Memory +1")
        effect0.set_effect_description("[On Deletion] If your opponent has 1 or less memory, <Draw 1>. If your opponent has 1 or more memory, gain 1 memory.")
        effect0.is_on_deletion = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1 if opponent has <=1 memory, gain 1 if opponent has >=1."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            # "opponent has X memory" means the memory gauge is on opponent's side
            # In the engine, memory is from P1 perspective; positive = P1's, negative = P2's
            # We check if opponent has 1 or less memory (their memory)
            if player.player_id == 1:
                opponent_memory = -game.memory
            else:
                opponent_memory = game.memory
            # If opponent has 1 or less memory, Draw 1
            if opponent_memory <= 1:
                player.draw_cards(1)
            # If opponent has 1 or more memory, gain 1
            if opponent_memory >= 1:
                player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
