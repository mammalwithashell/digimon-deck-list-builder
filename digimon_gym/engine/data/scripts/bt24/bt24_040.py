from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_040(CardScript):
    """BT24-040 Venusmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: Lv.5 with [TS] trait: Cost 3
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-040 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        effect0._alt_digi_trait = "TS"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played, if you have 3 or fewer security cards, reduce the play cost by 5.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name("BT24-040 Reduce play cost (5)")
        effect1.set_effect_description("When this card would be played, if you have 3 or fewer security cards, reduce the play cost by 5.")
        effect1.cost_reduction = 5

        def condition1(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            owner = getattr(card, 'owner', None)
            if not owner:
                return False
            return len(owner.security_cards) <= 3

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Shared logic for On Play / When Digivolving:
        # Trash ALL digivolution cards of 1 of your opponent's Digimon.
        # Then, until the end of your opponent's turn, all of your Digimon with the [TS] trait
        # can't have their DP reduced.
        def _venusmon_on_play_when_digivolving(player, game):
            if not (player and game):
                return
            # Select 1 opponent Digimon to trash ALL its digivolution cards
            def opp_filter(p):
                return p.is_digimon

            def on_select_opp(target_perm):
                if not target_perm:
                    return
                # Trash ALL digivolution cards (all sources except top card)
                digi_count = max(0, len(target_perm.card_sources) - 1)
                if digi_count > 0:
                    trashed = target_perm.trash_digivolution_cards(digi_count)
                    if player.enemy:
                        player.enemy.trash_cards.extend(trashed)

            game.effect_select_opponent_permanent(
                player, on_select_opp,
                filter_fn=opp_filter,
                is_optional=False
            )

            # Grant IMMUNE_FROM_DP_MINUS to all own [TS]-trait Digimon until end of opponent's turn
            from digimon_gym.engine.interfaces.modifiers import ModifierType
            for perm in list(player.battle_area):
                if not perm.is_digimon:
                    continue
                traits = getattr(perm.top_card, 'card_traits', []) or []
                if 'TS' in traits:
                    game.register_modifier(
                        ModifierType.IMMUNE_FROM_DP_MINUS, perm,
                        value_fn=lambda: True, expiry='end_of_opponent_turn'
                    )

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Trash all digivolution cards of 1 of your opponent's Digimon.
        # Then, until the end of your opponent's turn, all of your Digimon with the [TS] trait can't have their DP reduced.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT24-040 Trash digi cards, TS digimon immune to DP reduction")
        effect3.set_effect_description("[On Play] Trash all digivolution cards of 1 of your opponent's Digimon. Then, until the end of your opponent's turn, all of your Digimon with the [TS] trait can't have their DP reduced.")
        effect3.is_on_play = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            _venusmon_on_play_when_digivolving(player, game)

        effect3.set_on_process_callback(process3)
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Trash all digivolution cards of 1 of your opponent's Digimon.
        # Then, until the end of your opponent's turn, all of your Digimon with the [TS] trait can't have their DP reduced.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT24-040 Trash digi cards, TS digimon immune to DP reduction")
        effect4.set_effect_description("[When Digivolving] Trash all digivolution cards of 1 of your opponent's Digimon. Then, until the end of your opponent's turn, all of your Digimon with the [TS] trait can't have their DP reduced.")
        effect4.is_when_digivolving = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            _venusmon_on_play_when_digivolving(player, game)

        effect4.set_on_process_callback(process4)
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] [Once Per Turn] When this Digimon would leave the battle area by your opponent's effects,
        # it doesn't leave.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.WhenRemoveField)
        effect5.set_effect_name("BT24-040 This digimon won't leave by opponent's effects (OPT)")
        effect5.set_effect_description("[All Turns] [Once Per Turn] When this Digimon would leave the battle area by your opponent's effects, it doesn't leave.")
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("BT24_040_AT_self")

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = getattr(card, 'owner', None)
            if not owner:
                return False
            # Must be this Digimon leaving
            event_perm = context.get('event_permanent') or context.get('permanent')
            owner_perm = card.permanent_of_this_card()
            if event_perm is None or event_perm is not owner_perm:
                return False
            # Must be by opponent's effect
            if not context.get('is_opponent_effect', False):
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Cancel this Digimon leaving by opponent's effect."""
            owner_perm = card.permanent_of_this_card()
            game = ctx.get('game')
            if owner_perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_REMOVED, owner_perm,
                    value_fn=lambda: True, expiry='end_of_turn'
                )

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [All Turns] [Once Per Turn] When another Digimon is deleted, if you have a blue Tamer, gain 1 memory.
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.OnDestroyedAnyone)
        effect6.set_effect_name("BT24-040 When another Digimon deleted, if blue Tamer, gain 1 memory")
        effect6.set_effect_description("[All Turns] [Once Per Turn] When another Digimon is deleted, if you have a blue Tamer, gain 1 memory.")
        effect6.set_max_count_per_turn(1)
        effect6.set_hash_string("BT24_040_AT_mem")

        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = getattr(card, 'owner', None)
            if not owner:
                return False
            # The deleted permanent must not be this card's permanent
            deleted = context.get('deleted_permanent')
            owner_perm = card.permanent_of_this_card()
            if deleted is not None and owner_perm is not None and deleted is owner_perm:
                return False
            # Must be a Digimon that was deleted
            if deleted is not None and not deleted.is_digimon:
                return False
            # Must have a blue Tamer on own field
            from digimon_gym.engine.data.enums import CardColor
            has_blue_tamer = any(
                p.is_tamer and CardColor.Blue in getattr(p.top_card, 'card_colors', [])
                for p in owner.battle_area
            )
            return has_blue_tamer

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Action: Gain 1 memory."""
            player = ctx.get('player')
            if player:
                player.add_memory(1)

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
