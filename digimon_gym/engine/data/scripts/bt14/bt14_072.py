from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_072(CardScript):
    """BT14-072 Fangmon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Return 1 purple Digimon card with the [Dark Animal] trait from your trash to the hand. Then, trash 1 card in your hand.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-072 Return 1 card from trash to hand and trash 1 card from hand")
        effect0.set_effect_description("[On Play] Return 1 purple Digimon card with the [Dark Animal] trait from your trash to the hand. Then, trash 1 card in your hand.")
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

            def trash_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = [col.name for col in getattr(c, 'card_colors', [])]
                if 'Purple' not in colors:
                    return False
                traits = getattr(c, 'type_eng', []) or []
                return 'Dark Animal' in traits

            def on_returned(selected):
                if selected in player.trash_cards:
                    player.trash_cards.remove(selected)
                    player.hand_cards.append(selected)

            game.effect_select_trash_card(
                player, trash_filter, on_returned, is_optional=False
            )

            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)

            game.effect_select_hand_card(
                player, lambda c: True, on_trashed, is_optional=False
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] Return 1 purple Digimon card with the [Dark Animal] trait from your trash to the hand. Then, trash 1 card in your hand.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-072 Return 1 card from trash to hand and trash 1 card from hand")
        effect1.set_effect_description("[When Attacking] Return 1 purple Digimon card with the [Dark Animal] trait from your trash to the hand. Then, trash 1 card in your hand.")
        effect1.is_on_attack = True

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

            def trash_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = [col.name for col in getattr(c, 'card_colors', [])]
                if 'Purple' not in colors:
                    return False
                traits = getattr(c, 'type_eng', []) or []
                return 'Dark Animal' in traits

            def on_returned(selected):
                if selected in player.trash_cards:
                    player.trash_cards.remove(selected)
                    player.hand_cards.append(selected)

            game.effect_select_trash_card(
                player, trash_filter, on_returned, is_optional=False
            )

            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)

            game.effect_select_hand_card(
                player, lambda c: True, on_trashed, is_optional=False
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
