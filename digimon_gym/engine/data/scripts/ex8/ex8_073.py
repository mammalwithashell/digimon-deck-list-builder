from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_073(CardScript):
    """EX8-073 Gallantmon (X Antibody) | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX8-073 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Gallantmon] for cost 1
        effect0._alt_digi_cost = 1
        effect0._alt_digi_name = "Gallantmon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Gallantmon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If [Gallantmon]/[X Antibody] is in this Digimon's digivolution cards, this Digimon gets +4000 DP and 1 of your opponent's Digimon gets -4000 DP until the end of their turn.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX8-073 This Digimon gets +4000 DP and gives -4000 DP to an opponent's Digimon")
        effect1.set_effect_description("[When Digivolving] If [Gallantmon]/[X Antibody] is in this Digimon's digivolution cards, this Digimon gets +4000 DP and 1 of your opponent's Digimon gets -4000 DP until the end of their turn.")
        effect1.is_when_digivolving = True
        effect1.dp_modifier = 4000

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP +4000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(4000)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] If [Gallantmon]/[X Antibody] is in this Digimon's digivolution cards, this Digimon gets +4000 DP and 1 of your opponent's Digimon gets -4000 DP until the end of their turn.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("EX8-073 This Digimon gets +4000 DP and gives -4000 DP to an opponent's Digimon")
        effect2.set_effect_description("[When Attacking] If [Gallantmon]/[X Antibody] is in this Digimon's digivolution cards, this Digimon gets +4000 DP and 1 of your opponent's Digimon gets -4000 DP until the end of their turn.")
        effect2.is_on_attack = True
        effect2.dp_modifier = 4000

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: DP +4000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(4000)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] [Once Per Turn] Delete 1 of your opponent's Digimon with 10000 DP or less. If this didn't delete, trash your opponent's top security card and this Digimon unsuspends.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("EX8-073 Delete 1 Digimon with 10000 DP or less, or trash the opponent's top security and unsuspend")
        effect3.set_effect_description("[When Digivolving] [Once Per Turn] Delete 1 of your opponent's Digimon with 10000 DP or less. If this didn't delete, trash your opponent's top security card and this Digimon unsuspends.")
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Delete_EX8_073")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Destroy Security, Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEndAttack
        # [End of Attack] [Once Per Turn] Delete 1 of your opponent's Digimon with 10000 DP or less. If this didn't delete, trash your opponent's top security card and this Digimon unsuspends.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEndAttack)
        effect4.set_effect_name("EX8-073 Delete 1 Digimon with 10000 DP or less, or trash the opponent's top security and unsuspend")
        effect4.set_effect_description("[End of Attack] [Once Per Turn] Delete 1 of your opponent's Digimon with 10000 DP or less. If this didn't delete, trash your opponent's top security card and this Digimon unsuspends.")
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("Delete_EX8_073")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Destroy Security, Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.None
        # Effect Immunity
        effect5 = ICardEffect()
        effect5.set_effect_name("EX8-073 Not affected by opponent's Digimon's effects if you have 0 or less memory")
        effect5.set_effect_description("Effect Immunity")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
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

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
