from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_004(CardScript):
    """EX10-004 Cupimon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnMove
        # [Your Turn] [Once Per Turn] When any of your Digimon with [Lucemon] in their names move from the breeding area to the battle area, by trashing 1 card in your hand, <Draw 1> (Draw 1 card from your deck.) and gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-004 Trash 1 card, Gain 1 Memory, Draw 1")
        effect0.set_effect_description("[Your Turn] [Once Per Turn] When any of your Digimon with [Lucemon] in their names move from the breeding area to the battle area, by trashing 1 card in your hand, <Draw 1> (Draw 1 card from your deck.) and gain 1 memory.")
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("ESS_EX10-004")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1, Gain 1 memory, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def hand_filter(c):
                if not (any('Lucemon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True)
            if player:
                player.draw_cards(1)
            if player:
                player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
