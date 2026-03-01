from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_081(CardScript):
    """BT16-081 MaloMyotismon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By deleting one of your Digimon or Tamers, delete 1 of your opponent's unsuspended Digimon. If no Digimon is deleted by this effect, delete one of your opponent's tamers.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT16-081 Delete your Digimon or Tamer to delete an opponent's unsuspended Digimon. Otherwise delete 1 of your opponent's tamers.")
        effect0.set_effect_description("[When Digivolving] By deleting one of your Digimon or Tamers, delete 1 of your opponent's unsuspended Digimon. If no Digimon is deleted by this effect, delete one of your opponent's tamers.")
        effect0.is_when_digivolving = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
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

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] By deleting one of your Digimon or Tamers, delete 1 of your opponent's unsuspended Digimon. If no Digimon is deleted by this effect, delete one of your opponent's tamers.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnAllyAttack)
        effect1.set_effect_name("BT16-081 Delete your Digimon or Tamer to delete an opponent's unsuspended Digimon. Otherwise delete 1 of your opponent's tamers.")
        effect1.set_effect_description("[When Attacking] By deleting one of your Digimon or Tamers, delete 1 of your opponent's unsuspended Digimon. If no Digimon is deleted by this effect, delete one of your opponent's tamers.")
        effect1.is_on_attack = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
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

        # Timing: EffectTiming.OnDestroyedAnyone
        # [All Turns] [Once Per Turn] When one of your Digimon or Tamers is deleted by an effect, trash the top card of your opponent's security stack.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDestroyedAnyone)
        effect2.set_effect_name("BT16-081 When one of your Digimon or Tamers are deleted by an effect, trash the top card of your opponent's security.")
        effect2.set_effect_description("[All Turns] [Once Per Turn] When one of your Digimon or Tamers is deleted by an effect, trash the top card of your opponent's security stack.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("TrashSecurity_BT16_081")
        effect2.is_on_deletion = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
