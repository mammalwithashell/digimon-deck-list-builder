from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_042(CardScript):
    """BT20-042 Groundramon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-042 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Coredramon] for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_name = "Coredramon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Coredramon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Suspend 1 of your opponent's Digimon or Tamers. Then, 1 of their Digimon or Tamers can't unsuspend until the end of their turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-042 Suspend 1 Digimon or Tamer and 1 Digimon or Tamer cannot unsuspend")
        effect1.set_effect_description("[On Play] Suspend 1 of your opponent's Digimon or Tamers. Then, 1 of their Digimon or Tamers can't unsuspend until the end of their turn.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
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
                player, on_suspend, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Suspend 1 of your opponent's Digimon or Tamers. Then, 1 of their Digimon or Tamers can't unsuspend until the end of their turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-042 Suspend 1 Digimon or Tamer and 1 Digimon or Tamer cannot unsuspend")
        effect2.set_effect_description("[When Digivolving] Suspend 1 of your opponent's Digimon or Tamers. Then, 1 of their Digimon or Tamers can't unsuspend until the end of their turn.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
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
                player, on_suspend, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.None
        # Effect
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-042 Also treated as level 6 for DNA Digivolution")
        effect3.set_effect_description("Effect")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.None
        # Effect
        effect4 = ICardEffect()
        effect4.set_effect_name("BT20-042 Also treated as [Breakdramon]")
        effect4.set_effect_description("Effect")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEndBattle
        # [All Turns] (Once Per Turn) When this Digimon deletes an opponent's Digimon in battle, trash the top card of your opponent's security stack.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT20-042 Trash the top card of opponent's security")
        effect5.set_effect_description("[All Turns] (Once Per Turn) When this Digimon deletes an opponent's Digimon in battle, trash the top card of your opponent's security stack.")
        effect5.is_inherited_effect = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("TrashSecurity_BT20_042")

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
                        trashed = enemy.security_cards.pop()
                        enemy.trash_cards.append(trashed)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
