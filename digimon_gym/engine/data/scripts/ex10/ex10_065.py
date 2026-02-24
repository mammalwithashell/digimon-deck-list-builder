from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_065(CardScript):
    """EX10-065 Yukio Oikawa"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: set_memory_3
        # Set memory to 3
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-065 Set memory to 3")
        effect0.set_effect_description("Set memory to 3")
        # [Start of Your Turn] Set memory to 3 if <= 2

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [All Turns] When any of your Digimon with [Myotismon] in their names are played, by deleting this Tamer, 1 of those Digimon gains <Rush> for the turn. (This Digimon can attack the turn it comes into play.) Then, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-065 Delete this tamer, give one of your played digimon Rush, then gain 1 memory")
        effect1.set_effect_description("[All Turns] When any of your Digimon with [Myotismon] in their names are played, by deleting this Tamer, 1 of those Digimon gains <Rush> for the turn. (This Digimon can attack the turn it comes into play.) Then, gain 1 memory.")
        effect1.is_optional = True
        effect1.is_on_play = True
        effect1._is_rush = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 1 memory, Gain Keyword Rush"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)
            if not (player and game):
                return
            def target_filter(p):
                if not (p.contains_card_name('Myotismon')):
                    return False
                return p.is_digimon
            def on_grant(target_perm):
                target_perm.grant_keyword('_is_rush')
            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-065 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
