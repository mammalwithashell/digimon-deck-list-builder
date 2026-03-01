from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_081(CardScript):
    """BT23-081 Chitose Imai"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may play 1 play cost 5 or lower Digimon card with the [Hudie] trait from your hand without paying the cost.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT23-081 play 1 play cost 5- [Hudie] trait digimon from hand")
        effect0.set_effect_description("[On Play] You may play 1 play cost 5 or lower Digimon card with the [Hudie] trait from your hand without paying the cost.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not getattr(c, 'has_play_cost', False):
                    return False
                if getattr(c, 'get_cost_itself', 0) > 5:
                    return False
                if not (any('Hudie' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns] When any of your [Hudie] trait Digimon suspend, by suspending this Tamer, 1 of your opponent's Digimon gets -3000 DP for the turn.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnTappedAnyone)
        effect1.set_effect_name("BT23-081 By suspending this tamer, -3K DP 1 digimon")
        effect1.set_effect_description("[All Turns] When any of your [Hudie] trait Digimon suspend, by suspending this Tamer, 1 of your opponent's Digimon gets -3000 DP for the turn.")
        effect1.is_optional = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return False
            # Chitose must be unsuspended (suspending is the cost)
            if tamer_perm.is_suspended:
                return False
            # The suspended permanent must be one of your [Hudie] trait Digimon
            tapped_perm = context.get('permanent')
            if not tapped_perm:
                return False
            # Must be our Digimon, not opponent's
            player = card.owner
            if player and tapped_perm not in player.battle_area:
                return False
            if not tapped_perm.is_digimon:
                return False
            traits = getattr(tapped_perm.top_card, 'card_traits', []) or []
            if not any('Hudie' in t for t in traits):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend Chitose as cost, then give -3000 DP to 1 opponent Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Suspend Chitose as cost
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm and not tamer_perm.is_suspended:
                tamer_perm.suspend()
            # Select 1 opponent Digimon to give -3000 DP
            def dp_filter(p):
                return p.is_digimon and p.dp is not None
            def on_dp_reduce(target_perm):
                target_perm.change_dp(-3000)
            game.effect_select_opponent_permanent(
                player, on_dp_reduce, filter_fn=dp_filter, is_optional=False,
                prompt="Select 1 opponent Digimon to give -3000 DP.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-081 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
