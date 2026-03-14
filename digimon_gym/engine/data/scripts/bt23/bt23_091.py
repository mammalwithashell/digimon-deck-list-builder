from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_091(CardScript):
    """BT23-091 Wolkenapalm"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Ignore Color Req
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-091 Ignore color requirements")
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
        # [Main] Delete 1 of your opponent's Digimon with the lowest DP. Then, place this card in the battle area.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("BT23-091 Delete 1 digimon with lowest DP, then place in battle area")
        effect1.set_effect_description("[Main] Delete 1 of your opponent's Digimon with the lowest DP. Then, place this card in the battle area.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete opponent's lowest DP Digimon"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            if not opp_digimon:
                return
            min_dp = min((p.dp or 0) for p in opp_digimon)
            def target_filter(p):
                return p.is_digimon and (p.dp or 0) <= min_dp
            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: delay
        # Delay
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-091 Delay")
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

        # Timing: EffectTiming.OnUseAttack
        # [Your Turn] When one of your [CS] trait Digimon attacks <Delay>, delete 1 lowest DP opp Digimon.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnUseAttack)
        effect3.set_effect_name("BT23-091 Delay delete lowest DP")
        effect3.set_effect_description("[Your Turn] When one of your [CS] trait Digimon attacks <Delay>, delete 1 of your opponent's Digimon with the lowest DP.")
        effect3.is_optional = True
        effect3.is_on_attack = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check attacking Digimon has CS trait
            atk_perm = context.get('attacking_permanent') or context.get('permanent')
            if atk_perm:
                traits = getattr(atk_perm.top_card, 'card_traits', []) or []
                if not any('CS' in t for t in traits):
                    return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Delete opponent's lowest DP Digimon (Delay)"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            if not opp_digimon:
                return
            min_dp = min((p.dp or 0) for p in opp_digimon)
            def target_filter(p):
                return p.is_digimon and (p.dp or 0) <= min_dp
            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Delete 1 of your opponent's Digimon with the lowest DP. Then, place this card in the battle area.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.SecuritySkill)
        effect4.set_effect_name("BT23-091 Delete 1 digimon with lowest DP, then place in battle area")
        effect4.set_effect_description("[Security] Delete 1 of your opponent's Digimon with the lowest DP. Then, place this card in the battle area.")
        effect4.is_security_effect = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Delete opponent's lowest DP Digimon (security)"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            if not opp_digimon:
                return
            min_dp = min((p.dp or 0) for p in opp_digimon)
            def target_filter(p):
                return p.is_digimon and (p.dp or 0) <= min_dp
            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
