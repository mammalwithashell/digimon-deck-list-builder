from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_029(CardScript):
    """BT21-029 Medusamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: progress
        # Progress
        effect0 = ICardEffect()
        effect0.set_effect_name("BT21-029 Progress")
        effect0.set_effect_description("Progress")
        effect0._is_progress = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: security_attack_plus
        # Security Attack +1
        effect1 = ICardEffect()
        effect1.set_effect_name("BT21-029 Security Attack +1")
        effect1.set_effect_description("Security Attack +1")
        effect1._security_attack_modifier = 1

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT21-029 Delete lowest DP Digimon")
        effect2.set_effect_description("Effect")
        effect2.set_hash_string("Delete_BT21_029")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEndAttack
        # Effect
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEndAttack)
        effect3.set_effect_name("BT21-029 Delete lowest DP Digimon")
        effect3.set_effect_description("Effect")
        effect3.set_hash_string("Delete_BT21_029")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [All Turns] [Once Per Turn] When any of your opponent's Digimon are deleted, they play 1 [Petrification] Token
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnDestroyedAnyone)
        effect4.set_effect_name("BT21-029 Play 1 [Petrification Token]")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When any of your opponent's Digimon are deleted, they play 1 [Petrification] Token")
        effect4.set_hash_string("PlayToken_BT21_029")
        effect4.is_on_deletion = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Play Petrification Token on opponent's field"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                enemy = player.enemy if player else None
                if enemy:
                    game.effect_play_token(enemy, 'petrification')

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns] [Once Per Turn] When opponents security stack is removed from, they play 1 [Petrification] Token
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnLoseSecurity)
        effect5.set_effect_name("BT21-029 Play 1 [Petrification Token]")
        effect5.set_effect_description("[All Turns] [Once Per Turn] When opponents security stack is removed from, they play 1 [Petrification] Token")
        effect5.set_hash_string("PlayToken_BT21_029")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Play Petrification Token on opponent's field"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                enemy = player.enemy if player else None
                if enemy:
                    game.effect_play_token(enemy, 'petrification')

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
