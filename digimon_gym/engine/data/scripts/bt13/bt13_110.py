from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_110(CardScript):
    """BT13-110 Royal Knights of the Purge

    [Main] <Draw 1>. You may place 1 Digimon card from your hand as the
    bottom digivolution card of 1 of your [King Drasil_7D6] in the breeding
    area. Then, place this card in the battle area.

    [Main] <Delay> - Play 1 [Royal Knight] trait card from the digivolution
    cards of your Digimon in the breeding area without paying the cost.
    [On Play] effects on Digimon played by this effect don't activate,
    and they gain <Rush> for the turn.

    [Security] Place this card in the battle area.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Main] Draw 1, place Digimon under King Drasil, place self ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT13-110 Draw 1 and place Digimon under King Drasil")
        effect0.set_effect_description(
            "[Main] <Draw 1>. You may place 1 Digimon card from your hand as the "
            "bottom digivolution card of 1 of your [King Drasil_7D6] in the breeding "
            "area. Then, place this card in the battle area."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Draw 1
            player.draw_cards(1)

            # Check if breeding area has King Drasil_7D6
            breeding = player.breeding_area
            if breeding is not None and (
                breeding.contains_card_name('King Drasil_7D6')
                or breeding.contains_card_name('KingDrasil_7D6')
            ):
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
                    prompt="You may place 1 Digimon card from your hand under "
                           "King Drasil_7D6 in the breeding area.")
            # "Then, place this card in the battle area" — handled by engine
            # option lifecycle (PlaceDelayOptionCards in C#)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Delay marker ---
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

        # --- Effect 2: [Main] <Delay> Play Royal Knight from breeding digi sources ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2._is_field_main = True
        effect2.set_effect_name("BT13-110 Play 1 Royal Knight from breeding area digi sources")
        effect2.set_effect_description(
            "[Main] <Delay> - Play 1 [Royal Knight] trait card from the digivolution "
            "cards of your Digimon in the breeding area without paying the cost. "
            "[On Play] effects on Digimon played by this effect don't activate, "
            "and they gain <Rush> for the turn."
        )

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # C# ref: First delete the delay option card from the battle area
            delay_perm = card.permanent_of_this_card() if card else None
            if delay_perm and delay_perm in player.battle_area:
                player.delete_permanent(delay_perm)

            breeding = player.breeding_area
            if breeding is None or len(breeding.card_sources) < 2:
                return

            # Find Royal Knight cards in digi sources (all except top card)
            digi_sources = [
                cs for cs in breeding.card_sources
                if cs is not breeding.top_card
            ]
            royal_knight_sources = []
            for cs in digi_sources:
                traits = getattr(cs, 'card_traits', []) or []
                if any('Royal Knight' in t for t in traits):
                    royal_knight_sources.append(cs)

            if not royal_knight_sources:
                return

            if len(royal_knight_sources) == 1:
                # Only one choice — play it directly
                _play_royal_knight(royal_knight_sources[0], breeding, player, game)
            else:
                # Multiple choices — let player choose via effect_choose_branch
                rk_list = list(royal_knight_sources)

                def on_branch(branch_idx):
                    if 0 <= branch_idx < len(rk_list):
                        _play_royal_knight(rk_list[branch_idx], breeding, player, game)

                names = []
                for cs in rk_list:
                    card_names = getattr(cs, 'card_names', [])
                    names.append(card_names[0] if card_names else 'Unknown')
                game.effect_choose_branch(
                    player, len(rk_list), on_branch,
                    prompt="Select 1 [Royal Knight] digivolution card to play.",
                    branch_labels=names)

        def _play_royal_knight(selected, breeding_perm, player, game):
            """Remove from digi sources, play to field, suppress On Play, grant Rush."""
            if selected in breeding_perm.card_sources:
                breeding_perm.card_sources.remove(selected)

            # Play the card to the battle area without paying cost
            played_perm = player.play_card_from_source(selected, pay_cost=False)

            if played_perm:
                # Fire OnEnterFieldAnyone with _suppress_on_play to skip On Play effects
                game.execute_effects(
                    EffectTiming.OnEnterFieldAnyone,
                    {
                        "played_card": selected,
                        "played_permanent": played_perm,
                        "event_player": player,
                        "_suppress_on_play": True,
                    },
                )
                # Grant Rush for the turn
                played_perm.grant_keyword('_is_rush', duration=game.turn_count)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Security Effect: [Security] Place this card in the battle area ---
        effect_security = ICardEffect()
        effect_security.set_timing(EffectTiming.SecuritySkill)
        effect_security.set_effect_name("BT13-110 Security: Place in battle area")
        effect_security.set_effect_description("[Security] Place this card in the battle area.")
        effect_security.is_security_effect = True

        def condition_security(context: Dict[str, Any]) -> bool:
            return True
        effect_security.set_can_use_condition(condition_security)

        def process_security(ctx: Dict[str, Any]):
            """Security: Place this card in the battle area
            — engine handles security play automatically via PlaceSelfDelayOptionSecurityEffect"""
            pass

        effect_security.set_on_process_callback(process_security)
        effects.append(effect_security)

        return effects
