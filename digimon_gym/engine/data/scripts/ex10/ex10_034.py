from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_034(CardScript):
    """EX10-034 Blastmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-034 Effect")
        effect0.set_effect_description("Effect")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: collision
        # Collision
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-034 Collision")
        effect1.set_effect_description("Collision")
        effect1._is_collision = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: fragment
        # Fragment
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-034 Fragment")
        effect2.set_effect_description("Fragment")
        effect2._is_fragment = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: blocker
        # Blocker
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-034 Blocker")
        effect3.set_effect_description("Blocker")
        effect3._is_blocker = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect4 = ICardEffect()
        effect4.set_effect_name("EX10-034 1 digimon gains attack on Start of your main phase")
        effect4.set_effect_description("Effect")
        effect4.is_on_play = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Start of Your Main Phase] This Digimon attacks.
        effect5 = ICardEffect()
        effect5.set_effect_name("EX10-034 [Start of Your Main Phase] This Digimon attacks.")
        effect5.set_effect_description("[Start of Your Main Phase] This Digimon attacks.")
        effect5.is_on_play = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
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

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect6 = ICardEffect()
        effect6.set_effect_name("EX10-034 1 digimon gains attack on Start of your main phase")
        effect6.set_effect_description("Effect")
        effect6.is_when_digivolving = True

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect6.set_can_use_condition(condition6)
        effects.append(effect6)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Start of Your Main Phase] This Digimon attacks.
        effect7 = ICardEffect()
        effect7.set_effect_name("EX10-034 [Start of Your Main Phase] This Digimon attacks.")
        effect7.set_effect_description("[Start of Your Main Phase] This Digimon attacks.")
        effect7.is_on_play = True

        effect = effect7  # alias for condition closure
        def condition7(context: Dict[str, Any]) -> bool:
            return True

        effect7.set_can_use_condition(condition7)

        def process7(ctx: Dict[str, Any]):
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

        effect7.set_on_process_callback(process7)
        effects.append(effect7)

        # Timing: EffectTiming.OnAllyAttack
        # [All Turns] [Once Per Turn] When Digimon attack, by trashing any 2 of this Digimon's digivolution cards, this Digimon gains <Security A. +1> and +3000 DP until your turn ends.
        effect8 = ICardEffect()
        effect8.set_effect_name("EX10-034 Trash 2 source cards, gain <Security A. +1> and +3000 DP")
        effect8.set_effect_description("[All Turns] [Once Per Turn] When Digimon attack, by trashing any 2 of this Digimon's digivolution cards, this Digimon gains <Security A. +1> and +3000 DP until your turn ends.")
        effect8.is_optional = True
        effect8.set_max_count_per_turn(1)
        effect8.set_hash_string("AT_EX10_034")
        effect8.is_on_attack = True

        effect = effect8  # alias for condition closure
        def condition8(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and len(permanent.digivolution_cards) >= 2):
                return False
            return True

        effect8.set_can_use_condition(condition8)

        def process8(ctx: Dict[str, Any]):
            """Action: DP +3000, Trash Digivolution Cards"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(3000)
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)

        effect8.set_on_process_callback(process8)
        effects.append(effect8)

        return effects
