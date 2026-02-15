from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_056(CardScript):
    """BT20-056 Alphamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: barrier
        # Barrier
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-056 Barrier")
        effect0.set_effect_description("Barrier")
        effect0._is_barrier = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] <Recovery +1 (Deck)>. Then, if during an attack, 1 of your Digimon in the breeding area may digivolve into a level 6 or lower [Chronicle] trait Digimon card in the hand or trash without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-056 <Recovery +1 (Deck)>")
        effect1.set_effect_description("[On Play] <Recovery +1 (Deck)>. Then, if during an attack, 1 of your Digimon in the breeding area may digivolve into a level 6 or lower [Chronicle] trait Digimon card in the hand or trash without paying the cost.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Recovery +1, Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.recovery(1)
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

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] <Recovery +1 (Deck)>. Then, if during an attack, 1 of your Digimon in the breeding area may digivolve into a level 6 or lower [Chronicle] trait Digimon card in the hand or trash without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-056 <Recovery +1 (Deck)>")
        effect2.set_effect_description("[When Digivolving] <Recovery +1 (Deck)>. Then, if during an attack, 1 of your Digimon in the breeding area may digivolve into a level 6 or lower [Chronicle] trait Digimon card in the hand or trash without paying the cost.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Recovery +1, Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.recovery(1)
            if not (player and perm and game):
                return
            def digi_filter(c):
                if getattr(c, 'level', None) is None or c.level > 6:
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns] (Once Per Turn) When security stacks are removed from, 1 of your opponent's Digimon gets -8000 DP for the turn.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-056 1 of your opponent's Digimon gets -8000 DP")
        effect3.set_effect_description("[All Turns] (Once Per Turn) When security stacks are removed from, 1 of your opponent's Digimon gets -8000 DP for the turn.")
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("RemovedSec_BT20_056")
        effect3.dp_modifier = -8000

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: DP -8000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-8000)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] (Once Per Turn) When this Digimon would leave the battle are other than by your effects, if this Digimon is [Alphamon: Ouryuken], by trashing your top security card, it doesn't leave.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT20-056 Trash your top security card to prevent this Digimon from leaving the battle area")
        effect4.set_effect_description("[All Turns] (Once Per Turn) When this Digimon would leave the battle are other than by your effects, if this Digimon is [Alphamon: Ouryuken], by trashing your top security card, it doesn't leave.")
        effect4.is_inherited_effect = True
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("TrashSecurityToStay_BT20_056")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Alphamon: Ouryuken'))):
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
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

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
