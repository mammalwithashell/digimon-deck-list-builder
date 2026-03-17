from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_093(CardScript):
    """BT22-093 Ami Aiba"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("BT22-093 Memory +1")
        effect0.set_effect_description("[Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            enemy = card.owner.enemy if card and card.owner else None
            return bool(enemy and any(p.is_digimon for p in enemy.battle_area))

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Your Turn] When any of your Digimon digivolve into a Digimon with the [CS] trait,
        # if it has a digivolution card with the same level as the digivolved Digimon,
        # by suspending this Tamer, that Digimon may digivolve into a Digimon card with
        # the [CS] trait in the hand without paying the cost.
        # Uses _is_digivolve_observer pattern (fires from _fire_digivolve_observers)
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-093 Suspend tamer, digivolve CS Digimon free")
        effect1.set_effect_description("[Your Turn] When any of your Digimon digivolve into a Digimon with the [CS] trait, if it has a digivolution card with the same level as the digivolved Digimon, by suspending this Tamer, that Digimon may digivolve into a Digimon card with the [CS] trait in the hand without paying the cost.")
        effect1.is_optional = True
        effect1._is_digivolve_observer = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Tamer must be unsuspended (suspending is the cost)
            if tamer_perm.is_suspended:
                return False
            # The digivolved permanent must have [CS] trait
            digi_perm = context.get('digivolved_permanent')
            if not digi_perm or not digi_perm.top_card:
                return False
            traits = getattr(digi_perm.top_card, 'card_traits', []) or []
            if not any('CS' in t for t in traits):
                return False
            # Must belong to us
            player = card.owner if card else None
            if player and digi_perm not in player.battle_area:
                return False
            # Check: has a digivolution card with the same level as the digivolved Digimon
            digi_level = digi_perm.top_card.level if digi_perm.top_card else None
            if digi_level is not None:
                has_same_level = any(
                    getattr(cs, 'level', None) == digi_level
                    for cs in digi_perm.digivolution_cards
                )
                if not has_same_level:
                    return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend this Tamer, then digivolve the triggering Digimon into a CS Digimon from hand free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Cost: suspend this Tamer
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm and not tamer_perm.is_suspended:
                tamer_perm.suspend()
            # Digivolve the triggering permanent into a CS Digimon from hand free
            digi_perm = ctx.get('digivolved_permanent')
            if not digi_perm:
                return
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('CS' in t for t in traits)
            game.effect_digivolve_from_hand(
                player, digi_perm, digi_filter, cost_override=0, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT22-093 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
