from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_092(CardScript):
    """BT23-092 Ice Archery"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Ignore Color Req
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-092 Ignore color requirements")
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
        # [Main] Until your opponent's turn ends, 1 of their Digimon and 1 of their Tamers can't suspend. Then, place this card in the battle area.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-092 1 digimon and tamer cant suspend until their turn ends")
        effect1.set_effect_description("[Main] Until your opponent's turn ends, 1 of their Digimon and 1 of their Tamers can't suspend. Then, place this card in the battle area.")
        effect1._is_cannot_suspend = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain Keyword Cannot Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_grant(target_perm):
                target_perm.grant_keyword('_is_cannot_suspend')
            game.effect_select_opponent_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: delay
        # Delay
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-092 Delay")
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
        # [Your Turn] When one of your [CS] trait Digimon attacks <Delay>, Until your opponent's turn ends, 1 of their Digimon and 1 of their Tamers can't suspend.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-092 1 digimon and tamer cant suspend until their turn ends")
        effect3.set_effect_description("[Your Turn] When one of your [CS] trait Digimon attacks <Delay>, Until your opponent's turn ends, 1 of their Digimon and 1 of their Tamers can't suspend.")
        effect3.is_optional = True
        effect3.is_on_attack = True
        effect3._is_cannot_suspend = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Gain Keyword Cannot Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_grant(target_perm):
                target_perm.grant_keyword('_is_cannot_suspend')
            game.effect_select_opponent_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Until your opponent's turn ends, 1 of their Digimon and 1 of their Tamers can't suspend. Then, place this card in the battle area.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT23-092 1 digimon and tamer cant suspend until their turn ends")
        effect4.set_effect_description("[Security] Until your opponent's turn ends, 1 of their Digimon and 1 of their Tamers can't suspend. Then, place this card in the battle area.")
        effect4.is_security_effect = True
        effect4.is_security_effect = True
        effect4._is_cannot_suspend = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Gain Keyword Cannot Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_grant(target_perm):
                target_perm.grant_keyword('_is_cannot_suspend')
            game.effect_select_opponent_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
