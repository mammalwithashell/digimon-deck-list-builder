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

        def build_grant_effect(is_on_play: bool = False, is_when_digivolving: bool = False):
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            effect.set_effect_name(
                "EX10-028 By trashing 1 sources, 1 Digimon gains <Reboot>, <Blocker> and +3000 DP"
            )
            effect.set_effect_description(
                "[On Play] By trashing any 1 card with the [Mineral] or [Rock] trait from your Digimon's digivolution cards, 1 of your Digimon with the [Mineral] or [Rock] trait gains <Reboot>, <Blocker> and +3000 DP until your opponent's turn ends."
                if is_on_play else
                "[When Digivolving] By trashing any 1 card with the [Mineral] or [Rock] trait from your Digimon's digivolution cards, 1 of your Digimon with the [Mineral] or [Rock] trait gains <Reboot>, <Blocker> and +3000 DP until your opponent's turn ends."
            )
            effect.is_optional = True
            effect.is_on_play = is_on_play
            effect.is_when_digivolving = is_when_digivolving
            effect._is_reboot = True
            effect._is_blocker = True

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return
                expiry_turn = game.turn_count + 1

                def target_filter(p):
                    return p.is_digimon and (p.has_trait('Mineral') or p.has_trait('Rock'))

                def on_target(target_perm):
                    if not target_perm.has_no_digivolution_cards:
                        trashed = target_perm.trash_digivolution_cards(1)
                        player.trash_cards.extend(trashed)
                    target_perm.change_dp(3000)
                    target_perm.grant_keyword('_is_reboot', expiry_turn)
                    target_perm.grant_keyword('_is_blocker', expiry_turn)

                game.effect_select_own_permanent(
                    player, on_target, filter_fn=target_filter, is_optional=True)

            effect.set_on_process_callback(process)
            return effect

        effects.append(build_grant_effect(is_on_play=True))
        effects.append(build_grant_effect(is_when_digivolving=True))

        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDigivolutionCardDiscarded)
        effect2.set_effect_name("EX10-028 Delete 4 cost or less Digimon")
        effect2.set_effect_description(
            "When effects trash this card from a [Mineral] or [Rock] trait Digimon's digivolution cards, delete 1 of your opponent's Digimon with a play cost of 4 or less."
        )
        effect2.is_inherited_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return p.is_digimon and getattr(p.top_card, 'get_cost_itself', 0) <= 4

            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
