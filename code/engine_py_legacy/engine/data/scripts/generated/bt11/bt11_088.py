from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_088(CardScript):
    """BT11-088 Bagramon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] If your opponent has 1 or fewer Digimon in play, look at your opponent's hand and trash 1 card in it. If your opponent has 2 or more Digimon in play, place 1 of your opponent's Digimon under 1 of your opponent's other Digimon as its bottom digivolution card.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-088 Look at opponent's hand or place 1 Digimon under other Digimon as its bottom digivolution card.")
        effect0.set_effect_description("[On Play] If your opponent has 1 or fewer Digimon in play, look at your opponent's hand and trash 1 card in it. If your opponent has 2 or more Digimon in play, place 1 of your opponent's Digimon under 1 of your opponent's other Digimon as its bottom digivolution card.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def hand_filter(c):
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
        # [When Digivolving] If your opponent has 1 or fewer Digimon in play, look at your opponent's hand and trash 1 card in it. If your opponent has 2 or more Digimon in play, place 1 of your opponent's Digimon under 1 of your opponent's other Digimon as its bottom digivolution card.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT11-088 Look at opponent's hand or place 1 Digimon under other Digimon as its bottom digivolution card.")
        effect1.set_effect_description("[When Digivolving] If your opponent has 1 or fewer Digimon in play, look at your opponent's hand and trash 1 card in it. If your opponent has 2 or more Digimon in play, place 1 of your opponent's Digimon under 1 of your opponent's other Digimon as its bottom digivolution card.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def hand_filter(c):
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [All Turns][Once Per Turn] When an opponent's Digimon digivolves or an effect adds cards to the digivolution cards of an opponent's Digimon, by trashing 1 card in this Digimon's digivolution cards, trash the top card of your opponent's security stack.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT11-088 Trash the top card of opponent's security")
        effect2.set_effect_description("[All Turns][Once Per Turn] When an opponent's Digimon digivolves or an effect adds cards to the digivolution cards of an opponent's Digimon, by trashing 1 card in this Digimon's digivolution cards, trash the top card of your opponent's security stack.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("TrashSecurity_BT11_088")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Trash Digivolution Cards, Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAddDigivolutionCards
        # [All Turns][Once Per Turn] When an opponent's Digimon digivolves or an effect adds cards to the digivolution cards of an opponent's Digimon, by trashing 1 card in this Digimon's digivolution cards, trash the top card of your opponent's security stack.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT11-088 Trash the top card of opponent's security")
        effect3.set_effect_description("[All Turns][Once Per Turn] When an opponent's Digimon digivolves or an effect adds cards to the digivolution cards of an opponent's Digimon, by trashing 1 card in this Digimon's digivolution cards, trash the top card of your opponent's security stack.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("TrashSecurity_BT11_088")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Trash Digivolution Cards, Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
