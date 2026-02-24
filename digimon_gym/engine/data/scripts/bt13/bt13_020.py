from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_020(CardScript):
    """BT13-020 ShineGreymon: Burst Mode | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-020 Effect")
        effect0.set_effect_description("Effect")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may play 1 [Marcus Damon] from your hand without paying the cost. For the turn, the Tamer played by this effect is also treated as a 12000 DP Digimon, can't digivolve, and gains <Rush>.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-020 Play 1 [Marcus Damon] from hand")
        effect1.set_effect_description("[When Digivolving] You may play 1 [Marcus Damon] from your hand without paying the cost. For the turn, the Tamer played by this effect is also treated as a 12000 DP Digimon, can't digivolve, and gains <Rush>.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True
        effect1._is_rush = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Card, Gain Keyword Rush"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            if perm:
                perm.grant_keyword('_is_rush')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnTappedAnyone
        # [Your Turn][Once Per Turn] When one of your Tamers becomes suspended, trash the top card of your opponent's security stack.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-020 Trash the top card of opponent's security")
        effect2.set_effect_description("[Your Turn][Once Per Turn] When one of your Tamers becomes suspended, trash the top card of your opponent's security stack.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("TrashSecurity_BT13_020")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop()
                        enemy.trash_cards.append(trashed)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
