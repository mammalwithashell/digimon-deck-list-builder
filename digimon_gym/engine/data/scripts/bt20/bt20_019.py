from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_019(CardScript):
    """BT20-019 Jesmon (X Antibody) | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-019 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Jesmon] for cost 1
        effect0._alt_digi_cost = 1
        effect0._alt_digi_name = "Jesmon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Jesmon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alliance
        # Alliance
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-019 Alliance")
        effect1.set_effect_description("Alliance")
        effect1.is_on_attack = True
        effect1._is_alliance = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If [Jesmon] or [X Antibody] is in this Digimon's digivolution cards, for the turn, 1 of your Digimon isn't affected by your opponent's effects. Then, 1 of your Digimon may attack.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-019 Become unaffected by opponents effects, then can attack")
        effect2.set_effect_description("[When Digivolving] If [Jesmon] or [X Antibody] is in this Digimon's digivolution cards, for the turn, 1 of your Digimon isn't affected by your opponent's effects. Then, 1 of your Digimon may attack.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Force Attack, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Force attack — target Digimon may attack (requires engine SelectAttack)
            pass  # descriptive-tagged: force_attack
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.None
        # Grant Skill
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-019 Your Digimon with [Sistermon] in their names or the [Royal Knight] trait gain Pierce")
        effect3.set_effect_description("Grant Skill")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Grant Skill"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant keyword to other permanents (AddSkillClass) — not yet in engine
            pass  # descriptive-tagged: grant_skill

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.None
        # Attack Unsuspended
        effect4 = ICardEffect()
        effect4.set_effect_name("BT20-019 Attack Unsuspended")
        effect4.set_effect_description("Attack Unsuspended")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Attack Unsuspended"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Can attack unsuspended Digimon via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CAN_ATTACK_UNSUSPENDED, perm,
                    value_fn=lambda: True, expiry='persistent')

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.None
        # Grant Skill, Attack Unsuspended
        effect5 = ICardEffect()
        effect5.set_effect_name("BT20-019 [Your Turn] While this Digimon is [Jesmon GX], all of your Digimon gain <Piercing> and can also attack your opponent's unsuspended Digimon.")
        effect5.set_effect_description("Grant Skill, Attack Unsuspended")
        effect5.is_inherited_effect = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Jesmon GX'))):
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Grant Skill, Attack Unsuspended"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant keyword to other permanents (AddSkillClass) — not yet in engine
            pass  # descriptive-tagged: grant_skill
            # Can attack unsuspended Digimon via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CAN_ATTACK_UNSUSPENDED, perm,
                    value_fn=lambda: True, expiry='persistent')

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
