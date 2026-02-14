from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_023(CardScript):
    """BT14-023 Ikkakumon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Trash any 2 digivolution cards from your opponent's Digimon.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-023 Trash digivolution cards and ")
        effect0.set_effect_description("[When Digivolving] Trash any 2 digivolution cards from your opponent's Digimon.")
        effect0.is_when_digivolving = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Trash Digivolution Cards"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking][Once Per Turn] Until the end of your opponent's turn, 1 of your opponent's Digimon with as many or fewer digivolution cards as this Digimon can't attack.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-023 Opponent's 1 Digimon can't attack")
        effect1.set_effect_description("[When Attacking][Once Per Turn] Until the end of your opponent's turn, 1 of your opponent's Digimon with as many or fewer digivolution cards as this Digimon can't attack.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("CantAtatck_BT14_023")
        effect1.is_on_attack = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking][Once Per Turn] Until the end of your opponent's turn, 1 of your opponent's Digimon with as many or fewer digivolution cards as this Digimon can't attack.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-023 Opponent's 1 Digimon can't attack")
        effect2.set_effect_description("[When Attacking][Once Per Turn] Until the end of your opponent's turn, 1 of your opponent's Digimon with as many or fewer digivolution cards as this Digimon can't attack.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("CantAtatck_BT14_023_inherited")
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
