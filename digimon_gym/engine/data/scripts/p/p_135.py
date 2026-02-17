from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_135(CardScript):
    """P-135 ShoeShoemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Until the end of your opponent's turn, 1 of their Digimon can't attack Digimon and gain <Security A. -1>.
        effect0 = ICardEffect()
        effect0.set_effect_name("P-135 1 Opponent's Digimon can't attack Digimon, and gain <Security A. -1>")
        effect0.set_effect_description("[When Digivolving] Until the end of your opponent's turn, 1 of their Digimon can't attack Digimon and gain <Security A. -1>.")
        effect0.is_when_digivolving = True
        effect0._is_cannot_attack = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain Keyword Cannot Attack, Change Security Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_cannot_attack')
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: jamming
        # Jamming
        effect1 = ICardEffect()
        effect1.set_effect_name("P-135 Jamming")
        effect1.set_effect_description("Jamming")
        effect1._is_jamming = True

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] 1 of your opponent's Digimon gets -2000 DP for the turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("P-135 Opponent's Digimon gets -2000 DP for the turn")
        effect2.set_effect_description("[When Attacking] [Once Per Turn] 1 of your opponent's Digimon gets -2000 DP for the turn.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("ChangeDP_P_135")
        effect2.is_on_attack = True
        effect2.dp_modifier = -2000

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: DP -2000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-2000)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
