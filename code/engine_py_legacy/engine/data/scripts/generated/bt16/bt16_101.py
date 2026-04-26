from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_101(CardScript):
    """BT16-101 Rapidmon (X Antibody) | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: armor_purge
        # Armor Purge
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-101 Armor Purge")
        effect0.set_effect_description("Armor Purge")
        effect0._is_armor_purge = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect1 = ICardEffect()
        effect1.set_effect_name("BT16-101 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 4
        effect1._alt_digi_cost = 4

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Suspend all of your opponent's Digimon. Then, this Digimon may attack.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT16-101 Suspend all opponent's Digimon and attack.")
        effect2.set_effect_description("[When Digivolving] Suspend all of your opponent's Digimon. Then, this Digimon may attack.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Suspend, Force Attack, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)
            # Force attack — target Digimon may attack (requires engine SelectAttack)
            pass  # descriptive-tagged: force_attack
            # Grant effect immunity via modifier system
            if perm and game:
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [All Turns][Once Per Turn] When an opponent's Digimon is deleted in battle or by dropping to 0 DP, gain 2 memory.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT16-101 Memory +2")
        effect3.set_effect_description("[All Turns][Once Per Turn] When an opponent's Digimon is deleted in battle or by dropping to 0 DP, gain 2 memory.")
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Memory+2_BT16_101")
        effect3.is_on_deletion = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Gain 2 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(2)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEndBattle
        # [All Turns][Once Per Turn] When an opponent's Digimon is deleted in battle or by dropping to 0 DP, gain 2 memory.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT16-101 Memory +2")
        effect4.set_effect_description("[All Turns][Once Per Turn] When an opponent's Digimon is deleted in battle or by dropping to 0 DP, gain 2 memory.")
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("Memory+2_BT16_101")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Gain 2 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(2)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Factory effect: dp_modifier_all
        # All your Digimon DP modifier
        effect5 = ICardEffect()
        effect5.set_effect_name("BT16-101 All your Digimon DP modifier")
        effect5.set_effect_description("All your Digimon DP modifier")
        effect5.dp_modifier = -4000
        effect5._applies_to_all_own_digimon = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
