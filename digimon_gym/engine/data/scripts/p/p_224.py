from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_224(CardScript):
    """P-224 Kotone Amano"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # Effect
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("P-224 Effect")
        effect0.set_effect_description("Effect")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("P-224 Effect")
        effect1.set_effect_description("Effect")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDeclaration
        # [Main] By suspending this Tamer, you may play 1 level 5 or higher [Xros Heart] trait Digimon card from under any of your Tamers with the play cost reduced by 1.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2._is_field_main = True
        effect2.set_effect_name("P-224 Suspend to Play a Digimon from tamer")
        effect2.set_effect_description("[Main] By suspending this Tamer, you may play 1 level 5 or higher [Xros Heart] trait Digimon card from under any of your Tamers with the play cost reduced by 1.")
        effect2.is_optional = True
        effect2.cost_reduction = 1

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Cost -1, Suspend, Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level < 5:
                    return False
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level < 5:
                    return False
                if not (any('Xros Heart' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            # Cost reduction by 1 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: security_play
        # Security: Play this card
        effect3 = ICardEffect()
        effect3.set_effect_name("P-224 Security: Play this card")
        effect3.set_effect_description("Security: Play this card")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
