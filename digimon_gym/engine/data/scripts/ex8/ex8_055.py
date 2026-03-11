from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_055(CardScript):
    """EX8-055 Pyramidimon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        effect0 = ICardEffect()
        effect0.set_effect_name("EX8-055 Fragment")
        effect0.set_effect_description("Fragment")
        effect0._is_fragment = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        def build_unsuspend_effect(is_when_digivolving: bool = False, is_on_attack: bool = False):
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone if is_when_digivolving else EffectTiming.OnUseAttack)
            effect.set_effect_name("EX8-055 By trashing 3 sources, Unsuspend and gain Security A. +1")
            effect.set_effect_description(
                "[When Digivolving] By trashing any 3 [Mineral] or [Rock] trait cards from any of your Digimon's digivolution cards, this Digimon unsuspends, and it gains <Security A. +1> for the turn."
                if is_when_digivolving else
                "[When Attacking] By trashing any 3 [Mineral] or [Rock] trait cards from any of your Digimon's digivolution cards, this Digimon unsuspends, and it gains <Security A. +1> for the turn."
            )
            effect.is_optional = True
            effect.is_when_digivolving = is_when_digivolving
            effect.is_on_attack = is_on_attack
            effect._security_attack_modifier = 1

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                perm = ctx.get('permanent')
                if not perm:
                    return
                if not perm.has_no_digivolution_cards:
                    trashed = perm.trash_digivolution_cards(3)
                    if player:
                        player.trash_cards.extend(trashed)
                perm.unsuspend()
                perm._temp_sa_modifier += 1

            effect.set_on_process_callback(process)
            return effect

        effects.append(build_unsuspend_effect(is_when_digivolving=True))
        effects.append(build_unsuspend_effect(is_on_attack=True))

        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEndTurn)
        effect3.set_effect_name("EX8-055 Place 3 cards from trash as bottom sources")
        effect3.set_effect_description(
            "[End of Your Turn] [Once Per Turn] You may place up to 3 [Mineral] or [Rock] cards from your trash as this Digimon's bottom digivolution cards."
        )
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("EOT_EX8_055")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
