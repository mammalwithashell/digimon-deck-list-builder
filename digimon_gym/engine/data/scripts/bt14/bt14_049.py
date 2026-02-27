from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_049(CardScript):
    """BT14-049 Lillymon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blast_digivolve
        # Blast Digivolve
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-049 Blast Digivolve")
        effect0.set_effect_description("Blast Digivolve")
        effect0.is_counter_effect = True
        effect0._is_blast_digivolve = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Suspend 1 of your opponent's Digimon. Then, you may return 1 of your opponent's suspended Digimon with 5000 DP or less to the bottom of the deck.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-049 Suspend 1 Digimon and return 1 Digimon to the bottom of Deck")
        effect1.set_effect_description("[On Play] Suspend 1 of your opponent's Digimon. Then, you may return 1 of your opponent's suspended Digimon with 5000 DP or less to the bottom of the deck.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Suspend 1 opponent Digimon (any)
            def on_suspend(target_perm):
                target_perm.suspend()

            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=None, is_optional=False)

            # Then, you may return 1 suspended opponent Digimon with 5000 DP or less to bottom deck
            def bottom_filter(p):
                if not getattr(p, 'is_suspended', False):
                    return False
                if p.dp is None or p.dp > 5000:
                    return False
                return True

            def on_bottom_deck(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.move_permanent_to_bottom_of_deck(target_perm)

            game.effect_select_opponent_permanent(
                player, on_bottom_deck, filter_fn=bottom_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Suspend 1 of your opponent's Digimon. Then, you may return 1 of your opponent's suspended Digimon with 5000 DP or less to the bottom of the deck.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-049 Suspend 1 Digimon and return 1 Digimon to the bottom of Deck")
        effect2.set_effect_description("[When Digivolving] Suspend 1 of your opponent's Digimon. Then, you may return 1 of your opponent's suspended Digimon with 5000 DP or less to the bottom of the deck.")
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Suspend 1 opponent Digimon (any)
            def on_suspend(target_perm):
                target_perm.suspend()

            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=None, is_optional=False)

            # Then, you may return 1 suspended opponent Digimon with 5000 DP or less to bottom deck
            def bottom_filter(p):
                if not getattr(p, 'is_suspended', False):
                    return False
                if p.dp is None or p.dp > 5000:
                    return False
                return True

            def on_bottom_deck(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.move_permanent_to_bottom_of_deck(target_perm)

            game.effect_select_opponent_permanent(
                player, on_bottom_deck, filter_fn=bottom_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
