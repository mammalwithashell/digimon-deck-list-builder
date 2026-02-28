from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_009(CardScript):
    """BT11-009 Shoutmon + Star Sword | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Also Treated As
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-009 Also treated as [Shoutmon]")
        effect0.set_effect_description("Also Treated As")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Also Treated As"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Also treated as [Name] — name aliasing not modeled in engine
            pass  # descriptive-tagged: also_treated_as_name

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Also Treated As
        effect1 = ICardEffect()
        effect1.set_effect_name("BT11-009 Also treated as [Starmons]")
        effect1.set_effect_description("Also Treated As")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Also Treated As"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Also treated as [Name] — name aliasing not modeled in engine
            pass  # descriptive-tagged: also_treated_as_name

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: save
        # Save
        effect2 = ICardEffect()
        effect2.set_effect_name("BT11-009 Save")
        effect2.set_effect_description("Save")
        effect2._is_save = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: material_save
        # Material Save
        effect3 = ICardEffect()
        effect3.set_effect_name("BT11-009 Material Save")
        effect3.set_effect_description("Material Save")
        effect3._is_material_save = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] 1 of your opponent's Digimon gets -3000 DP for the turn. Then, if DigiXrosing with 2 cards, delete 1 of your opponent's Digimon with 2000 DP or less.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT11-009 DP -3000 and delete 1 Digimon with 2000 DP or less")
        effect4.set_effect_description("[On Play] 1 of your opponent's Digimon gets -3000 DP for the turn. Then, if DigiXrosing with 2 cards, delete 1 of your opponent's Digimon with 2000 DP or less.")
        effect4.is_on_play = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: DP -3000, Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-3000)
            if not (player and game):
                return
            def target_filter(p):
                if p.dp is None or p.dp > 2000:
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.None
        # Effect
        effect5 = ICardEffect()
        effect5.set_effect_name("BT11-009 Effect")
        effect5.set_effect_description("Effect")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            return True

        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
