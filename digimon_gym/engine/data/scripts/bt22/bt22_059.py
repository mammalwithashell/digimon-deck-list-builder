from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_059(CardScript):
    """BT22-059 Infermon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-059 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.4 for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Delete 1 of your opponent's Digimon with a play cost of 5 or less. Then, if you have [Arata Sanada] or [Eater Adam], your opponent's effects can't reduce this Digimon's DP or return it to hands or decks until their turn ends.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT22-059 Delete a 5 or less play cost digimon, then if you have [Arata Sanada]/[Eater Adam], gain immunity to DP reduction and bounce")
        effect1.set_effect_description("[On Play] Delete 1 of your opponent's Digimon with a play cost of 5 or less. Then, if you have [Arata Sanada] or [Eater Adam], your opponent's effects can't reduce this Digimon's DP or return it to hands or decks until their turn ends.")
        effect1.is_on_play = True
        effect1._is_immune_dp_minus = True
        effect1._is_cannot_return_to_deck = True
        effect1._is_cannot_return_to_hand = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete, Gain Keyword Immune Dp Minus, Gain Keyword Cannot Return To Deck, Gain Keyword Cannot Return To Hand, Grant Bounce Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon and p.top_card and (p.top_card.get_cost_itself or 0) <= 5
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
            has_support = any(
                any('Arata Sanada' in name or 'Eater Adam' in name for name in getattr(p.top_card, 'card_names', []))
                for p in player.battle_area
            )
            if perm and has_support:
                perm.grant_keyword('_is_immune_dp_minus')
                perm.grant_keyword('_is_cannot_return_to_deck')
                perm.grant_keyword('_is_cannot_return_to_hand')
            # Prevent return to hand/deck via modifier system
            if perm and game and has_support:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    perm, ModifierType.CANNOT_RETURN_TO_HAND,
                    value_fn=lambda: True, expiry='end_of_turn')
                game.register_modifier(
                    perm, ModifierType.CANNOT_RETURN_TO_DECK,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Delete 1 of your opponent's Digimon with a play cost of 5 or less. Then, if you have [Arata Sanada] or [Eater Adam], your opponent's effects can't reduce this Digimon's DP or return it to hands or decks until their turn ends.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT22-059 Delete a 5 or less play cost digimon, then if you have [Arata Sanada]/[Eater Adam], gain immunity to DP reduction and bounce")
        effect2.set_effect_description("[When Digivolving] Delete 1 of your opponent's Digimon with a play cost of 5 or less. Then, if you have [Arata Sanada] or [Eater Adam], your opponent's effects can't reduce this Digimon's DP or return it to hands or decks until their turn ends.")
        effect2.is_when_digivolving = True
        effect2._is_immune_dp_minus = True
        effect2._is_cannot_return_to_deck = True
        effect2._is_cannot_return_to_hand = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete, Gain Keyword Immune Dp Minus, Gain Keyword Cannot Return To Deck, Gain Keyword Cannot Return To Hand, Grant Bounce Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon and p.top_card and (p.top_card.get_cost_itself or 0) <= 5
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
            has_support = any(
                any('Arata Sanada' in name or 'Eater Adam' in name for name in getattr(p.top_card, 'card_names', []))
                for p in player.battle_area
            )
            if perm and has_support:
                perm.grant_keyword('_is_immune_dp_minus')
                perm.grant_keyword('_is_cannot_return_to_deck')
                perm.grant_keyword('_is_cannot_return_to_hand')
            # Prevent return to hand/deck via modifier system
            if perm and game and has_support:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    perm, ModifierType.CANNOT_RETURN_TO_HAND,
                    value_fn=lambda: True, expiry='end_of_turn')
                game.register_modifier(
                    perm, ModifierType.CANNOT_RETURN_TO_DECK,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [All Turns] [Once Per Turn] When any of your Digimon with the [Unidentified] trait are deleted, you may play 1 [Diaboromon] Token without paying the cost. (Digimon/Cost 14/Lv.6/White/Mega/Unknown/Unidentified/3000 DP)
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDestroyedAnyone)
        effect3.set_effect_name("BT22-059 Play 1 [Diaboromon] Token")
        effect3.set_effect_description("[All Turns] [Once Per Turn] When any of your Digimon with the [Unidentified] trait are deleted, you may play 1 [Diaboromon] Token without paying the cost. (Digimon/Cost 14/Lv.6/White/Mega/Unknown/Unidentified/3000 DP)")
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("BT22_059_PlayDiaboromonToken")
        effect3.is_on_deletion = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Token"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Play Diaboromon Token — token play not yet supported in engine
            if player and game:
                game.effect_play_token(player, 'diaboromon')

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
