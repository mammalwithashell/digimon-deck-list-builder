from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_097(CardScript):
    """BT24-097 Soul Fear"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Ignore Color Req — static effect handled by engine color-req system
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-097 Ignore color requirements")
        effect0.set_effect_description("Ignore Color Req")

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Ignore Color Req"""
            pass  # descriptive-tagged — engine handles color-req bypass
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: security_play
        # Security: Activate main effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-097 Security: Play this card")
        effect1.set_effect_description("Security: Play this card")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OptionSkill
        # [Main] Delete 1 of your opponent's level 6 or higher Digimon.
        # Then, you may link this card to 1 of your Digimon without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OptionSkill)
        effect2.set_effect_name("BT24-097 Delete 1 opponent's level 6 or higher Digimon. Then, you may link this card.")
        effect2.set_effect_description("[Main] Delete 1 of your opponent's level 6 or higher Digimon. Then, you may link this card to 1 of your Digimon on the field without paying the cost.")

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete opponent's Lv.6+ Digimon"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                if not p.is_digimon:
                    return False
                if getattr(p, 'level', None) is None or p.level < 6:
                    return False
                return True

            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # NOTE: The C# has a Link ESS [When Attacking] effect that deletes Lv.5-
        # opponent Digimon. This is a linked effect (SetIsLinkedEffect=true) which
        # requires the Link mechanic. The fabricated When Attacking effect that was
        # here previously has been removed as it was incorrectly modeled as a normal
        # inherited effect. Link ESS effects require engine Link support.

        return effects
