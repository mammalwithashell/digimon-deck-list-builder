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

        def process2(ctx: Dict[str, Any]):
            """Activate [When Digivolving] effect (play 1 Diaboromon Token)."""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                game.effect_play_token(player, 'diaboromon')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # [All Turns] All of your other Digimon with [Diaboromon] in their names gain <Jamming> and <Blocker>.
        # Implemented as a start-of-main-phase continuous grant + on-play/when-digivolving initial grant.
        def _grant_keywords_to_diaboromon(player, perm):
            """Grant Blocker and Jamming to all other Diaboromon-named Digimon."""
            for p in player.battle_area:
                if p is perm:
                    continue
                if p.is_digimon and p.contains_card_name('Diaboromon'):
                    p.grant_keyword('_is_blocker')
                    p.grant_keyword('_is_jamming')

        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnStartMainPhase)
        effect3.set_effect_name("EX6-043 Grant Blocker+Jamming to other Diaboromon")
        effect3.set_effect_description("[All Turns] All of your other Digimon with [Diaboromon] in their names gain <Jamming> and <Blocker>.")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Grant Blocker+Jamming to all other Diaboromon-named Digimon."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player and perm:
                _grant_keywords_to_diaboromon(player, perm)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Also grant keywords when EX6-043 enters play or digivolves
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("EX6-043 Grant Blocker+Jamming (on play)")
        effect4.set_effect_description("[All Turns] All of your other Digimon with [Diaboromon] in their names gain <Jamming> and <Blocker>.")
        effect4.is_on_play = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Grant Blocker+Jamming to all other Diaboromon on entry."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player and perm:
                _grant_keywords_to_diaboromon(player, perm)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect5.set_effect_name("EX6-043 Grant Blocker+Jamming (when digivolving)")
        effect5.set_effect_description("[All Turns] All of your other Digimon with [Diaboromon] in their names gain <Jamming> and <Blocker>.")
        effect5.is_when_digivolving = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Grant Blocker+Jamming to all other Diaboromon on digivolve."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player and perm:
                _grant_keywords_to_diaboromon(player, perm)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
