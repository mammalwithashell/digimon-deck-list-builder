from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_041(CardScript):
    """EX6-041 Infermon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By deleting 1 of your Digimon with [Diaboromon] in its name, this Digimon may digivolve into [Diaboromon] in your hand without paying the cost.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX6-041 Delete 1 [Diaboromon], Digivolve into a [Diaboromon]")
        effect0.set_effect_description("[On Play] By deleting 1 of your Digimon with [Diaboromon] in its name, this Digimon may digivolve into [Diaboromon] in your hand without paying the cost.")
        effect0.is_optional = True
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Delete own Diaboromon, then Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def delete_filter(p):
                if p is perm:
                    return False
                if not p.is_digimon:
                    return False
                return p.contains_card_name('Diaboromon')

            # Must have a valid deletion target to pay the cost
            if not any(delete_filter(p) for p in player.battle_area):
                return

            def on_delete(target_perm):
                player.delete_permanent(target_perm)
                # After paying the cost, digivolve into Diaboromon from hand
                def digi_filter(c):
                    if not (any('Diaboromon' in _n for _n in getattr(c, 'card_names', []))):
                        return False
                    return True
                game.effect_digivolve_from_hand(
                    player, perm, digi_filter, is_optional=True)

            game.effect_select_own_permanent(
                player, on_delete, filter_fn=delete_filter, is_optional=False,
                prompt="Select a Digimon with [Diaboromon] to delete.")

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By deleting 1 of your Digimon with [Diaboromon] in its name, this Digimon may digivolve into [Diaboromon] in your hand without paying the cost.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX6-041 Delete 1 [Diaboromon], Digivolve into a [Diaboromon]")
        effect1.set_effect_description("[When Digivolving] By deleting 1 of your Digimon with [Diaboromon] in its name, this Digimon may digivolve into [Diaboromon] in your hand without paying the cost.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete own Diaboromon, then Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def delete_filter(p):
                if p is perm:
                    return False
                if not p.is_digimon:
                    return False
                return p.contains_card_name('Diaboromon')

            # Must have a valid deletion target to pay the cost
            if not any(delete_filter(p) for p in player.battle_area):
                return

            def on_delete(target_perm):
                player.delete_permanent(target_perm)
                # After paying the cost, digivolve into Diaboromon from hand
                def digi_filter(c):
                    if not (any('Diaboromon' in _n for _n in getattr(c, 'card_names', []))):
                        return False
                    return True
                game.effect_digivolve_from_hand(
                    player, perm, digi_filter, is_optional=True)

            game.effect_select_own_permanent(
                player, on_delete, filter_fn=delete_filter, is_optional=False,
                prompt="Select a Digimon with [Diaboromon] to delete.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [All Turns] [Once Per Turn] When one of your other Digimon with [Diaboromon] in its name is played, <De-Digivolve 1> 1 of your opponent's Digimon.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX6-041 <De-Digivolve 1> 1 of your opponent's Digimon")
        effect2.set_effect_description("[All Turns] [Once Per Turn] When one of your other Digimon with [Diaboromon] in its name is played, <De-Digivolve 1> 1 of your opponent's Digimon.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("DeDigivolve_EX6_041")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
