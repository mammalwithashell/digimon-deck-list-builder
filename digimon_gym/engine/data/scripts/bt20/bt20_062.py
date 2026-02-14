from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_062(CardScript):
    """BT20-062 Candlemon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: retaliation
        # Retaliation
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-062 Retaliation")
        effect0.set_effect_description("Retaliation")
        effect0._is_retaliation = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] By trashing 1 card in your hand, delete 1 of your opponent's level 4 or lower Digimon.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-062 Trash 1 card from hand to delete 1 of your opponent's level 4 or lower Digimon")
        effect1.set_effect_description("[On Deletion] By trashing 1 card in your hand, delete 1 of your opponent's level 4 or lower Digimon.")
        effect1.is_inherited_effect = True
        effect1.is_optional = True
        effect1.is_on_deletion = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
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
                player, on_delete, filter_fn=target_filter, is_optional=True)
            if not (player and game):
                return
            def hand_filter(c):
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
