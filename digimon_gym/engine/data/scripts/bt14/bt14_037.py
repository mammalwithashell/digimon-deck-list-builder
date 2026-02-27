from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_037(CardScript):
    """BT14-037 MagnaAngemon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blast_digivolve
        # Blast Digivolve
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-037 Blast Digivolve")
        effect0.set_effect_description("Blast Digivolve")
        effect0.is_counter_effect = True
        effect0._is_blast_digivolve = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] If you have 5 or fewer security cards, <Recovery +1 (Deck)>. Then, for the turn, 1 of your opponent's Digimon gets -1000 DP for each card in your security stack.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-037 Recovery +1 and opponent's 1 Digimon reduces DP")
        effect1.set_effect_description("[On Play] If you have 5 or fewer security cards, <Recovery +1 (Deck)>. Then, for the turn, 1 of your opponent's Digimon gets -1000 DP for each card in your security stack.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = context.get('player')
            if player is None:
                return False
            try:
                return len(player.security) <= 5
            except Exception:
                return False

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player is None:
                return

            # Recovery +1 (Deck)
            player.recovery(1)

            # Then apply DP reduction based on current security count.
            try:
                sec_count = len(player.security)
            except Exception:
                sec_count = 0
            dp_minus = 1000 * sec_count
            if dp_minus <= 0:
                return

            if game is None:
                return

            opponents = game.get_opponent_digimon(player)
            if not opponents:
                return

            target = opponents[0]
            game.apply_dp_modifier_until_end_of_turn(target, -dp_minus, source=card)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If you have 5 or fewer security cards, <Recovery +1 (Deck)>. Then, for the turn, 1 of your opponent's Digimon gets -1000 DP for each card in your security stack.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-037 Recovery +1 and opponent's 1 Digimon reduces DP")
        effect2.set_effect_description("[When Digivolving] If you have 5 or fewer security cards, <Recovery +1 (Deck)>. Then, for the turn, 1 of your opponent's Digimon gets -1000 DP for each card in your security stack.")
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = context.get('player')
            if player is None:
                return False
            try:
                return len(player.security) <= 5
            except Exception:
                return False

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player is None:
                return

            # Recovery +1 (Deck)
            player.recovery(1)

            # Then apply DP reduction based on current security count.
            try:
                sec_count = len(player.security)
            except Exception:
                sec_count = 0
            dp_minus = 1000 * sec_count
            if dp_minus <= 0:
                return

            if game is None:
                return

            opponents = game.get_opponent_digimon(player)
            if not opponents:
                return

            target = opponents[0]
            game.apply_dp_modifier_until_end_of_turn(target, -dp_minus, source=card)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
