from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_013(CardScript):
    """BT20-013 BaoHuckmon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDeclaration
        # [Main] [Once Per Turn] You may play 1 Digimon card with [Sistermon] or [Gankoomon] in its name from you hand with the play cost reduced by 2.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-013 Play a Digimon with the cost reduced by 2")
        effect0.set_effect_description("[Main] [Once Per Turn] You may play 1 Digimon card with [Sistermon] or [Gankoomon] in its name from you hand with the play cost reduced by 2.")
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("PlayCost_BT20_013")
        effect0.cost_reduction = 2

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Cost -2, Play Card, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not (any('Sistermon' in _n or 'Gankoomon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def hand_filter(c):
                if not (any('Sistermon' in _n or 'Gankoomon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)
            # Cost reduction handled via cost_reduction property

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: dp_modifier_all
        # All your Digimon DP modifier
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-013 All your Digimon DP modifier")
        effect1.set_effect_description("All your Digimon DP modifier")
        effect1.is_inherited_effect = True
        effect1.dp_modifier = 1000
        effect1._applies_to_all_own_digimon = True

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
