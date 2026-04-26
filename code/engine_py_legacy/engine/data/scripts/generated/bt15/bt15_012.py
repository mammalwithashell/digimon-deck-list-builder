from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_012(CardScript):
    """BT15-012 Shoutmon X2 | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Also Treated As
        effect0 = ICardEffect()
        effect0.set_effect_name("BT15-012 Also treated as [Shoutmon]")
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
        effect1.set_effect_name("BT15-012 Also treated as [Ballistamon]")
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
        effect2.set_effect_name("BT15-012 Save")
        effect2.set_effect_description("Save")
        effect2._is_save = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: material_save
        # Material Save
        effect3 = ICardEffect()
        effect3.set_effect_name("BT15-012 Material Save")
        effect3.set_effect_description("Material Save")
        effect3._is_material_save = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnStartTurn
        # [Start of Your Turn] By deleting this Digimon, gain 1 memory.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT15-012 Delete this Digimon to gain Memory +1")
        effect4.set_effect_description("[Start of Your Turn] By deleting this Digimon, gain 1 memory.")
        effect4.is_optional = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Suspend 1 of your opponent's Digimon. If DigiXrosing with 2 cards, that Digimon can't unsuspend during your opponent's next unsuspend phase.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT15-012 Suspend 1 Digimon and it can't unsuspend")
        effect5.set_effect_description("[On Play] Suspend 1 of your opponent's Digimon. If DigiXrosing with 2 cards, that Digimon can't unsuspend during your opponent's next unsuspend phase.")
        effect5.is_on_play = True
        effect5._is_cannot_unsuspend = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Suspend, Gain Keyword Cannot Unsuspend"""
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
            if perm:
                perm.grant_keyword('_is_cannot_unsuspend')

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.None
        # Effect
        effect6 = ICardEffect()
        effect6.set_effect_name("BT15-012 Effect")
        effect6.set_effect_description("Effect")

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            return True

        effect6.set_can_use_condition(condition6)
        effects.append(effect6)

        return effects
