from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_063(CardScript):
    """BT24-063 Locomon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: Lv.4 with [TS] trait for cost 3
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-063 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4
        effect0._alt_digi_trait = "TS"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: Collision (self)
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-063 Collision")
        effect1.set_effect_description("Collision")
        effect1._is_collision = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Shared process for On Play / When Digivolving:
        # Reveal top 3 from deck; play 1 cost≤5 [Machine]/[Cyborg]/[TS] card for free (not suspended);
        # return rest to top or bottom.
        # C#: SimplifiedRevealDeckTopCardsAndSelect(revealCount:3, remainingCardsPlace: DeckTopOrBottom, isTapped: false)
        def _card_ok(c) -> bool:
            cost = getattr(c, 'get_cost_itself', None)
            if cost is None:
                cost = getattr(c, 'play_cost', 0)
            if cost > 5:
                return False
            if not getattr(c, 'has_play_cost', True):
                return False
            traits = getattr(c, 'card_traits', []) or []
            return any('Machine' in t or 'Cyborg' in t or 'TS' in t for t in traits)

        def _shared_process(ctx: Dict[str, Any]):
            """Reveal top 3 from deck; play 1 cost≤5 [Machine]/[Cyborg]/[TS] card; return rest to top or bottom."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_revealed(selected, remaining):
                # Play selected card for free (not suspended)
                if selected is not None:
                    played = player.play_card_from_source(selected, pay_cost=False)
                    if played:
                        game.execute_effects(
                            EffectTiming.OnEnterFieldAnyone,
                            {"played_card": selected, "played_permanent": played,
                             "event_player": player},
                        )
                # Return remaining cards to top or bottom (player choice)
                # Use deck_bottom as default when no explicit choice mechanism available
                for c in remaining:
                    player.library_cards.insert(0, c)

            game.effect_reveal_and_select(
                player, 3, _card_ok, on_revealed, is_optional=True,
                prompt="Select 1 play cost 5 or lower [Machine], [Cyborg], or [TS] trait card.")

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Reveal the top 3 cards of your deck. You may play 1 play cost 5 or lower [Machine], [Cyborg] or [TS] trait card among them without paying the cost. Return the rest to the top or bottom of the deck.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT24-063 Reveal 3, Play cost 5- correct trait, return rest to top or bot.")
        effect2.set_effect_description("[On Play] Reveal the top 3 cards of your deck. You may play 1 play cost 5 or lower [Machine], [Cyborg] or [TS] trait card among them without paying the cost. Return the rest to the top or bottom of the deck.")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_shared_process)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Reveal the top 3 cards of your deck. You may play 1 play cost 5 or lower [Machine], [Cyborg] or [TS] trait card among them without paying the cost. Return the rest to the top or bottom of the deck.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT24-063 Reveal 3, Play cost 5- correct trait, return rest to top or bot.")
        effect3.set_effect_description("[When Digivolving] Reveal the top 3 cards of your deck. You may play 1 play cost 5 or lower [Machine], [Cyborg] or [TS] trait card among them without paying the cost. Return the rest to the top or bottom of the deck.")
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_shared_process)
        effects.append(effect3)

        # Factory effect: Collision ESS (inherited)
        effect4 = ICardEffect()
        effect4.set_effect_name("BT24-063 Collision")
        effect4.set_effect_description("Collision")
        effect4.is_inherited_effect = True
        effect4._is_collision = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
