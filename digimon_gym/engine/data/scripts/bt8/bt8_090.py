from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_090(CardScript):
    """BT8-090 Kari Kamiya | Tamer | Yellow | Cost 4

    [Start of Your Turn] If you have 2 or less memory, set it to 3.
    [Your Turn] When a card is added to your security stack, you may
        suspend this Tamer to gain 1 memory.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Start of Your Turn] Set memory to 3 if <= 2 ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartTurn)
        effect0.set_effect_name("BT8-090 Set memory to 3")
        effect0.set_effect_description(
            "[Start of Your Turn] If you have 2 or less memory, set it to 3."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and game.memory <= 2:
                game.memory = 3
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Your Turn] When a card is added to YOUR security, suspend for +1 memory ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnAddSecurity)
        effect1.set_effect_name("BT8-090 Memory +1")
        effect1.set_effect_description(
            "[Your Turn] When a card is added to your security stack, you may "
            "suspend this Tamer to gain 1 memory."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Must be this player's security that gained a card
            event_player = context.get('event_player') or context.get('player')
            if event_player and card.owner and event_player is not card.owner:
                return False
            # Tamer must not already be suspended (suspend is cost)
            perm = card.permanent_of_this_card()
            if perm and perm.is_suspended:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Suspend this Tamer, then gain 1 memory."""
            perm = card.permanent_of_this_card() if card else None
            player = ctx.get('player')
            if not (perm and player):
                return
            # Cost: suspend
            perm.suspend()
            # Gain 1 memory
            player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Security: Play this card ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT8-090 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
