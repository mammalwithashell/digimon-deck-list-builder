from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_170(CardScript):
    """P-170 AvengeKidmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("P-170 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 4
        effect0._alt_digi_cost = 4
        effect0._alt_digi_level = 5

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Three Musketeers' in text):
                    return False
            else:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played, by returning 3 cards with [Three Musketeers] in their texts from your trash to the bottom of the deck, reduce the play cost by 6.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name("P-170 Return 3 [Three Musketeers] to get Play Cost -6")
        effect1.set_effect_description("When this card would be played, by returning 3 cards with [Three Musketeers] in their texts from your trash to the bottom of the deck, reduce the play cost by 6.")
        effect1.set_hash_string("PlayCost-6_P_170")
        effect1.cost_reduction = 6

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Cost -6, Return To Deck"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction by 6 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_return(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.return_permanent_to_deck_bottom(target_perm)
            game.effect_select_opponent_permanent(
                player, on_return, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.None
        # Cost -6
        effect2 = ICardEffect()
        effect2.set_effect_name("P-170 Play Cost -1")
        effect2.set_effect_description("Cost -6")
        effect2.cost_reduction = 6

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Cost -6"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction by 6 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: raid
        # Raid
        effect3 = ICardEffect()
        effect3.set_effect_name("P-170 Raid")
        effect3.set_effect_description("Raid")
        effect3.is_on_attack = True
        effect3._is_raid = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Factory effect: retaliation
        # Retaliation
        effect4 = ICardEffect()
        effect4.set_effect_name("P-170 Retaliation")
        effect4.set_effect_description("Retaliation")
        effect4.is_on_deletion = True
        effect4._is_retaliation = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Factory effect: blocker
        # Blocker
        effect5 = ICardEffect()
        effect5.set_effect_name("P-170 Blocker")
        effect5.set_effect_description("Blocker")
        effect5._is_blocker = True

        def condition5(context: Dict[str, Any]) -> bool:
            return True
        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] You may play 1 [Three Musketeers] trait Digimon card with a play cost of 12 or less from your hand or trash without paying the cost.
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.OnDestroyedAnyone)
        effect6.set_effect_name("P-170 Play 1 [Three Musketeers] trait Digimon from hand/trash")
        effect6.set_effect_description("[On Deletion] You may play 1 [Three Musketeers] trait Digimon card with a play cost of 12 or less from your hand or trash without paying the cost.")
        effect6.is_optional = True
        effect6.is_on_deletion = True

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'has_play_cost', False):
                    return False
                if getattr(c, 'get_cost_itself', 0) > 12:
                    return False
                if not (any('Three Musketeers' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True)

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
