from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_060(CardScript):
    """EX6-060 Belphemon: Rage Mode | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-060 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 1
        effect0._alt_digi_cost = 1

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may trash up to 3 cards in your hand. For each card trashed by this effect, suspend 1 of your opponent's level 5 or lower Digimon. Then, delete all of your opponent's suspended Digimon with the lowest play cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX6-060 Trash up to 3 cards, suspend 1 opponent's digimon. Then delete all opponents suspended digimon with lowest play cost.")
        effect1.set_effect_description("[On Play] You may trash up to 3 cards in your hand. For each card trashed by this effect, suspend 1 of your opponent's level 5 or lower Digimon. Then, delete all of your opponent's suspended Digimon with the lowest play cost.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete, Trash From Hand, Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level > 5:
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
            if not (player and game):
                return
            def hand_filter(c):
                if getattr(c, 'level', None) is None or c.level > 5:
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level > 5:
                    return False
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may trash up to 3 cards in your hand. For each card trashed by this effect, suspend 1 of your opponent's level 5 or lower Digimon. Then, delete all of your opponent's suspended Digimon with the lowest play cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX6-060 Trash up to 3 cards, suspend 1 opponent's digimon. Then delete all opponents suspended digimon with lowest play cost.")
        effect2.set_effect_description("[When Digivolving] You may trash up to 3 cards in your hand. For each card trashed by this effect, suspend 1 of your opponent's level 5 or lower Digimon. Then, delete all of your opponent's suspended Digimon with the lowest play cost.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete, Trash From Hand, Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level > 5:
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
            if not (player and game):
                return
            def hand_filter(c):
                if getattr(c, 'level', None) is None or c.level > 5:
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level > 5:
                    return False
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When this Digimon would leave the battle area other than in battle, place 1 card with the [Seven Great Demon Lord] trait from your trash as the bottom digivolution cards of one of your [Gate of Deadly Sins] in your breeding area.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX6-060 Place 1 7GDL from trash to bottom of [Gate of Deadly Sins]")
        effect3.set_effect_description("[All Turns] When this Digimon would leave the battle area other than in battle, place 1 card with the [Seven Great Demon Lord] trait from your trash as the bottom digivolution cards of one of your [Gate of Deadly Sins] in your breeding area.")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
