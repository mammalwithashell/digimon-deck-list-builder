from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_043(CardScript):
    """BT8-043 Cherubimon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.BeforePayCost
        # When you would play this card from your hand, you may delete 1 of your purple [Cherubimon] to reduce this card's play cost by 8.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("BT8-043 Delete 1 purple [Cherubimon] and Play Cost -8")
        effect0.set_effect_description("When you would play this card from your hand, you may delete 1 of your purple [Cherubimon] to reduce this card's play cost by 8.")
        effect0.set_hash_string("PlayCost-8_BT8_043")
        effect0.cost_reduction = 8

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Cost -8, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction by 8 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction
            # Grant effect immunity via modifier system
            if perm and game:
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Cost -8, Effect Immunity
        effect1 = ICardEffect()
        effect1.set_effect_name("BT8-043 Play Cost -8")
        effect1.set_effect_description("Cost -8, Effect Immunity")
        effect1.cost_reduction = 8

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Cost -8, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction by 8 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction
            # Grant effect immunity via modifier system
            if perm and game:
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] For each Tamer you have in play, activate the effect below. - 1 of your opponent's Digimon gets <Security Attack -2> until the end of your opponent's next turn. (This Digimon checks 2 fewer security cards.)
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT8-043 Security Attack -2 for each your Tamer")
        effect2.set_effect_description("[On Play] For each Tamer you have in play, activate the effect below. - 1 of your opponent's Digimon gets <Security Attack -2> until the end of your opponent's next turn. (This Digimon checks 2 fewer security cards.)")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Change Security Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] For each Tamer you have in play, activate the effect below. - 1 of your opponent's Digimon gets <Security Attack -2> until the end of your opponent's next turn. (This Digimon checks 2 fewer security cards.)
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT8-043 Security Attack -2 for each your Tamer")
        effect3.set_effect_description("[When Digivolving] For each Tamer you have in play, activate the effect below. - 1 of your opponent's Digimon gets <Security Attack -2> until the end of your opponent's next turn. (This Digimon checks 2 fewer security cards.)")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Change Security Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
