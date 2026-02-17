from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_115(CardScript):
    """P-115 SkullKnightmon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] You may play 1 tamer card with [Nene Amano]/[Yuu Amano] in its name from your hand or trash without paying the cost. Then, <Save> (You may place this card under one of your Tamers).
        effect0 = ICardEffect()
        effect0.set_effect_name("P-115 Play 1 [Nene Amano] or [Yuu Amano] from hand or trash, and save")
        effect0.set_effect_description("[On Deletion] You may play 1 tamer card with [Nene Amano]/[Yuu Amano] in its name from your hand or trash without paying the cost. Then, <Save> (You may place this card under one of your Tamers).")
        effect0.is_on_deletion = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not (any('Nene Amano' in _n or 'Yuu Amano' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def hand_filter(c):
                if not (any('Nene Amano' in _n or 'Yuu Amano' in _n for _n in getattr(c, 'card_names', []))):
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

        # Factory effect: security_attack_plus
        # Security Attack +1
        effect1 = ICardEffect()
        effect1.set_effect_name("P-115 Security Attack +1")
        effect1.set_effect_description("Security Attack +1")
        effect1.is_inherited_effect = True
        effect1._security_attack_modifier = 1

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
