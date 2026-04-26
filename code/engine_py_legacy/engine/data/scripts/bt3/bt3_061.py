from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType, ModifierEntry
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT3_061(CardScript):
    """BT3-061 Chuumon | Lv.3 (Black, Cost 3)

    [All Turns] Your opponent can't gain memory other than by Tamer effects.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [All Turns] Opponent can't gain memory except by Tamer effects.
        # Registered as CANNOT_ADD_MEMORY modifier targeting the opponent.
        # The engine enforces this in Player.add_memory().
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT3-061 Opponent can't gain memory except by Tamers")
        effect0.set_effect_description(
            "[All Turns] Your opponent can't gain memory other than by Tamer effects."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            enemy = player.enemy
            if not enemy:
                return
            # Register CANNOT_ADD_MEMORY targeting opponent player
            game.modifiers.register(ModifierEntry(
                modifier_type=ModifierType.CANNOT_ADD_MEMORY,
                condition=lambda t, c, p=perm, e=enemy: (
                    c.get('player') is e and p in (e.enemy.battle_area if e.enemy else [])
                ),
                source_permanent=perm,
                expiry='permanent',
            ))
            game.logger.log("[BT3-061] Opponent can't gain memory (CANNOT_ADD_MEMORY registered)")

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
