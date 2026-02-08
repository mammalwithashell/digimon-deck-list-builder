from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_033(CardScript):
    """Auto-transpiled from DCGO BT14_033.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] Search your security stack. This Digimon may digivolve into a yellow Digimon card with the [Vaccine] trait among them without paying the cost. Then, shuffle your security stack. If digivolved by this effect, you may place 1 yellow card with the [Vaccine] trait from your hand at the bottom of your security stack.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-033 This Digimon digivolves into Digimon card in security")
        effect0.set_effect_description("[Start of Your Main Phase] Search your security stack. This Digimon may digivolve into a yellow Digimon card with the [Vaccine] trait among them without paying the cost. Then, shuffle your security stack. If digivolved by this effect, you may place 1 yellow card with the [Vaccine] trait from your hand at the bottom of your security stack.")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Search security, digivolve into yellow Vaccine Digimon, then optionally place yellow Vaccine from hand to bottom of security."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            if not player.security_cards:
                return

            def is_yellow_vaccine_digimon(c):
                """Filter: yellow Digimon with [Vaccine] trait."""
                if not c.is_digimon:
                    return False
                from ....data.enums import CardColor
                if c.color != CardColor.Yellow:
                    return False
                traits = getattr(c, 'traits', []) or []
                return any('Vaccine' in t for t in traits)

            def on_security_selected(sec_card):
                # Remove from security, stack onto permanent
                player.remove_from_security(sec_card)
                perm.add_card_source(sec_card)
                game.logger.log(
                    f"[Effect] {player.player_name} digivolved "
                    f"{perm.top_card.card_names[0] if perm.top_card else 'Unknown'} "
                    f"from security (no cost)")
                # Draw 1 (digivolution bonus)
                player.draw()
                # Shuffle security stack
                import random
                random.shuffle(player.security_cards)
                # Then, may place 1 yellow [Vaccine] from hand at bottom of security
                hand_vaccine = [c for c in player.hand_cards if is_yellow_vaccine_digimon(c)]
                if hand_vaccine:
                    def on_hand_selected(hand_card):
                        player.add_to_security_from_hand(hand_card, to_top=False)
                        game.logger.log(
                            f"[Effect] {player.player_name} placed "
                            f"{hand_card.card_names[0]} at bottom of security")
                    game.effect_select_hand_card(
                        player, is_yellow_vaccine_digimon, on_hand_selected,
                        is_optional=True)

            game.effect_select_own_security(
                player, is_yellow_vaccine_digimon, on_security_selected,
                is_optional=True)

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
            perm = ctx.get('permanent')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
