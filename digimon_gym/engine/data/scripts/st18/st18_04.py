from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST18_04(CardScript):
    """ST18-04 Pteromon | Lv.3, Green, Bird Dragon/LIBERATOR, DP 1000, Cost 3

    [On Play] Reveal the top 3 cards of your deck. Add 1 card with [Bird] or
        [Avian] in any of its traits and 1 card with the [Vortex Warriors] or
        [LIBERATOR] trait among them to the hand. Return the rest to the bottom
        of the deck.
    [Inherited][Your Turn] This Digimon gets +2000 DP.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [On Play] Reveal top 3, add 1 Bird/Avian + 1 Vortex Warriors/LIBERATOR ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("ST18-04 Reveal top 3, add Bird/Avian + Vortex Warriors/LIBERATOR")
        effect0.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Add 1 card with "
            "[Bird] or [Avian] in any of its traits and 1 card with the "
            "[Vortex Warriors] or [LIBERATOR] trait among them to the hand. "
            "Return the rest to the bottom of the deck."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Reveal top 3; add 1 Bird/Avian card and 1 Vortex Warriors/LIBERATOR card."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def reveal_filter_bird_avian(c):
                traits = getattr(c, 'card_traits', []) or []
                return any('Bird' in t or 'Avian' in t for t in traits)

            def reveal_filter_vortex_liberator(c):
                traits = getattr(c, 'card_traits', []) or []
                return any('Vortex Warriors' in t or 'LIBERATOR' in t for t in traits)

            game.effect_reveal_and_select_multi(
                player, 3,
                [(reveal_filter_bird_avian, 'hand'), (reveal_filter_vortex_liberator, 'hand')],
                remaining_placement='deck_bottom', is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Inherited][Your Turn] +2000 DP ---
        effect1 = ICardEffect()
        effect1.set_effect_name("ST18-04 Inherited: +2000 DP your turn")
        effect1.set_effect_description(
            "[Inherited][Your Turn] This Digimon gets +2000 DP."
        )
        effect1.is_inherited_effect = True
        effect1.dp_modifier = 2000

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
