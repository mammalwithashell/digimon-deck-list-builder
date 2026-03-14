from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_071(CardScript):
    """EX11-071 Cool Boy"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Reveal the top 3 cards of your deck. Add 1 [Omekamon] or [Omnimon (X Antibody)] and 1 [Royal Knight] or [LIBERATOR] trait card among them to the hand. Return the rest to the bottom of the deck.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX11-071 Reveal 3, add 1 [Omekamon] or [Omnimon (X Antibody)] and 1 [Royal Knight] or [LIBERATOR] trait card, bot deck the rest.")
        effect0.set_effect_description("[On Play] Reveal the top 3 cards of your deck. Add 1 [Omekamon] or [Omnimon (X Antibody)] and 1 [Royal Knight] or [LIBERATOR] trait card among them to the hand. Return the rest to the bottom of the deck.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Reveal And Select Multi"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def reveal_filter_0(c):
                # Exact name match for Omekamon, and exact full-string match for Omnimon (X Antibody)
                names = getattr(c, 'card_names', []) or []
                return any('Omekamon' in _n or 'Omnimon (X Antibody)' in _n for _n in names)
            def reveal_filter_1(c):
                return any('Royal Knight' in _t or 'LIBERATOR' in _t for _t in (getattr(c, 'card_traits', []) or []))
            game.effect_reveal_and_select_multi(
                player, 3, [(reveal_filter_0, 'hand'), (reveal_filter_1, 'hand')],
                remaining_placement='deck_bottom', is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDeclaration
        # [Main] By returning this Tamer to the bottom of the deck, you may play 1 play cost 4 or higher [Royal Knight] or [LIBERATOR] trait card from your hand with the play cost reduced by 2.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDeclaration)
        effect1._is_field_main = True
        effect1.set_effect_name("EX11-071 Return tamer to play 4 cost or higher [Royal Knight] or [LIBERATOR] trait card for 2 less")
        effect1.set_effect_description("[Main] By returning this Tamer to the bottom of the deck, you may play 1 play cost 4 or higher [Royal Knight] or [LIBERATOR] trait card from your hand with the play cost reduced by 2.")
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Cost: return this Tamer to deck bottom.
            Action: play 1 Royal Knight/LIBERATOR cost 4+ from hand with cost -2."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Cost: return this tamer to the bottom of the deck.
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return
            player.return_permanent_to_deck_bottom(tamer_perm)

            def play_filter(c):
                cost = getattr(c, 'get_cost_itself', None)
                if cost is None or cost < 4:
                    return False
                if not any('Royal Knight' in _t or 'LIBERATOR' in _t for _t in (getattr(c, 'card_traits', []) or [])):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=False, manual_reduction=2, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("EX11-071 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
