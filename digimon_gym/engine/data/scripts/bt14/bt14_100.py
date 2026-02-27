from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_100(CardScript):
    """BT14-100 Pummel Whack"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDiscardHand
        # When one of your effects trashes this card in your hand, <Draw 1>.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-100 Draw 1")
        effect0.set_effect_description("When one of your effects trashes this card in your hand, <Draw 1>.")

        def condition0(context: Dict[str, Any]) -> bool:
            # Require this card to be trashed from hand by its owner's effect.
            if context.get('from_zone') != 'hand':
                return False
            if context.get('cause') != 'effect':
                return False
            player = context.get('player')
            source_player = context.get('source_player')
            if player is not None and source_player is not None and player != source_player:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            if player:
                player.draw_cards(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [Main] Delete 1 of your opponent's level 4 or lower Digimon.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-100 Delete")
        effect1.set_effect_description("[Main] Delete 1 of your opponent's level 4 or lower Digimon.")

        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                if p.level is None or p.level > 4:
                    return False
                return p.is_digimon

            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
