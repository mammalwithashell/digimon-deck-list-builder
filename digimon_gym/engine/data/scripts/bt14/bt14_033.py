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
        effect0.set_effect_name("BT14-033 Security digivolve into yellow [Vaccine]")
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

            def sec_filter(c):
                colors = [col.name for col in getattr(c, 'card_colors', [])]
                attrs = [a.lower() for a in getattr(c, 'attribute_eng', [])]
                return ('Yellow' in colors) and ('vaccine' in attrs)

            digivolved = False
            if hasattr(game, 'effect_digivolve_from_zone'):
                try:
                    digivolved = bool(game.effect_digivolve_from_zone(
                        player,
                        perm,
                        'security',
                        sec_filter,
                        free=True,
                        is_optional=True,
                        shuffle_zone_after=True,
                    ))
                except TypeError:
                    digivolved = bool(game.effect_digivolve_from_zone(
                        player,
                        perm,
                        'security',
                        sec_filter,
                        free=True,
                        is_optional=True,
                    ))
                    if hasattr(player, 'shuffle_security'):
                        player.shuffle_security()
            elif hasattr(game, 'effect_search_digivolve_from_security'):
                digivolved = bool(game.effect_search_digivolve_from_security(
                    player,
                    perm,
                    sec_filter,
                    free=True,
                    is_optional=True,
                    shuffle_after=True,
                ))
            else:
                return

            if not digivolved:
                return

            def hand_filter(c):
                colors = [col.name for col in getattr(c, 'card_colors', [])]
                attrs = [a.lower() for a in getattr(c, 'attribute_eng', [])]
                return ('Yellow' in colors) and ('vaccine' in attrs)

            if hasattr(game, 'effect_place_from_hand_to_security_bottom'):
                game.effect_place_from_hand_to_security_bottom(
                    player,
                    hand_filter,
                    is_optional=True,
                )
            elif hasattr(game, 'effect_move_card_between_zones'):
                game.effect_move_card_between_zones(
                    player,
                    from_zone='hand',
                    to_zone='security_bottom',
                    card_filter=hand_filter,
                    is_optional=True,
                    count=1,
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
