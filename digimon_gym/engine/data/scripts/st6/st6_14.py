from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST6_14(CardScript):
    """ST6-14 Matt Ishida | Tamer Purple Cost 2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Your Turn] When one of your Digimon is deleted,
        #     you may suspend this Tamer to gain 1 memory. ---
        # Uses _is_deletion_observer pattern: fires via _fire_deletion_observers
        # when OTHER permanents are deleted (not OnDestroyedAnyone self-deletion).
        effect0 = ICardEffect()
        effect0.set_effect_name("ST6-14 Suspend to gain 1 memory on own Digimon deletion")
        effect0.set_effect_description(
            "[Your Turn] When one of your Digimon is deleted, you may "
            "suspend this Tamer to gain 1 memory."
        )
        effect0.is_optional = True
        effect0._is_deletion_observer = True

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
            # [Your Turn] only
            if not player.is_my_turn:
                return False
            # The deleted permanent must be one of our Digimon
            deleted = context.get('deleted_permanent')
            if deleted is None:
                return False
            if not getattr(deleted, 'is_digimon', False):
                return False
            # Must be OUR Digimon, not opponent's
            deleted_owner = getattr(deleted, 'owner', None)
            if deleted_owner is not player:
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
        effect1.set_effect_name("ST6-14 Security: Play this card")
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
                player.play_card_from_source(card, pay_cost=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
