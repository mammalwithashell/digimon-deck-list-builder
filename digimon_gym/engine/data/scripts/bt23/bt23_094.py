from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_094(CardScript):
    """BT23-094 Nanomachine Break"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Ignore Color Req
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-094 Ignore color requirements")
        effect0.set_effect_description("Ignore Color Req")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Ignore Color Req"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Ignores color requirement for playing Options — not modeled in engine
            pass  # descriptive-tagged

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [Main] Until your opponent's turn ends, give 1 of their Digimon <Security A. -1> and it can't activate [When Digivolving] or [When Attacking] effects. Then, place this card in the battle area.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-094 1 digimon gains Sec Atk -1 and cant use [When Digivolving] & [When Attacking effects]. then place is battle area")
        effect1.set_effect_description("[Main] Until your opponent's turn ends, give 1 of their Digimon <Security A. -1> and it can't activate [When Digivolving] or [When Attacking] effects. Then, place this card in the battle area.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Change Security Attack, Disable Effect, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack
            # Disable/invalidate effects on target — not yet in engine
            pass  # descriptive-tagged: disable_effect
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: delay
        # Delay
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-094 Delay")
        effect2.set_effect_description("Delay")
        effect2.is_on_attack = True
        effect2._is_delay = True

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # When one of your [CS] trait Digimon attacks <Delay>. Until your opponent's turn ends, give 1 of their Digimon <Security A. -1> and it can't activate [When Digivolving] or [When Attacking] effects.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-094 1 digimon gains Sec Atk -1 and cant use [When Digivolving] & [When Attacking effects].")
        effect3.set_effect_description("When one of your [CS] trait Digimon attacks <Delay>. Until your opponent's turn ends, give 1 of their Digimon <Security A. -1> and it can't activate [When Digivolving] or [When Attacking] effects.")
        effect3.is_optional = True
        effect3.is_on_attack = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Change Security Attack, Disable Effect, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack
            # Disable/invalidate effects on target — not yet in engine
            pass  # descriptive-tagged: disable_effect
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Until your opponent's turn ends, give 1 of their Digimon <Security A. -1> and it can't activate [When Digivolving] or [When Attacking] effects. Then, place this card in the battle area.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT23-094 1 digimon gains Sec Atk -1 and cant use [When Digivolving] & [When Attacking effects]. then place is battle area")
        effect4.set_effect_description("[Security] Until your opponent's turn ends, give 1 of their Digimon <Security A. -1> and it can't activate [When Digivolving] or [When Attacking] effects. Then, place this card in the battle area.")
        effect4.is_security_effect = True
        effect4.is_security_effect = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Change Security Attack, Disable Effect, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack
            # Disable/invalidate effects on target — not yet in engine
            pass  # descriptive-tagged: disable_effect
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
