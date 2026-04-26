from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT4_090(CardScript):
    """BT4-090 Chaosmon | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: piercing
        effect_p = ICardEffect()
        effect_p.set_effect_name("BT4-090 Piercing")
        effect_p.set_effect_description("Piercing")
        effect_p._is_piercing = True

        def condition_p(context: Dict[str, Any]) -> bool:
            return True
        effect_p.set_can_use_condition(condition_p)
        effects.append(effect_p)

        # [When Digivolving] Unsuspend this Digimon. Then, it can attack your
        # opponent's Digimon. This effect allows you to attack unsuspended
        # Digimon as well.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.is_when_digivolving = True
        effect0.set_effect_name("BT4-090 When Digivolving: Unsuspend, attack opponent's Digimon")
        effect0.set_effect_description(
            "[When Digivolving] Unsuspend this Digimon. Then, it can attack "
            "your opponent's Digimon. This effect allows you to attack "
            "unsuspended Digimon as well."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            perm = card.permanent_of_this_card() if card else None
            if not (player and game and perm):
                return
            # Unsuspend this Digimon
            perm.unsuspend()
            # Then, it can attack opponent's Digimon (including unsuspended)
            from ....interfaces.modifiers import ModifierType
            game.register_modifier(
                perm, ModifierType.FORCE_ATTACK,
                value_fn=lambda: True, expiry='end_of_turn')
            game.register_modifier(
                perm, ModifierType.CAN_ATTACK_UNSUSPENDED,
                value_fn=lambda: True, expiry='end_of_turn')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
