from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_052(CardScript):
    """BT19-052 Vespamon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-052 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.4 with [Royal Base] trait for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4
        effect0._alt_digi_trait = "Royal Base"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Royal Base' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Delete 1 of your opponent's Digimon with a play cost of 2 or less. For each of your face up security cards, add 2 to this effect's play cost.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT19-052 Delete 1 Digimon with a play cost of 2 or less")
        effect1.set_effect_description("[On Play] Delete 1 of your opponent's Digimon with a play cost of 2 or less. For each of your face up security cards, add 2 to this effect's play cost.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete"""
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
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Delete 1 of your opponent's Digimon with a play cost of 2 or less. For each of your face up security cards, add 2 to this effect's play cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT19-052 Delete 1 Digimon with a play cost of 2 or less")
        effect2.set_effect_description("[When Digivolving] Delete 1 of your opponent's Digimon with a play cost of 2 or less. For each of your face up security cards, add 2 to this effect's play cost.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete"""
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
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEndBattle
        # [All Turns] [Once Per Turn] When this Digimon deletes an opponent's Digimon in battle, trash the top card of your opponent's security stack.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEndBattle)
        effect3.set_effect_name("BT19-052 Trash the top card of opponent's security")
        effect3.set_effect_description("[All Turns] [Once Per Turn] When this Digimon deletes an opponent's Digimon in battle, trash the top card of your opponent's security stack.")
        effect3.is_inherited_effect = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("TrashSecurity_BT18_052")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
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

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Factory effect: blocker
        # Blocker
        effect4 = ICardEffect()
        effect4.set_effect_name("BT19-052 Blocker")
        effect4.set_effect_description("Blocker")
        effect4._is_blocker = True

        def condition4(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Royal Base' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
