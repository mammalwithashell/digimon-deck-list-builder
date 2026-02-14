from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_058(CardScript):
    """BT20-058 Raidenmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Delete 1 of your opponent's Digimon with a play cost of 7 or less.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-058 Delete 1 Digimon")
        effect0.set_effect_description("[On Play] Delete 1 of your opponent's Digimon with a play cost of 7 or less.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
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

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Delete 1 of your opponent's Digimon with a play cost of 7 or less.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-058 Delete 1 Digimon")
        effect1.set_effect_description("[When Digivolving] Delete 1 of your opponent's Digimon with a play cost of 7 or less.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
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

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When this Digimon would leave the battle area, you may play 1 play cost 11 or lower Digimon card with the [Machine] or [Cyborg] trait from this Digimon's digivolution cards without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-058 Play 11 cost or lower Digimon from sources")
        effect2.set_effect_description("[All Turns] When this Digimon would leave the battle area, you may play 1 play cost 11 or lower Digimon card with the [Machine] or [Cyborg] trait from this Digimon's digivolution cards without paying the cost.")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.None
        # Effect
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-058 Effect")
        effect3.set_effect_description("Effect")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
