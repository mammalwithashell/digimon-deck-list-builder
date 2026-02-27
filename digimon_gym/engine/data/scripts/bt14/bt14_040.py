from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_040(CardScript):
    """BT14-040 Jijimon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] You may place 1 Tamer card from your hand on top of your security stack.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-040 Place 1 Tamer card from hand at the top of security")
        effect0.set_effect_description("[On Play] You may place 1 Tamer card from your hand on top of your security stack.")
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def tamer_filter(c):
                return getattr(c, 'card_kind', None) == 1 or getattr(c, 'is_tamer', False)

            game.effect_move_card_from_zone_to_security(
                player, 'hand', tamer_filter, to_top=True, is_optional=True
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [When Digivolving] You may place 1 Tamer card from your hand on top of your security stack.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-040 Place 1 Tamer card from hand at the top of security")
        effect1.set_effect_description("[When Digivolving] You may place 1 Tamer card from your hand on top of your security stack.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True

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

            def tamer_filter(c):
                return getattr(c, 'card_kind', None) == 1 or getattr(c, 'is_tamer', False)

            game.effect_move_card_from_zone_to_security(
                player, 'hand', tamer_filter, to_top=True, is_optional=True
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [All Turns][Once Per Turn] When you play a Tamer, you may play 1 level 3 Digimon from your hand without paying its cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-040 Play 1 level 3 Digimon from hand")
        effect2.set_effect_description("[All Turns][Once Per Turn] When you play a Tamer, you may play 1 level 3 Digimon card from your hand without paying the cost.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Play1Lv3Digimon_WhenPlayTamer_BT14_040")
        effect2.is_on_tamer_play = True

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

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return getattr(c, 'level', None) == 3

            game.effect_play_from_zone(player, 'hand', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
