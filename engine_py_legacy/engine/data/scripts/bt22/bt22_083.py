from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_083(CardScript):
    """BT22-083 Yuuko Kamishiro"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: security_play
        # Security: Play this card
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-083 Security: Play this card")
        effect0.set_effect_description("Security: Play this card")
        effect0.is_security_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name("BT22-083 Memory +1")
        effect1.set_effect_description("[Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAttackTargetChanged
        # [All Turns] When attack targets change, by suspending this Tamer, for the turn, your opponent's Digimon's effects don't affect 1 of your Digimon with [Greymon] in its name or the [CS] trait and it gets +3000 DP.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnAttackTargetChanged)
        effect2.set_effect_name("BT22-083 Suspend tamer, give 1 digimon with [Greymon] in name or [CS] trait effect immunity & 3K DP")
        effect2.set_effect_description("[All Turns] When attack targets change, by suspending this Tamer, for the turn, your opponent's Digimon's effects don't affect 1 of your Digimon with [Greymon] in its name or the [CS] trait and it gets +3000 DP.")
        effect2.is_optional = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Suspend, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not (p.contains_card_name('Greymon')):
                    return False
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)
            # Grant effect immunity via modifier system
            if perm and game:
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAttackTargetChanged
        # [All Turns] [Once Per Turn] When attack targets change, if this Digimon is [Eater Eve], 1 of your Digimon gets +3000 DP for the turn
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnAttackTargetChanged)
        effect3.set_effect_name("BT22-083 give 1 digimon 3K DP")
        effect3.set_effect_description("[All Turns] [Once Per Turn] When attack targets change, if this Digimon is [Eater Eve], 1 of your Digimon gets +3000 DP for the turn")
        effect3.is_inherited_effect = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("BT22_083_Give3KDP")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Eater Eve'))):
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
