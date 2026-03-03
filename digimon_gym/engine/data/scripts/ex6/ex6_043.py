from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_043(CardScript):
    """EX6-043 Diaboromon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] You may play 1 [Diaboromon] (Digimon | 14 Cost | Level 6 | White | Mega | Unknown | Unidentified | DP3000) Token without paying the cost.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("EX6-043 Play 1 Diaboromon Token")
        effect0.set_effect_description("[Start of Your Main Phase] You may play 1 [Diaboromon] (Digimon | 14 Cost | Level 6 | White | Mega | Unknown | Unidentified | DP3000) Token without paying the cost.")
        effect0.is_optional = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Token"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Play Diaboromon Token — token play not yet supported in engine
            if player and game:
                game.effect_play_token(player, 'diaboromon')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may play 1 [Diaboromon] (Digimon | 14 Cost | Level 6 | White | Mega | Unknown | Unidentified | DP3000) Token without paying the cost.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX6-043 Play 1 Diaboromon Token")
        effect1.set_effect_description("[When Digivolving] You may play 1 [Diaboromon] (Digimon | 14 Cost | Level 6 | White | Mega | Unknown | Unidentified | DP3000) Token without paying the cost.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Token"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Play Diaboromon Token — token play not yet supported in engine
            if player and game:
                game.effect_play_token(player, 'diaboromon')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [All Turns] [Once Per Turn] When an opponent's Digimon is played, you may activate 1 of this Digimon's [When Digivolving] effects.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX6-043 Activate 1 [When Digivolving] effect")
        effect2.set_effect_description("[All Turns] [Once Per Turn] When an opponent's Digimon is played, you may activate 1 of this Digimon's [When Digivolving] effects.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("PlayToken_EX6_043")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: blocker
        # Blocker
        effect3 = ICardEffect()
        effect3.set_effect_name("EX6-043 Blocker")
        effect3.set_effect_description("Blocker")
        effect3._is_blocker = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Factory effect: jamming
        # Jamming
        effect4 = ICardEffect()
        effect4.set_effect_name("EX6-043 Jamming")
        effect4.set_effect_description("Jamming")
        effect4._is_jamming = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
