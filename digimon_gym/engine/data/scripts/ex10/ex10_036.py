from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_036(CardScript):
    """EX10-036 Magneticdramon | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-036 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Close] for cost 6
        effect0._alt_digi_cost = 6
        effect0._alt_digi_name = "Close"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Close') or permanent.contains_card_name('Proganomon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: fragment
        # Fragment
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-036 Fragment")
        effect1.set_effect_description("Fragment")
        effect1._is_fragment = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] [Once Per Turn] By placing 3 [Mineral] or [Rock] trait cards from your trash as this Digimon's bottom digivolution cards, it unsuspends.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-036 Place 3 cards from trash as bottom sources")
        effect2.set_effect_description("[When Digivolving] [Once Per Turn] By placing 3 [Mineral] or [Rock] trait cards from your trash as this Digimon's bottom digivolution cards, it unsuspends.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("WDWA_EX10_036")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] By placing 3 [Mineral] or [Rock] trait cards from your trash as this Digimon's bottom digivolution cards, it unsuspends.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-036 Place 3 cards from trash as bottom sources")
        effect3.set_effect_description("[When Attacking] [Once Per Turn] By placing 3 [Mineral] or [Rock] trait cards from your trash as this Digimon's bottom digivolution cards, it unsuspends.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("WDWA_EX10_036")
        effect3.is_on_attack = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By trashing 3 [Mineral] or [Rock] trait cards from any of your Digimon's digivolution cards, delete 1 of your opponent's Digimon and trash their top security card.
        effect4 = ICardEffect()
        effect4.set_effect_name("EX10-036 By trashing 3 sources, Delete 1 digimon and trash top security")
        effect4.set_effect_description("[When Digivolving] By trashing 3 [Mineral] or [Rock] trait cards from any of your Digimon's digivolution cards, delete 1 of your opponent's Digimon and trash their top security card.")
        effect4.is_optional = True
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Delete, Trash Digivolution Cards, Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=True)
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] By trashing 3 [Mineral] or [Rock] trait cards from any of your Digimon's digivolution cards, delete 1 of your opponent's Digimon and trash their top security card.
        effect5 = ICardEffect()
        effect5.set_effect_name("EX10-036 By trashing 3 sources, Delete 1 digimon and trash top security")
        effect5.set_effect_description("[When Attacking] By trashing 3 [Mineral] or [Rock] trait cards from any of your Digimon's digivolution cards, delete 1 of your opponent's Digimon and trash their top security card.")
        effect5.is_optional = True
        effect5.is_on_attack = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Delete, Trash Digivolution Cards, Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=True)
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
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
