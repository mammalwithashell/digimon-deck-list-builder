from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_023(CardScript):
    """BT10-023 Thetismon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If you have 6 or fewer cards in your hand, <Draw 2>. (Draw 2 cards from your deck.)
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT10-023 Draw 2")
        effect0.set_effect_description("[When Digivolving] If you have 6 or fewer cards in your hand, <Draw 2>. (Draw 2 cards from your deck.)")
        effect0.is_when_digivolving = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 2"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(2)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking][Once Per Turn] If you have 8 or more cards in your hand, by trashing 2 cards in your hand, unsuspend this Digimon.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUseAttack)
        effect1.set_effect_name("BT10-023 Trash 2 cards from hand to unsuspend this Digimon")
        effect1.set_effect_description("[When Attacking][Once Per Turn] If you have 8 or more cards in your hand, by trashing 2 cards in your hand, unsuspend this Digimon.")
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Unsuspend_BT10_023")
        effect1.is_on_attack = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Trash From Hand, Unsuspend"""
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
                player, hand_filter, on_trashed, is_optional=True)
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
