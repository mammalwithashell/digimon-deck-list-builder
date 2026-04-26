from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT1_090(CardScript):
    """BT1-090 Gravity Crush

    [Main] Gain 2 memory. At end of turn, lose 2 memory.

    C# ref: AddEffectToPlayer with OnEndTurn timing and UntilEachTurnEnd
    duration. The -2 memory fires as a player-level OnEndTurn effect that
    lasts until end of turn (fires once then expires).
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Gain 2 memory. At end of turn, lose 2 memory.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT1-090 Gain 2 memory")
        effect0.set_effect_description("[Main] Gain 2 memory. At end of turn, lose 2 memory.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        # Track whether the -2 memory effect should fire
        _pending_loss = [False]

        def process0(ctx: Dict[str, Any]):
            """Action: Gain 2 memory, schedule -2 at end of turn"""
            player = ctx.get('player')
            if player:
                player.add_memory(2)
                _pending_loss[0] = True

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # OnEndTurn effect: lose 2 memory
        # This fires from trash (engine scans trash for OnEndTurn effects).
        # The _pending_loss flag ensures it only fires if the Main effect was used.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEndTurn)
        effect1.set_effect_name("BT1-090 End of turn: lose 2 memory")
        effect1.set_effect_description("At end of turn, lose 2 memory.")

        def condition1(context: Dict[str, Any]) -> bool:
            if not _pending_loss[0]:
                return False
            # Must be in trash (option goes to trash after resolving)
            player = card.owner if card else None
            if not player:
                return False
            if card not in player.trash_cards:
                return False
            # Only on own turn's end
            if not player.is_my_turn:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Lose 2 memory"""
            player = ctx.get('player')
            if player:
                player.add_memory(-2)
            # One-shot: disable after firing
            _pending_loss[0] = False

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
