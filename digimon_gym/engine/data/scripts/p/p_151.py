from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_151(CardScript):
    """P-151 Digimon Liberator"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Ignore Color Req
        effect0 = ICardEffect()
        effect0.set_effect_name("P-151 Ignore color requirements")
        effect0.set_effect_description("Ignore Color Req")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Ignore Color Req"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Ignores color requirement for playing Options — not modeled in engine
            pass  # descriptive-tagged

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [Main] Reveal the top 3 cards of your deck. Add 1 card with the [LIBERATOR] trait among them to the hand. Return the rest to the bottom of the deck.Then, you may play 1 card with the [LIBERATOR] trait and a play cost of 3 or less from your hand without paying the cost.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("P-151 Reveal top 3, Add 1 with [LIBERATOR] trait. Then Play 1 with [LIBERATOR] trait")
        effect1.set_effect_description("[Main] Reveal the top 3 cards of your deck. Add 1 card with the [LIBERATOR] trait among them to the hand. Return the rest to the bottom of the deck.Then, you may play 1 card with the [LIBERATOR] trait and a play cost of 3 or less from your hand without paying the cost.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Reveal top 3, add 1 [LIBERATOR] to hand, rest to deck bottom, then play 1 [LIBERATOR] cost<=3 free."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # Step 1: Reveal top 3 cards, select 1 with [LIBERATOR] trait to add to hand.
            # Remaining cards go to deck bottom.
            def reveal_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                return 'LIBERATOR' in traits

            def on_revealed(selected, remaining):
                # Add selected card to hand
                player.hand_cards.append(selected)
                # Return the rest to deck bottom
                for c in remaining:
                    player.library_cards.append(c)
                # Step 2: Play 1 [LIBERATOR] trait card with cost <= 3 from hand free
                def play_filter(c):
                    traits = getattr(c, 'card_traits', []) or []
                    if 'LIBERATOR' not in traits:
                        return False
                    cost = getattr(c, 'get_cost_itself', None)
                    if cost is None:
                        cost = getattr(c, 'play_cost', None)
                    if cost is None or cost > 3:
                        return False
                    return True
                game.effect_play_from_zone(
                    player, 'hand', play_filter, free=True, is_optional=True)

            game.effect_reveal_and_select(
                player, 3, reveal_filter, on_revealed, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("P-151 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
