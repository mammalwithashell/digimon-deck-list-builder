from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_012(CardScript):
    """BT24-012 Dimetromon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blocker
        # Blocker
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-012 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.WhenPermanentWouldBeDeleted
        # [All Turns] When any of your other Digimon with the [Reptile] or [Dragonkin] trait would leave the battle area by your opponent's effects, by returning this Digimon to the hand, they don't leave.
        # Pattern: WhenPermanentWouldBeDeleted + _will_not_be_removed flag (matches BT24-030 Neptunemon)
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.WhenPermanentWouldBeDeleted)
        effect1.set_effect_name("BT24-012 By bouncing to hand, others don't leave")
        effect1.set_effect_description("[All Turns] When any of your other Digimon with the [Reptile] or [Dragonkin] trait would leave the battle area by your opponent's effects, by returning this Digimon to the hand, they don't leave.")
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            # This Dimetromon must be on the field
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be triggered by opponent's effect
            if not context.get('is_opponent_effect', False):
                return False
            # The leaving permanent (mapped to event_permanent by execute_effects)
            leaving_perm = context.get('event_permanent')
            if leaving_perm is None:
                return False
            # The leaving permanent must belong to this card's owner
            owner = getattr(card, 'owner', None)
            event_player = context.get('event_player')
            if not owner or not event_player or event_player is not owner:
                return False
            # The leaving permanent must have Reptile or Dragonkin trait
            if not (leaving_perm.has_trait('Reptile') or leaving_perm.has_trait('Dragonkin')):
                return False
            # The leaving permanent must NOT be this Dimetromon (must be an "other" Digimon)
            dimetromon_perm = card.permanent_of_this_card()
            if leaving_perm is dimetromon_perm:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Return Dimetromon to hand to prevent ally from leaving"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Return THIS Dimetromon to hand via engine API
            dimetromon_perm = card.permanent_of_this_card() if card else None
            if dimetromon_perm:
                player.bounce_permanent_to_hand(dimetromon_perm)
            # Prevent the target (leaving_perm) from leaving (DCGO: willBeRemoveField = false)
            leaving_perm = ctx.get('event_permanent')
            if leaving_perm:
                leaving_perm._will_not_be_removed = True

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnLoseSecurity
        # Gain 1 memory
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnLoseSecurity)
        effect2.set_effect_name("BT24-012 Gain 1 memory")
        effect2.set_effect_description("Gain 1 memory")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("GainMemory_BT24-012")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # "When your opponent's security stack is removed from" —
            # event_player is the player who lost security; must be the opponent
            event_player = context.get('event_player')
            if event_player is None or event_player is card.owner:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            perm = ctx.get('permanent')
            game = ctx.get('game')
            owner = card.owner if card else ctx.get('player')
            if owner:
                owner.add_memory(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
