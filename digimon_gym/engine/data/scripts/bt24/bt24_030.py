from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_030(CardScript):
    """BT24-030 Neptunemon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-030 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.5 with [TS] trait for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        effect0._alt_digi_trait = "TS"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played, if your opponent has 2 or more Digimon, reduce the play cost by 5.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name("BT24-030 Reduce play cost (5)")
        effect1.set_effect_description("When this card would be played, if your opponent has 2 or more Digimon, reduce the play cost by 5.")
        effect1.cost_reduction = 5

        def condition1(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            owner = getattr(card, 'owner', None)
            if not owner:
                return False
            enemy = owner.enemy if owner else None
            if not enemy:
                return False
            return len([p for p in enemy.battle_area if p.is_digimon]) >= 2

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Return all of your opponent's Digimon with the fewest digivolution cards to the bottom of the deck.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT24-030 Bottom deck all opponent digimon with lowest digivolution cards")
        effect3.set_effect_description("[On Play] Return all of your opponent's Digimon with the fewest digivolution cards to the bottom of the deck.")
        effect3.is_on_play = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        def _neptunemon_bottom_deck(player, game):
            """Return all opponent's Digimon with fewest digivolution cards to bottom of deck."""
            enemy = player.enemy if player else None
            if not enemy:
                return
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            if not opp_digimon:
                return
            min_sources = min(len(p.card_sources) for p in opp_digimon)
            to_remove = [p for p in opp_digimon if len(p.card_sources) == min_sources]
            for target in to_remove:
                if target in enemy.battle_area:
                    enemy.return_permanent_to_deck_bottom(target)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                _neptunemon_bottom_deck(player, game)

        effect3.set_on_process_callback(process3)
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Return all of your opponent's Digimon with the fewest digivolution cards to the bottom of the deck.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT24-030 Bottom deck all opponent digimon with lowest digivolution cards")
        effect4.set_effect_description("[When Digivolving] Return all of your opponent's Digimon with the fewest digivolution cards to the bottom of the deck.")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                _neptunemon_bottom_deck(player, game)

        effect4.set_on_process_callback(process4)
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns] [Once Per Turn] When this Digimon suspends, it may unsuspend.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnTappedAnyone)
        effect5.set_effect_name("BT24-030 Unsuspend this digimon")
        effect5.set_effect_description("[All Turns] [Once Per Turn] When this Digimon suspends, it may unsuspend.")
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("BT24_030_AT")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Unsuspend this Digimon"""
            perm = ctx.get('permanent')
            if perm:
                perm.unsuspend()

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When any of your Digimon with the [TS] trait or [Aqua] or [Sea Animal] in any of their traits would leave the battle area by your opponent's effects, by suspending this Digimon, they don't leave.
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.WhenRemoveField)
        effect6.set_effect_name("BT24-030 By suspending this digimon, your [TS]/[Aqua]/[Sea Animal] digimon wont leave the field")
        effect6.set_effect_description("[All Turns] When any of your Digimon with the [TS] trait or [Aqua] or [Sea Animal] in any of their traits would leave the battle area by your opponent's effects, by suspending this Digimon, they don't leave.")
        effect6.is_optional = True

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Action: Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
