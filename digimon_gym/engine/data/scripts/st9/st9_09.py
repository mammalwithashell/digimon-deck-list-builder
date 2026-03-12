from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST9_09(CardScript):
    """ST9-09 Stingmon | Lv.4 Digimon"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Your Turn] When you would play this card from your hand, if you have
        # a blue Digimon in play, reduce its play cost by 1.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("ST9-09 play cost -1")
        effect0.set_effect_description(
            "When you would play this card from your hand, if you have a blue Digimon "
            "in play, reduce its play cost by 1."
        )
        effect0.set_hash_string("PlayCost-1_ST9_09")
        effect0.cost_reduction = 1

        def condition0(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False  # LEAK GUARD — only reduce cost for THIS card
            owner = card.owner if card else None
            if not owner:
                return False
            # Card must be in hand (played from hand)
            if card not in owner.hand_cards:
                return False
            # Must have a blue Digimon in play
            has_blue_digimon = any(
                p.is_digimon and any(
                    'Blue' in cl.name
                    for cl in getattr(p.top_card, 'card_colors', [])
                )
                for p in owner.battle_area
            )
            return has_blue_digimon

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [Inherited] [When Attacking] If you have a blue Digimon in play, <Draw 1>.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUseAttack)
        effect1.set_effect_name("ST9-09 Draw 1 (inherited)")
        effect1.set_effect_description(
            "[When Attacking] If you have a blue Digimon in play, <Draw 1>."
        )
        effect1.is_on_attack = True
        effect1.is_inherited_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            if not owner.is_my_turn:
                return False
            # Must have a blue Digimon in play
            has_blue_digimon = any(
                p.is_digimon and any(
                    'Blue' in cl.name
                    for cl in getattr(p.top_card, 'card_colors', [])
                )
                for p in owner.battle_area
            )
            if not has_blue_digimon:
                return False
            # Must have at least 1 card in deck to draw
            if not owner.library_cards:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.draw()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
