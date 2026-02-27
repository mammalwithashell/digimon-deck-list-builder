from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_039(CardScript):
    """BT14-039 Monzaemon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-039 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: armor_purge
        # Armor Purge
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-039 Armor Purge")
        effect1.set_effect_description("Armor Purge")
        effect1._is_armor_purge = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By placing 1 Digimon card with [Numemon] in its name from your trash as this Digimon's bottom digivolution card, gain 2 memory.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-039 Place 1 card from trash to digivolution cards to gain Memory +2")
        effect2.set_effect_description("[On Play] By placing 1 Digimon card with [Numemon] in its name from your trash as this Digimon's bottom digivolution card, gain 2 memory.")
        effect2.is_optional = True
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain 2 memory"""
            player = ctx.get('player')
            if player:
                player.add_memory(2)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: security_attack_plus
        # Security Attack +1
        effect3 = ICardEffect()
        effect3.set_effect_name("BT14-039 Security Attack +1")
        effect3.set_effect_description("[Your Turn] While this Digimon has [Monzaemon] or [Numemon] in its name, it gains Security Attack +1")
        effect3.is_inherited_effect = True
        effect3._security_attack_modifier = 1

        def condition3(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False

            # Active only while this Digimon has [Monzaemon] or [Numemon] in its name
            stack_cards = []
            if hasattr(perm, 'digivolution_cards') and perm.digivolution_cards is not None:
                stack_cards = perm.digivolution_cards
            elif hasattr(perm, 'evolution_cards') and perm.evolution_cards is not None:
                stack_cards = perm.evolution_cards

            for c in stack_cards:
                name = getattr(c, 'card_name_eng', None) or getattr(c, 'name', None) or ''
                if isinstance(name, str) and ("Monzaemon" in name or "Numemon" in name):
                    return True
            return False

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
