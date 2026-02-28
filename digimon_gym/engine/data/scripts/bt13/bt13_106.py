from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_106(CardScript):
    """BT13-106 Odin's Breath"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDiscardSecurity
        # When an effect trashes this card from the security stack, activate this card's [Main] effect.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-106 Activate [Main] effect")
        effect0.set_effect_description("When an effect trashes this card from the security stack, activate this card's [Main] effect.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [Main] 1 of your opponent's Digimon gets -3000 DP until the end of your opponent's turn. Then, if there're 6 or fewer total cards in both players' security stacks, all of your opponent's Digimon gain <Security Attack -1> until the end of your opponent's turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-106 DP -3000")
        effect1.set_effect_description("[Main] 1 of your opponent's Digimon gets -3000 DP until the end of your opponent's turn. Then, if there're 6 or fewer total cards in both players' security stacks, all of your opponent's Digimon gain <Security Attack -1> until the end of your opponent's turn.")
        effect1.dp_modifier = -3000

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP -3000"""
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

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-106 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
