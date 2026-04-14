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
                return 'Iliad' in (getattr(c, 'card_traits', []) or [])

            def ts_filter(c):
                return 'TS' in (getattr(c, 'card_traits', []) or [])

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

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] [Once Per Turn] You may add your top security card to the hand. Then, if you have 0 security cards, <Recovery +1 (Deck)>.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("BT24-031 May add 1 sec card to hand, if at 0 <Recovery +1>.")
        effect2.set_effect_description("[When Attacking] [Once Per Turn] You may add your top security card to the hand. Then, if you have 0 security cards, <Recovery +1 (Deck)>.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT24_031_Inherited")
        effect2.is_on_attack = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """You may add top security to hand. If 0 security, Recovery +1."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Only attempt to add if security cards exist
            if player.security_cards:
                # Optional: player chooses whether to add
                def on_choice(choice: int):
                    if choice == 0:  # yes
                        if player.security_cards:
                            top_sec = player.security_cards.pop(0)
                            player.hand_cards.append(top_sec)
                    # Then if 0 security after the choice, Recovery +1 (Deck)
                    if not player.security_cards:
                        player.recovery(1)
                game.effect_choose_branch(
                    player, 2, on_choice,
                    branch_labels=["Add to hand", "Don't add to hand"]
                )
            else:
                # No security cards — check for Recovery +1
                if not player.security_cards:
                    player.recovery(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
