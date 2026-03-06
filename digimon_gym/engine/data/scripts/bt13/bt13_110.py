from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_110(CardScript):
    """BT13-110 Royal Knights of the Purge"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] <Draw 1>. You may place 1 Digimon card from your hand under 1 of your [King Drasil_7D6] in the breeding area as its bottom digivolution card. Then, place this card in the battle area.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT13-110 Draw 1 and place Digimon under King Drasil")
        effect0.set_effect_description("[Main] <Draw 1>. You may place 1 Digimon card from your hand under 1 of your [King Drasil_7D6] in the breeding area as its bottom digivolution card. Then, place this card in the battle area.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1, then may place a Digimon from hand under King Drasil_7D6 in breeding"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # Draw 1
            player.draw_cards(1)

            # You may place 1 Digimon card from hand under King Drasil_7D6 in breeding area
            breeding = player.breeding_area
            if breeding is not None and breeding.contains_card_name('King Drasil_7D6'):
                def hand_filter(c):
                    return getattr(c, 'is_digimon', False)

                def on_select(selected_card):
                    if selected_card in player.hand_cards:
                        player.hand_cards.remove(selected_card)
                    # Insert at index 0 (bottom of the digivolution stack)
                    breeding_perm = player.breeding_area
                    if breeding_perm is not None:
                        breeding_perm.card_sources.insert(0, selected_card)

                game.effect_select_hand_card(
                    player, hand_filter, on_select, is_optional=True,
                    prompt="You may place 1 Digimon card from your hand under King Drasil_7D6 in the breeding area.")
            # "Then, place this card in the battle area" — handled by engine option lifecycle

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: delay
        # Delay
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-110 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not permanent:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDeclaration
        # [Main] <Delay> - Play 1 card with the [Royal Knight] trait from the digivolution cards of your Digimon in the breeding area without paying its cost. Any [On Play] effects on Digimon played with this effect don't activate, and they gain <Rush> for the turn.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2.set_effect_name("BT13-110 Play 1 Royal Knight from breeding area digi sources")
        effect2.set_effect_description("[Main] <Delay> - Play 1 [Royal Knight] trait card from the digivolution cards of your Digimon in the breeding area without paying the cost. [On Play] effects on Digimon played by this effect don't activate, and they gain <Rush> for the turn.")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Royal Knight from breeding area digi sources, grant Rush"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            breeding = player.breeding_area
            if breeding is None or len(breeding.card_sources) < 2:
                # Need at least 2 cards (top card + at least 1 digi source)
                return

            # Find Royal Knight cards in digi sources (all except top card)
            digi_sources = breeding.card_sources[:-1]  # index 0=bottom, -1=top
            royal_knight_sources = []
            for cs in digi_sources:
                traits = getattr(cs, 'card_traits', []) or []
                if any('Royal Knight' in t for t in traits):
                    royal_knight_sources.append(cs)

            if not royal_knight_sources:
                return

            # Select one to play — use effect_select_hand_card-style manual selection
            # Since this is from breeding digi sources (not a standard zone), handle manually
            # For simplicity, play the first valid Royal Knight card found
            # TODO: Implement proper selection UI for breeding digi source cards
            selected = royal_knight_sources[0]
            breeding.card_sources.remove(selected)

            # Play the card to the battle area without paying cost
            played_perm = player.play_card_from_source(selected, pay_cost=False)

            # TODO: suppress On Play effects for this play
            # Grant Rush to the PLAYED Digimon (not the option card)
            if played_perm:
                played_perm.grant_keyword('_is_rush')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Security Effect: [Security] Place this card in the battle area.
        effect_security = ICardEffect()
        effect_security.set_timing(EffectTiming.SecuritySkill)
        effect_security.set_effect_name("BT13-110 Security: Place in battle area")
        effect_security.set_effect_description("[Security] Place this card in the battle area.")
        effect_security.is_security_effect = True

        def condition_security(context: Dict[str, Any]) -> bool:
            return True
        effect_security.set_can_use_condition(condition_security)

        def process_security(ctx: Dict[str, Any]):
            """Security: Place this card in the battle area — engine handles security play automatically"""
            pass

        effect_security.set_on_process_callback(process_security)
        effects.append(effect_security)

        return effects
