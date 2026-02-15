from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_018(CardScript):
    """BT20-018 Ouryumon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] <De-Digivolve 2> 1 of your opponent's Digimon. Then, if during an attack, 1 of your Digimon in the breeding area may digivolve into a level 6 or lower [Chronicle] trait Digimon card in the hand or trash without paying the cost.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-018 <De-Digivolve 2> 1 of your opponent's Digimon")
        effect0.set_effect_description("[On Play] <De-Digivolve 2> 1 of your opponent's Digimon. Then, if during an attack, 1 of your Digimon in the breeding area may digivolve into a level 6 or lower [Chronicle] trait Digimon card in the hand or trash without paying the cost.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: De Digivolve, Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(2)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)
            if not (player and perm and game):
                return
            def digi_filter(c):
                if getattr(c, 'level', None) is None or c.level > 6:
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] <De-Digivolve 2> 1 of your opponent's Digimon. Then, if during an attack, 1 of your Digimon in the breeding area may digivolve into a level 6 or lower [Chronicle] trait Digimon card in the hand or trash without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-018 <De-Digivolve 2> 1 of your opponent's Digimon")
        effect1.set_effect_description("[When Digivolving] <De-Digivolve 2> 1 of your opponent's Digimon. Then, if during an attack, 1 of your Digimon in the breeding area may digivolve into a level 6 or lower [Chronicle] trait Digimon card in the hand or trash without paying the cost.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: De Digivolve, Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(2)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)
            if not (player and perm and game):
                return
            def digi_filter(c):
                if getattr(c, 'level', None) is None or c.level > 6:
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns] (Once Per Turn) When security stacks are removed from, delete 1 of your opponent's Digimon with the lowest DP.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-018 Delete 1 of your opponent's Digimon")
        effect2.set_effect_description("[All Turns] (Once Per Turn) When security stacks are removed from, delete 1 of your opponent's Digimon with the lowest DP.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("RemovedSec_BT20_018")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
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

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] (Once Per Turn) If this Digimon is [Alphamon: Ouryuken], trash your opponent's top security card.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-018 Trash your opponent's top security card")
        effect3.set_effect_description("[When Attacking] (Once Per Turn) If this Digimon is [Alphamon: Ouryuken], trash your opponent's top security card.")
        effect3.is_inherited_effect = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("WhenAttacking_BT20_018")
        effect3.is_on_attack = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Alphamon: Ouryuken'))):
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
                        trashed = enemy.security_cards.pop()
                        enemy.trash_cards.append(trashed)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
