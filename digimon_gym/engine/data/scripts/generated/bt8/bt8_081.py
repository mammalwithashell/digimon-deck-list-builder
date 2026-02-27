from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_081(CardScript):
    """BT8-081 Rasenmon Fury Mode | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEndAttack
        # [End of Attack] You may digivolve this Digimon into a [Rasenmon] in your hand, ignoring its digivolution requirements and without paying its digivolution cost.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT8-081 This Digimon digivolves")
        effect0.set_effect_description("[End of Attack] You may digivolve this Digimon into a [Rasenmon] in your hand, ignoring its digivolution requirements and without paying its digivolution cost.")
        effect0.is_optional = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] Trash the top card of your security stack.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT8-081 Trash the top card of your security")
        effect1.set_effect_description("[End of Your Turn] Trash the top card of your security stack.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDigivolutionCardDiscarded
        # [Your Turn] When this card is trashed as a digivolution card by [Rasenmon]'s effect, unsuspend 1 of your Digimon and it gets +3000 DP for the turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT8-081 Unsuspend 1 Digimon and DP +3000")
        effect2.set_effect_description("[Your Turn] When this card is trashed as a digivolution card by [Rasenmon]'s effect, unsuspend 1 of your Digimon and it gets +3000 DP for the turn.")
        effect2.is_inherited_effect = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: DP +3000, Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(3000)
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
