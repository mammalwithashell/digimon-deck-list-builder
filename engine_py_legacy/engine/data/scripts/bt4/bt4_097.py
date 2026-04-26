from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT4_097(CardScript):
    """BT4-097 Kari Kamiya | Tamer Purple Cost 3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [All Turns] When a card is removed from your security
        #     stack, by suspending this Tamer, gain 1 memory. ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnLoseSecurity)
        effect0.set_effect_name("BT4-097 Suspend to gain 1 memory on security loss")
        effect0.set_effect_description(
            "[All Turns] When a card is removed from your security stack, "
            "by suspending this Tamer, gain 1 memory."
        )
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            # Must not already be suspended (cost is suspend)
            if perm.is_suspended:
                return False
            player = card.owner if card else None
            if not player:
                return False
            # [All Turns] — triggers on both turns, but only for our security
            # The event_player / player in context should be card's owner
            event_player = context.get('event_player') or context.get('player')
            if event_player is None or event_player is not player:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Cost: suspend this Tamer. Reward: gain 1 memory."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if not (player and perm):
                return
            # Cost: suspend this Tamer
            perm.suspend()
            # Reward: gain 1 memory
            player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Security] Play this card without paying the cost. ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("BT4-097 Security: Play this card")
        effect1.set_effect_description(
            "[Security] Play this card without paying the cost."
        )
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Play this tamer from security without paying cost."""
            player = ctx.get('player')
            if player and card:
                card._security_played = True
                player.play_card_from_source(card, pay_cost=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
