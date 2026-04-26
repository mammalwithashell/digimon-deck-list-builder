from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_044(CardScript):
    """BT23-044 Lilamon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-044 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.4 for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played from the hand, if you have [Yuuko Kamishiro] or a [CS] trait Digimon, reduce the play cost by 3.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name("BT23-044 Play cost reduction -3")
        effect1.set_effect_description("When this card would be played from the hand, if you have [Yuuko Kamishiro] or a [CS] trait Digimon, reduce the play cost by 3.")
        effect1.set_hash_string("BT23_031_ReducePlayCost")
        effect1.cost_reduction = 3

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            # Must have Yuuko Kamishiro (Tamer) or a CS-trait Digimon on field
            owner = card.owner if card else None
            if not owner:
                return False
            has_yuuko_or_cs = False
            for p in owner.battle_area:
                tc = p.top_card
                if not tc:
                    continue
                # Check for Yuuko Kamishiro tamer
                if p.is_tamer and tc.contains_card_name('Yuuko Kamishiro'):
                    has_yuuko_or_cs = True
                    break
                # Check for CS-trait Digimon
                if p.is_digimon and any('CS' in t for t in (tc.card_traits or [])):
                    has_yuuko_or_cs = True
                    break
            return has_yuuko_or_cs

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Cost -3"""
            # Cost reduction by 3 — handled via cost_reduction property on effect1
            pass

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Hidden cost display effect (mirrors C# "Not Shown" ChangeCostClass)
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-044 Play Cost -3")
        effect2.set_effect_description("Cost -3")
        effect2.cost_reduction = 3

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            has_yuuko_or_cs = False
            for p in owner.battle_area:
                tc = p.top_card
                if not tc:
                    continue
                if p.is_tamer and tc.contains_card_name('Yuuko Kamishiro'):
                    has_yuuko_or_cs = True
                    break
                if p.is_digimon and any('CS' in t for t in (tc.card_traits or [])):
                    has_yuuko_or_cs = True
                    break
            return has_yuuko_or_cs

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Cost -3"""
            pass

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By suspending 1 Digimon, until your opponent's turn ends, their effects can't return 1 of your Digimon with [Vegetation], [Plant] or [Fairy] in any of its traits or the [CS] trait to hands or decks.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT23-044 By suspending 1 digimon, 1 digimon cant be bounced to hand & deck")
        effect3.set_effect_description("[On Play] By suspending 1 Digimon, until your opponent's turn ends, their effects can't return 1 of your Digimon with [Vegetation], [Plant] or [Fairy] in any of its traits or the [CS] trait to hands or decks.")
        effect3.is_on_play = True
        effect3._is_cannot_return_to_hand = True
        effect3._is_cannot_return_to_deck = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Suspend own Digimon as cost, then grant bounce immunity to 1 qualifying Digimon"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # "By suspending 1 Digimon" — cost: select own unsuspended Digimon to suspend
            def suspend_filter(p):
                return p.is_digimon and not p.is_suspended
            def on_suspend(target_perm):
                target_perm.suspend()
                # After paying the cost (suspend), grant bounce immunity to 1 qualifying Digimon
                def bounce_filter(p2):
                    if not p2.is_digimon:
                        return False
                    tc = p2.top_card
                    if not tc:
                        return False
                    traits = tc.card_traits or []
                    has_trait = (any('Vegetation' in t for t in traits)
                                or any('Plant' in t for t in traits)
                                or any('Fairy' in t for t in traits)
                                or any('CS' in t for t in traits))
                    return has_trait
                def on_protect(protected_perm):
                    if protected_perm and game:
                        from ....interfaces.modifiers import ModifierType
                        game.register_modifier(
                            protected_perm, ModifierType.CANNOT_BE_RETURNED,
                            value_fn=lambda: True, expiry='end_of_opponent_turn')
                game.effect_select_own_permanent(
                    player, on_protect, filter_fn=bounce_filter, is_optional=False)
            game.effect_select_own_permanent(
                player, on_suspend, filter_fn=suspend_filter, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By suspending 1 Digimon, until your opponent's turn ends, their effects can't return 1 of your Digimon with [Vegetation], [Plant] or [Fairy] in any of its traits or the [CS] trait to hands or decks.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT23-044 By suspending 1 digimon, 1 digimon cant be bounced to hand & deck")
        effect4.set_effect_description("[When Digivolving] By suspending 1 Digimon, until your opponent's turn ends, their effects can't return 1 of your Digimon with [Vegetation], [Plant] or [Fairy] in any of its traits or the [CS] trait to hands or decks.")
        effect4.is_when_digivolving = True
        effect4._is_cannot_return_to_hand = True
        effect4._is_cannot_return_to_deck = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Suspend own Digimon as cost, then grant bounce immunity to 1 qualifying Digimon"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # "By suspending 1 Digimon" — cost: select own unsuspended Digimon to suspend
            def suspend_filter(p):
                return p.is_digimon and not p.is_suspended
            def on_suspend(target_perm):
                target_perm.suspend()
                # After paying the cost (suspend), grant bounce immunity to 1 qualifying Digimon
                def bounce_filter(p2):
                    if not p2.is_digimon:
                        return False
                    tc = p2.top_card
                    if not tc:
                        return False
                    traits = tc.card_traits or []
                    has_trait = (any('Vegetation' in t for t in traits)
                                or any('Plant' in t for t in traits)
                                or any('Fairy' in t for t in traits)
                                or any('CS' in t for t in traits))
                    return has_trait
                def on_protect(protected_perm):
                    if protected_perm and game:
                        from ....interfaces.modifiers import ModifierType
                        game.register_modifier(
                            protected_perm, ModifierType.CANNOT_BE_RETURNED,
                            value_fn=lambda: True, expiry='end_of_opponent_turn')
                game.effect_select_own_permanent(
                    player, on_protect, filter_fn=bounce_filter, is_optional=False)
            game.effect_select_own_permanent(
                player, on_suspend, filter_fn=suspend_filter, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEndBattle
        # [All Turns] [Once Per Turn] When this Digimon deletes your opponent's Digimon in battle, trash their top security card.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEndBattle)
        effect5.set_effect_name("BT23-044 Trash opponent top security")
        effect5.set_effect_description("[All Turns] [Once Per Turn] When this Digimon deletes your opponent's Digimon in battle, trash their top security card.")
        effect5.is_inherited_effect = True
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("BT23_044_AT")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
