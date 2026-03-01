from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_028(CardScript):
    """EX10-028 Landramon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By trashing any 1 card with the [Mineral] or [Rock] trait from your Digimon's digivolution cards, 1 of your Digimon with the [Mineral] or [Rock] trait gains <Reboot>, <Blocker> and +3000 DP until your opponent's turn ends.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX10-028 By trashing 1 sources, 1 Digimon gains <Reboot>, <Blocker> and +3000 DP")
        effect0.set_effect_description("[On Play] By trashing any 1 card with the [Mineral] or [Rock] trait from your Digimon's digivolution cards, 1 of your Digimon with the [Mineral] or [Rock] trait gains <Reboot>, <Blocker> and +3000 DP until your opponent's turn ends.")
        effect0.is_optional = True
        effect0.is_on_play = True
        effect0._is_reboot = True
        effect0._is_blocker = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: DP +3000, Trash Digivolution Cards, Gain Keyword Reboot, Gain Keyword Blocker"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(3000)
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
            if perm:
                perm.grant_keyword('_is_reboot')
                perm.grant_keyword('_is_blocker')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By trashing any 1 card with the [Mineral] or [Rock] trait from your Digimon's digivolution cards, 1 of your Digimon with the [Mineral] or [Rock] trait gains <Reboot>, <Blocker> and +3000 DP until your opponent's turn ends.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX10-028 By trashing 1 sources, 1 Digimon gains <Reboot>, <Blocker> and +3000 DP")
        effect1.set_effect_description("[When Digivolving] By trashing any 1 card with the [Mineral] or [Rock] trait from your Digimon's digivolution cards, 1 of your Digimon with the [Mineral] or [Rock] trait gains <Reboot>, <Blocker> and +3000 DP until your opponent's turn ends.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True
        effect1._is_reboot = True
        effect1._is_blocker = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP +3000, Trash Digivolution Cards, Gain Keyword Reboot, Gain Keyword Blocker"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(3000)
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
            if perm:
                perm.grant_keyword('_is_reboot')
                perm.grant_keyword('_is_blocker')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDigivolutionCardDiscarded
        # When effects trash this card from a [Mineral] or [Rock] trait Digimon's digivolution cards, delete 1 of your opponent's Digimon with a play cost of 4 or less.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDigivolutionCardDiscarded)
        effect2.set_effect_name("EX10-028 Delete 4 cost or less Digimon")
        effect2.set_effect_description("When effects trash this card from a [Mineral] or [Rock] trait Digimon's digivolution cards, delete 1 of your opponent's Digimon with a play cost of 4 or less.")
        effect2.is_inherited_effect = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
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

        return effects
