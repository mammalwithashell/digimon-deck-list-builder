from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT3_096(CardScript):
    """BT3-096 Mimi Tachikawa | Tamer Cost 2
    [All Turns] When a player uses an Option card, you may suspend this Tamer to gain 1 memory.
    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [All Turns] When a player uses an Option card, suspend to gain 1 memory
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnUseOption)
        effect0.set_effect_name("BT3-096 Suspend to gain 1 memory when Option used")
        effect0.set_effect_description(
            "[All Turns] When a player uses an Option card, you may suspend "
            "this Tamer to gain 1 memory."
        )
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if perm and perm.is_suspended:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return
            perm = card.permanent_of_this_card() if card else None
            if perm:
                perm.suspend()
                player.add_memory(1)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Security] Play this card without paying the cost.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("BT3-096 Security: Play this card free")
        effect1.set_effect_description("[Security] Play this card without paying the cost.")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and card:
                player.play_card_from_source(card, pay_cost=False)
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
