from __future__ import annotations
import random
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_033(CardScript):
    """BT14-033 Patamon | Lv.3 Yellow Mammal (Data)

    [Start of Your Main Phase] Search your security stack. This Digimon may
    digivolve into a yellow Digimon card with the [Vaccine] trait among them
    without paying the cost. Then, shuffle your security stack. If digivolved
    by this effect, you may place 1 yellow card with the [Vaccine] trait from
    your hand at the bottom of your security stack.

    Inherited:
    [Your Turn] [Once Per Turn] When a card is added to your security stack,
    gain 1 memory.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects: List[ICardEffect] = []

        # ──────────────────────────────────────────────────────────────
        # [Start of Your Main Phase] Digivolve from security
        # ──────────────────────────────────────────────────────────────
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name(
            "BT14-033 Digivolve into yellow Vaccine Digimon from security"
        )
        effect0.set_effect_description(
            "[Start of Your Main Phase] Search your security stack. This "
            "Digimon may digivolve into a yellow Digimon card with the "
            "[Vaccine] trait among them without paying the cost. Then, "
            "shuffle your security stack. If digivolved by this effect, you "
            "may place 1 yellow card with the [Vaccine] trait from your "
            "hand at the bottom of your security stack."
        )
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Must be on field
            if card is None or card.permanent_of_this_card() is None:
                return False
            # [Your Turn]
            owner = card.owner
            if owner is None or not owner.is_my_turn:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def _is_yellow_vaccine_digimon(c) -> bool:
            """Yellow + Vaccine attribute + is Digimon.

            NOTE: "Vaccine" is an ATTRIBUTE in engine data, stored in
            c_entity_base.attribute_eng — NOT in card_traits/type_eng.
            The card text says "[Vaccine] trait" but semantically refers
            to the attribute.
            """
            if c is None or not getattr(c, 'is_digimon', False):
                return False
            colors = [col.name for col in (getattr(c, 'card_colors', None) or [])]
            if 'Yellow' not in colors:
                return False
            attrs = c.c_entity_base.attribute_eng if c.c_entity_base else []
            return 'Vaccine' in attrs

        def _is_yellow_vaccine_any(c) -> bool:
            """Yellow + Vaccine attribute, any card kind (for rider hand filter)."""
            if c is None:
                return False
            colors = [col.name for col in (getattr(c, 'card_colors', None) or [])]
            if 'Yellow' not in colors:
                return False
            attrs = c.c_entity_base.attribute_eng if c.c_entity_base else []
            return 'Vaccine' in attrs

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            if not player.security_cards:
                return

            # Closure state: tracks whether digivolve succeeded so the rider
            # can run conditionally ("If digivolved by this effect").
            state = {'digivolved': False}

            def on_security_selected(sec_card):
                # Verify the digivolve target is still valid and in security
                if sec_card not in player.security_cards:
                    return
                # Verify the host permanent still exists
                if perm not in player.battle_area:
                    return

                # Remove from security and place on top of this permanent's
                # stack as a digivolution.
                player.security_cards.remove(sec_card)
                perm.add_card_source(sec_card)
                perm.turn_digivolved = game.turn_count

                # Digivolution draws 1 (standard rule for all digivolutions,
                # including effect-based ones that ignore cost).
                player.draw()

                state['digivolved'] = True

                # "Then, shuffle your security stack." — runs after the
                # digivolve resolves.
                random.shuffle(player.security_cards)

                # Fire WhenDigivolving for the new top card's effects.
                game.execute_effects(
                    EffectTiming.WhenDigivolving,
                    {
                        "digivolved_permanent": perm,
                        "is_effect_play": True,
                    },
                )

                # "If digivolved by this effect, you may place 1 yellow card
                #  with the [Vaccine] trait from your hand at the bottom of
                #  your security stack."
                if state['digivolved']:
                    # Check host still on field for rider
                    if card.permanent_of_this_card() is None:
                        return

                    def on_hand_selected(hand_card):
                        if hand_card not in player.hand_cards:
                            return
                        # Engine convention: security_cards[0] = TOP (first
                        # to be hit on security check via pop(0)).
                        # "Bottom of security" => append() (index -1).
                        player.hand_cards.remove(hand_card)
                        player.security_cards.append(hand_card)
                        # Fire OnAddSecurity timing so inherited effects
                        # (e.g., BT14-033's own +1 memory trigger) can see
                        # this add.
                        game.execute_effects(
                            EffectTiming.OnAddSecurity,
                            {"added_cards": [hand_card], "player": player},
                        )

                    game.effect_select_hand_card(
                        player,
                        _is_yellow_vaccine_any,
                        on_hand_selected,
                        is_optional=True,
                        prompt=(
                            "Select a yellow [Vaccine] card from your hand "
                            "to place at the bottom of your security stack."
                        ),
                    )

            game.effect_select_own_security(
                player,
                _is_yellow_vaccine_digimon,
                on_security_selected,
                is_optional=True,
                prompt=(
                    "Select a yellow [Vaccine] Digimon in your security "
                    "stack to digivolve this card into."
                ),
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ──────────────────────────────────────────────────────────────
        # Inherited: [Your Turn][OPT] OnAddSecurity — gain 1 memory
        # ──────────────────────────────────────────────────────────────
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnAddSecurity)
        effect1.set_effect_name("BT14-033 Memory +1")
        effect1.set_effect_description(
            "[Your Turn] [Once Per Turn] When a card is added to your "
            "security stack, gain 1 memory."
        )
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Memory+1_BT14_033")

        def condition1(context: Dict[str, Any]) -> bool:
            # Host must be on field (inherited effects only fire while host
            # is in a stack on the battle/breeding area).
            if card is None or card.permanent_of_this_card() is None:
                return False
            owner = card.owner
            if owner is None or not owner.is_my_turn:
                return False
            # Owner gate: the add-to-security event must be OUR event.
            event_player = context.get('event_player')
            if event_player is not None and event_player is not owner:
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
