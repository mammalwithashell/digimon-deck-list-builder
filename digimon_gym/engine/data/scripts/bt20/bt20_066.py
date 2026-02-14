from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_066(CardScript):
    """BT20-066 Stingmon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Delete 1 of your opponent's level 3 Digimon. Then, if it's your turn, 2 of your Digimon may DNA digivolve into a Digimon card with [Imperialdramon] in its name or the [Free] trait in the hand.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-066 Delete digimon and digivolve into ImperialDramon")
        effect0.set_effect_description("[On Play] Delete 1 of your opponent's level 3 Digimon. Then, if it's your turn, 2 of your Digimon may DNA digivolve into a Digimon card with [Imperialdramon] in its name or the [Free] trait in the hand.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Delete, Play Card, Trash From Hand"""
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
            if not (player and game):
                return
            def play_filter(c):
                if not (any('Imperialdramon' in _n for _n in getattr(c, 'card_names', [])) or any('Free' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def hand_filter(c):
                if not (any('Imperialdramon' in _n for _n in getattr(c, 'card_names', [])) or any('Free' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Delete 1 of your opponent's level 3 Digimon. Then, if it's your turn, 2 of your Digimon may DNA digivolve into a Digimon card with [Imperialdramon] in its name or the [Free] trait in the hand.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-066 Delete digimon and digivolve into ImperialDramon")
        effect1.set_effect_description("[When Digivolving] Delete 1 of your opponent's level 3 Digimon. Then, if it's your turn, 2 of your Digimon may DNA digivolve into a Digimon card with [Imperialdramon] in its name or the [Free] trait in the hand.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete, Play Card, Trash From Hand"""
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
            if not (player and game):
                return
            def play_filter(c):
                if not (any('Imperialdramon' in _n for _n in getattr(c, 'card_names', [])) or any('Free' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def hand_filter(c):
                if not (any('Imperialdramon' in _n for _n in getattr(c, 'card_names', [])) or any('Free' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: retaliation
        # Retaliation
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-066 Retaliation")
        effect2.set_effect_description("Retaliation")
        effect2.is_inherited_effect = True
        effect2._is_retaliation = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
