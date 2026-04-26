from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_014(CardScript):
    """BT11-014 GrapLeomon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: raid
        # Raid
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-014 Raid")
        effect0.set_effect_description("Raid")
        effect0.is_on_attack = True
        effect0._is_raid = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAttackTargetChanged
        # [Your Turn][Once Per Turn] When this Digimon's attack target is switched, trash the top card of your opponent's security stack.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnAttackTargetChanged)
        effect1.set_effect_name("BT11-014 Trash the top card of opponent's security")
        effect1.set_effect_description("[Your Turn][Once Per Turn] When this Digimon's attack target is switched, trash the top card of your opponent's security stack.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("TrashSecurity_BT11_014")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
