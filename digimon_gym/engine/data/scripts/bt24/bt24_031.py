from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_031(CardScript):
    """BT24-031 Elecmon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-031 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.2 with [TS] trait for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_level = 2
        effect0._alt_digi_trait = "TS"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('TS' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Reveal the top 3 cards of your deck. Add 1 card with the [Iliad] trait and 1 card with [TS] trait among them to the hand. Return the rest to the bottom of the deck.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT24-031 Reveal 3, Add 1 [Iliad] trait, and 1 [TS] trait")
        effect1.set_effect_description("[On Play] Reveal the top 3 cards of your deck. Add 1 card with the [Iliad] trait and 1 card with [TS] trait among them to the hand. Return the rest to the bottom of the deck.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Reveal top 3, add 1 [Iliad] and 1 [TS] trait card to hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            if not player.library_cards:
                return

            def iliad_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                return any('Iliad' in t for t in traits)

            def ts_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                return any('TS' in t for t in traits)

            game.effect_reveal_and_select_multi(
                player, 3,
                passes=[
                    (iliad_filter, 'hand'),
                    (ts_filter, 'hand'),
                ],
                remaining_placement='deck_bottom',
                is_optional=True,
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] You may add your top security card to the hand. Then, if you have 0 security cards, <Recovery +1 (Deck)>.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnAllyAttack)
        effect2.set_effect_name("BT24-031 May add 1 sec card to hand, if at 0 <Recovery +1>.")
        effect2.set_effect_description("[When Attacking] [Once Per Turn] You may add your top security card to the hand. Then, if you have 0 security cards, <Recovery +1 (Deck)>.")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT24_031_Inherited")
        effect2.is_on_attack = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            # Must have security cards to add to hand
            if not player.security_cards:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """You may add top security to hand. If 0 security, Recovery +1."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Add top security card to hand
            if player.security_cards:
                top_sec = player.security_cards.pop(0)
                player.hand_cards.append(top_sec)
            # Then if 0 security, Recovery +1 (Deck)
            if not player.security_cards:
                player.recovery(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
