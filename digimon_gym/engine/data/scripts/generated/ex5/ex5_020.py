from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX5_020(CardScript):
    """EX5-020 Crescemon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX5-020 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Cost -2
        effect1 = ICardEffect()
        effect1.set_effect_name("EX5-020 Cost -2")
        effect1.set_effect_description("Cost -2")
        effect1.cost_reduction = 2

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and len(permanent.digivolution_cards) >= 3):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Cost -2"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction by 2 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: change_digi_cost
        # Change digivolution cost
        effect2 = ICardEffect()
        effect2.set_effect_name("EX5-020 Change digivolution cost")
        effect2.set_effect_description("Change digivolution cost")
        # Reduce digivolution cost by 2 for [Light Fang/LightFang/Night Claw/NightClaw/Galaxy] trait
        effect2.cost_reduction = 2

        def condition2(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and len(permanent.digivolution_cards) >= 3):
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] 1 of your opponent's Digimon can't suspend until the end of your opponent's turn.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX5-020 Opponent's 1 Digimon can't suspend")
        effect3.set_effect_description("[On Play] 1 of your opponent's Digimon can't suspend until the end of your opponent's turn.")
        effect3.is_on_play = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] 1 of your opponent's Digimon can't suspend until the end of your opponent's turn.
        effect4 = ICardEffect()
        effect4.set_effect_name("EX5-020 Opponent's 1 Digimon can't suspend")
        effect4.set_effect_description("[When Digivolving] 1 of your opponent's Digimon can't suspend until the end of your opponent's turn.")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Factory effect: dp_modifier
        # DP modifier
        effect5 = ICardEffect()
        effect5.set_effect_name("EX5-020 DP modifier")
        effect5.set_effect_description("DP modifier")
        effect5.is_inherited_effect = True
        effect5.dp_modifier = 2000

        def condition5(context: Dict[str, Any]) -> bool:
            return True
        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
