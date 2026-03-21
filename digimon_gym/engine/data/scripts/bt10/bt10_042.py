from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_042(CardScript):
    """BT10-042 Venusmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] All of your opponent's Digimon gain <Security Attack -1> until the end of your opponent's turn. (This Digimon checks 1 fewer security cards.)
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT10-042 Security Attack -1")
        effect0.set_effect_description("[When Digivolving] All of your opponent's Digimon gain <Security Attack -1> until the end of your opponent's turn. (This Digimon checks 1 fewer security cards.)")
        effect0.is_when_digivolving = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: All opponent Digimon get Security Attack -1 until end of opponent's turn"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            from ....interfaces.modifiers import ModifierType
            # Apply Security Attack -1 to all opponent Digimon until end of opponent's turn
            for opp_perm in list(enemy.battle_area):
                if opp_perm.is_digimon:
                    game.register_modifier(
                        opp_perm,
                        ModifierType.CHANGE_SECURITY_ATTACK,
                        value_fn=lambda cur, t, c: cur - 1,
                        expiry='end_of_opponent_turn',
                    )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Opponent's Turn] Your opponent's Digimon with <Security A.> can't attack
        # this Digimon and can't activate [When Attacking] and [When Digivolving] effects.
        # This is a continuous/static effect — modeled as a declarative effect.
        # The "can't attack this Digimon" portion is handled by CANNOT_ATTACK_TARGET modifier.
        # The "disable [When Attacking]/[When Digivolving]" is handled by DISABLE_EFFECT modifier.
        # Both are continuous while this card is on the field during opponent's turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT10-042 Opponent's SA Digimon can't attack this / effects disabled")
        effect1.set_effect_description("[Opponent's Turn] Your opponent's Digimon with <Security A.> can't attack this Digimon and can't activate [When Attacking] and [When Digivolving] effects.")
        effect1.is_declarative = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
