from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_033(CardScript):
    """BT14-033 Patamon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] Search your security stack. This Digimon may digivolve into a yellow Digimon card with the [Vaccine] trait among them without paying the cost. Then, shuffle your security stack. If digivolved by this effect, you may place 1 yellow card with the [Vaccine] trait from your hand at the bottom of your security stack.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-033 Security digivolve")
        effect0.set_effect_description("[Start of Your Main Phase] Search your security stack. This Digimon may digivolve into a yellow Digimon card with the [Vaccine] trait among them without paying the cost. Then, shuffle your security stack. If digivolved by this effect, you may place 1 yellow card with the [Vaccine] trait from your hand at the bottom of your security stack.")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            perm = ctx.get('permanent')
            if not (player and game and perm):
                return

            def security_digivolve_filter(c):
                colors = [col.name for col in getattr(c, 'card_colors', [])]
                attrs = [a.name for a in getattr(c, 'card_attributes', [])]
                return ('Yellow' in colors) and ('Vaccine' in attrs)

            digivolved = game.effect_digivolve_from_zone(
                player,
                perm,
                'security',
                security_digivolve_filter,
                cost_reduction=99,
                is_optional=True,
            )

            game.shuffle_security(player)

            if digivolved:
                def hand_security_filter(c):
                    colors = [col.name for col in getattr(c, 'card_colors', [])]
                    attrs = [a.name for a in getattr(c, 'card_attributes', [])]
                    return ('Yellow' in colors) and ('Vaccine' in attrs)

                game.effect_place_to_security_from_zone(
                    player,
                    'hand',
                    hand_security_filter,
                    count=1,
                    to_bottom=True,
                    is_optional=True,
                )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAddSecurity
        # [Your Turn][Once Per Turn] When a card is added to your security stack, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-033 Memory +1")
        effect1.set_effect_description("[Your Turn][Once Per Turn] When a card is added to your security stack, gain 1 memory.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Memory+1_BT14_033")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
