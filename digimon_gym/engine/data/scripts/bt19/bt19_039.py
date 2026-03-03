from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_039(CardScript):
    """BT19-039 SkullBaluchimon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def build_delete_effect(is_on_play: bool = False, is_when_digivolving: bool = False):
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            effect.set_effect_name("BT19-039 Trash your top security, delete a Digimon and get 1 memory")
            effect.set_effect_description(
                "[On Play] By trashing your top security card, delete 1 of your opponent's level 4 or lower Digimon and gain 1 memory."
                if is_on_play else
                "[When Digivolving] By trashing your top security card, delete 1 of your opponent's level 4 or lower Digimon and gain 1 memory."
            )
            effect.is_optional = True
            effect.is_on_play = is_on_play
            effect.is_when_digivolving = is_when_digivolving

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game and player.security_cards):
                    return

                trashed = player.security_cards.pop(0)
                player.trash_cards.append(trashed)
                player.add_memory(1)

                def target_filter(p):
                    return p.is_digimon and p.level is not None and p.level <= 4

                def on_delete(target_perm):
                    enemy = player.enemy if player else None
                    if enemy:
                        enemy.delete_permanent(target_perm)

                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=target_filter, is_optional=True)

            effect.set_on_process_callback(process)
            return effect

        effects.append(build_delete_effect(is_on_play=True))
        effects.append(build_delete_effect(is_when_digivolving=True))

        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDestroyedAnyone)
        effect2.set_effect_name("BT19-039 Recovery +1")
        effect2.set_effect_description("[On Deletion] <Recovery +1>.")
        effect2.is_on_deletion = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.recovery(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnLoseSecurity)
        effect3.set_effect_name("BT19-039 Unsuspend this Digimon")
        effect3.set_effect_description("[All Turns][Once Per Turn] When your security is reduced, you may unsuspend this Digimon.")
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Unsuspend_SkullBaluchimon_BT19_039")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
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

        return effects
