from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_067(CardScript):
    """BT20-067 Soulmon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] 1 of your Digimon gains <Retaliation> (When this Digimon is deleted after losing a battle, delete the Digimon it was battling), until the end of your opponent's turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-067 Your 1 Digimon gains <Retaliation>")
        effect0.set_effect_description("[On Play] 1 of your Digimon gains <Retaliation> (When this Digimon is deleted after losing a battle, delete the Digimon it was battling), until the end of your opponent's turn.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] 1 of your Digimon gains <Retaliation> (When this Digimon is deleted after losing a battle, delete the Digimon it was battling), until the end of your opponent's turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-067 Your 1 Digimon gains <Retaliation>")
        effect1.set_effect_description("[When Digivolving] 1 of your Digimon gains <Retaliation> (When this Digimon is deleted after losing a battle, delete the Digimon it was battling), until the end of your opponent's turn.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] By trashing 1 card in your hand, delete 1 of your opponent's level 4 or lower Digimon.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-067 Trash 1 card from hand to delete 1 of your opponent's level 4 or lower Digimon")
        effect2.set_effect_description("[On Deletion] By trashing 1 card in your hand, delete 1 of your opponent's level 4 or lower Digimon.")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.is_on_deletion = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
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

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
