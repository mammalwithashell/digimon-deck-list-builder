from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_010(CardScript):
    """EX10-010 BlackWarGreymon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blast_digivolve
        # Blast Digivolve
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-010 Blast Digivolve")
        effect0.set_effect_description("Blast Digivolve")
        effect0.is_counter_effect = True
        effect0._is_blast_digivolve = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: raid
        # Raid
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-010 Raid")
        effect1.set_effect_description("Raid")
        effect1._is_raid = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: blocker
        # Blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-010 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: reboot
        # Reboot
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-010 Reboot")
        effect3.set_effect_description("Reboot")
        effect3._is_reboot = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Delete
        effect4 = ICardEffect()
        effect4.set_effect_name("EX10-010 Delete 1 Digimon/Tamer")
        effect4.set_effect_description("Delete")
        effect4.is_on_play = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Delete
        effect5 = ICardEffect()
        effect5.set_effect_name("EX10-010 Delete 1 Digimon/Tamer")
        effect5.set_effect_description("Delete")
        effect5.is_when_digivolving = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.None
        # Effect Immunity
        effect6 = ICardEffect()
        effect6.set_effect_name("EX10-010 Isn't affected by opponent's Digimon's effects")
        effect6.set_effect_description("Effect Immunity")

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Action: Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant effect immunity (CanNotAffectedClass) — not yet in engine
            pass  # descriptive-tagged: effect_immunity

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
