from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_083(CardScript):
    """BT16-083 BigUkkomon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Return all Tamers to thier owner's hands. Then, you may play 1 Tamer card from your hand and 1 [Ukkomon] from your trash without paying the cost.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-083 Return all Tamers, then play a Tamer and [Ukkomon] from your trash.")
        effect0.set_effect_description("[On Deletion] Return all Tamers to thier owner's hands. Then, you may play 1 Tamer card from your hand and 1 [Ukkomon] from your trash without paying the cost.")
        effect0.is_on_deletion = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card, Bounce, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_bounce(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.bounce_permanent_to_hand(target_perm)
            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=False)
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] [Once Per Turn] By returning 1 Digi-Egg card from your trash to the bottom of your Digi-Egg deck, delete 1 of your opponent's Digimon with the lowest level. Then you may play 1 level 4 or lower Digimon card from your hand to the breeding area without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT16-083 Return 1 Digi-Egg to delete 1 opponent Digimon with the lowest level and play a level 4 Digimon.")
        effect1.set_effect_description("[End of Your Turn] [Once Per Turn] By returning 1 Digi-Egg card from your trash to the bottom of your Digi-Egg deck, delete 1 of your opponent's Digimon with the lowest level. Then you may play 1 level 4 or lower Digimon card from your hand to the breeding area without paying the cost.")
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Effect_BT16_083")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete, Play Card, Return To Deck"""
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
            def play_filter(c):
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level > 4:
                    return False
                return True
            def on_return(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.return_permanent_to_deck_bottom(target_perm)
            game.effect_select_opponent_permanent(
                player, on_return, filter_fn=target_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
