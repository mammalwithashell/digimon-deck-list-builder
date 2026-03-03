from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_033(CardScript):
    """BT14-033 Patamon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("BT14-033 This Digimon digivolves into Digimon card in security")
        effect0.set_effect_description(
            "[Start of Your Main Phase] Search your security stack. This Digimon may digivolve into a yellow Digimon card with the [Vaccine] trait among them without paying the cost. Then, shuffle your security stack. If digivolved by this effect, you may place 1 yellow card with the [Vaccine] trait from your hand at the bottom of your security stack."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def security_filter(sec_card):
                if not getattr(sec_card, 'is_digimon', False):
                    return False
                colors = [col.name for col in (getattr(sec_card, 'card_colors', []) or [])]
                if 'Yellow' not in colors:
                    return False
                return 'Vaccine' in getattr(sec_card, 'card_traits', [])

            def on_selected(sec_card):
                if sec_card not in player.security_cards:
                    return
                player.security_cards.remove(sec_card)
                perm.add_card_source(sec_card)
                perm.turn_digivolved = game.turn_count
                player.draw()
                game.execute_effects(
                    EffectTiming.WhenDigivolving,
                    {"digivolved_permanent": perm},
                )

                def hand_filter(hand_card):
                    colors = [col.name for col in (getattr(hand_card, 'card_colors', []) or [])]
                    if 'Yellow' not in colors:
                        return False
                    return 'Vaccine' in getattr(hand_card, 'card_traits', [])

                def on_hand_selected(hand_card):
                    if hand_card in player.hand_cards:
                        player.add_to_security_from_hand(hand_card, to_top=False)

                game.effect_select_hand_card(
                    player,
                    hand_filter,
                    on_hand_selected,
                    is_optional=True,
                    prompt="Select a yellow [Vaccine] card to place on the bottom of your security.",
                )

            game.effect_select_own_security(
                player,
                security_filter,
                on_selected,
                is_optional=True,
                prompt="Select a yellow [Vaccine] Digimon in your security to digivolve into.",
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnAddSecurity)
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
            player = ctx.get('player')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
