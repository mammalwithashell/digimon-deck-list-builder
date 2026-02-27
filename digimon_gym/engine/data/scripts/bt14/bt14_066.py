from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_066(CardScript):
    """BT14-066 PlatinumNumemon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By trashing 1 card with [Numemon] in its name in your hand, gain 2 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-066 Trash 1 card from hand to gain Memory +2")
        effect1.set_effect_description("[On Play] By trashing 1 card with [Numemon] in its name in your hand, gain 2 memory.")
        effect1.is_optional = True
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Trash From Hand, then Gain 2 memory"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            trashed = {'done': False}

            def hand_filter(c):
                return any('Numemon' in _n for _n in getattr(c, 'card_names', []))

            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
                    trashed['done'] = True

            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True)

            if trashed['done']:
                player.add_memory(2)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By trashing 1 card with [Numemon] in its name in your hand, gain 2 memory.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-066 Trash 1 card from hand to gain Memory +2")
        effect2.set_effect_description("[When Digivolving] By trashing 1 card with [Numemon] in its name in your hand, gain 2 memory.")
        effect2.is_optional = True
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Trash From Hand, then Gain 2 memory"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            trashed = {'done': False}

            def hand_filter(c):
                return any('Numemon' in _n for _n in getattr(c, 'card_names', []))

            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
                    trashed['done'] = True

            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True)

            if trashed['done']:
                player.add_memory(2)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] You may play 1 level 5 or lower Digimon card with [Numemon] or [Monzaemon] in its name from your hand without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT14-066 Play 1 Digimon from hand")
        effect3.set_effect_description("[On Deletion] You may play 1 level 5 or lower Digimon card with [Numemon] or [Monzaemon] in its name from your hand without paying the cost.")
        effect3.is_optional = True
        effect3.is_on_deletion = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level > 5:
                    return False
                if not any('Numemon' in _n or 'Monzaemon' in _n for _n in getattr(c, 'card_names', [])):
                    return False
                return True

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
