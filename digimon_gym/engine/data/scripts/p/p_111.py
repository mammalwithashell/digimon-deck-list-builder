from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_111(CardScript):
    """P-111 Knightmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blocker
        # Blocker
        effect0 = ICardEffect()
        effect0.set_effect_name("P-111 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] 1 of your opponent's Digimon gets -3000 DP for each of your Digimon for the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("P-111 Opponent's 1 Digimon reduces DP")
        effect1.set_effect_description("[On Play] 1 of your opponent's Digimon gets -3000 DP for each of your Digimon for the turn.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] 1 of your opponent's Digimon gets -3000 DP for each of your Digimon for the turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("P-111 Opponent's 1 Digimon reduces DP")
        effect2.set_effect_description("[When Digivolving] 1 of your opponent's Digimon gets -3000 DP for each of your Digimon for the turn.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # [All Turns] [Once Per Turn] When another Digimon attacks, you may play 1 black or yellow level 3 Digimon from your hand without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("P-111 Play 1 Digimon from hand")
        effect3.set_effect_description("[All Turns] [Once Per Turn] When another Digimon attacks, you may play 1 black or yellow level 3 Digimon from your hand without paying the cost.")
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Play1Digimon_P_111")
        effect3.is_on_attack = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not ('Yellow' in [col.name for col in getattr(c, 'card_colors', [])] or 'Black' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
