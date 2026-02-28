from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_067(CardScript):
    """BT15-067 Ouryumon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blocker
        # Blocker
        effect0 = ICardEffect()
        effect0.set_effect_name("BT15-067 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect1 = ICardEffect()
        effect1.set_effect_name("BT15-067 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 4
        effect1._alt_digi_cost = 4

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns][Once per turn] When this Digimon becomes suspeneded, you may play 1 Digimon card with the [Beast Dragon] or [DigiPolice] trait and 5000DP or less from your hand without paying the cost
        effect2 = ICardEffect()
        effect2.set_effect_name("BT15-067 Play 1 Digimon card with the [Beast Dragon] or [DigiPolice] trait with 5000DP or less from your hand.")
        effect2.set_effect_description("[All Turns][Once per turn] When this Digimon becomes suspeneded, you may play 1 Digimon card with the [Beast Dragon] or [DigiPolice] trait and 5000DP or less from your hand without paying the cost")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("PlayDigimon_BT15_067")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (any('DigiPolice' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If a Tamer card with the [DigiPolice] trait is in this Digimon's digivolution cards, return 1 of your opponent's suspended Digimon or Tamers to the bottom of the deck.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT15-067 Return 1 of your opponent's suspended Digimon or Tamers to the bottom of the deck.")
        effect3.set_effect_description("[When Digivolving] If a Tamer card with the [DigiPolice] trait is in this Digimon's digivolution cards, return 1 of your opponent's suspended Digimon or Tamers to the bottom of the deck.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Bounce"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not (any('DigiPolice' in t for t in (getattr(p.top_card, 'card_traits', []) or []))):
                    return False
                return True
            def on_bounce(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.bounce_permanent_to_hand(target_perm)
            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
