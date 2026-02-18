from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_099(CardScript):
    """BT13-099 Spencer Damon"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns][Once Per Turn] When one of your yellow Digimon becomes suspended, 1 of your opponent's Digimon gets -1000 DP until the end of your opponent's turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-099 DP -1000")
        effect0.set_effect_description("[All Turns][Once Per Turn] When one of your yellow Digimon becomes suspended, 1 of your opponent's Digimon gets -1000 DP until the end of your opponent's turn.")
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("DP-1000_BT13_099")
        effect0.dp_modifier = -1000

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: DP -1000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-1000)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn][Once Per Turn] If there're 6 or fewer total cards in both players' security stacks, until the end of your opponent's turn, this Tamer is also treated as a 3000 DP Digimon, can't digivolve, and gains <Blocker>.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-099 This Tamer becomes Digimon")
        effect1.set_effect_description("[End of Your Turn][Once Per Turn] If there're 6 or fewer total cards in both players' security stacks, until the end of your opponent's turn, this Tamer is also treated as a 3000 DP Digimon, can't digivolve, and gains <Blocker>.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("BecomeDigimon_BT13_099")
        effect1._is_blocker = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain Keyword Blocker"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_blocker')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-099 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
