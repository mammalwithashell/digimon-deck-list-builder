from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_076(CardScript):
    """BT23-076 Sistermon Blanc | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Add your top security card to the hand. Then, <Recovery +1 (Deck)>
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT23-076 Add your top security, <Recovery +1 (Deck)>")
        effect0.set_effect_description("[On Play] Add your top security card to the hand. Then, <Recovery +1 (Deck)>")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Add top security card to hand, then Recovery +1 (Deck)."""
            player = ctx.get('player')
            if not player:
                return
            # Step 1: Add your top security card to the hand (index 0 = top)
            if player.security_cards:
                top_security = player.security_cards.pop(0)
                player.hand_cards.append(top_security)
            # Step 2: Recovery +1 (move top card of deck to security)
            player.recovery(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnTappedAnyone
        # [Your Turn] When this Digimon suspends, 1 of your other Digimon may digivolve into a Digimon card with [Huckmon] in its name or the [Royal Knight] or [CS] trait in the hand or trash with the digivolution cost reduced by 1.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnTappedAnyone)
        effect1.set_effect_name("BT23-076 1 of your other Digimon digivolves without paying the cost")
        effect1.set_effect_description("[Your Turn] When this Digimon suspends, 1 of your other Digimon may digivolve into a Digimon card with [Huckmon] in its name or the [Royal Knight] or [CS] trait in the hand or trash with the digivolution cost reduced by 1.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Digivolve from hand or trash into Huckmon/RK/CS with cost reduced by 1."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            # Select a DIFFERENT Digimon to digivolve (1 of your OTHER Digimon)
            def digi_target_filter(p):
                return p is not perm and p.is_digimon
            def on_target(target_perm):
                def digi_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    names = getattr(c, 'card_names', []) or []
                    traits = getattr(c, 'card_traits', []) or []
                    has_huckmon = any('Huckmon' in _n for _n in names)
                    has_rk = any('Royal Knight' in _t for _t in traits)
                    has_cs = any('CS' in _t for _t in traits)
                    return has_huckmon or has_rk or has_cs
                # Digivolve from hand or trash with cost reduced by 1
                game.effect_digivolve_from_hand(
                    player, target_perm, digi_filter, cost_reduction=1,
                    is_optional=True, include_trash=True)
            game.effect_select_own_permanent(
                player, on_target, filter_fn=digi_target_filter, is_optional=True,
                prompt="Select 1 of your other Digimon to digivolve.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
