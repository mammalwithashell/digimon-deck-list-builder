from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT6_004(CardScript):
    """BT6-004 Pinamon | Lv.2 (Digi-Egg)"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Inherited Effect: [When Attacking] [Once Per Turn] When this Digimon
        # attacks one of your opponent's Digimon, <Draw 1>
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDeclaration)
        effect0.is_on_attack = True
        effect0.is_inherited_effect = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("BT6_004_WhenAttacking")
        effect0.set_effect_name("BT6-004 Inherited: Draw 1 when attacking opponent's Digimon")
        effect0.set_effect_description(
            "[When Attacking] [Once Per Turn] When this Digimon attacks one of "
            "your opponent's Digimon, Draw 1."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Check that attack target is an opponent's Digimon (not player)
            target = context.get('attack_target')
            if target is None:
                # If the attack target info isn't in context, check if there's a
                # target permanent (attacks against Digimon have one)
                target_perm = context.get('target_permanent')
                if target_perm is None:
                    return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and player.library_cards:
                player.draw()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
